use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use benchmark_common::{
    Harness, Scenario, Switches,
    dashboard::{Dashboard, THUMBNAILS},
    row_body, row_title,
};
use dioxus_native::prelude::*;

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run(scenario)
}

pub struct Bridge {
    pub scenario: Scenario,
    pub harness: RefCell<Harness>,
    pub quit: Cell<bool>,
    pub failure: RefCell<Option<String>>,
    pub scroll_target: Cell<Option<f64>>,
    pub virtual_position: RefCell<Option<Signal<f32>>>,
}

impl Bridge {
    pub fn new(scenario: Scenario, harness: Harness) -> Self {
        Self {
            scenario,
            harness: RefCell::new(harness),
            quit: Cell::new(false),
            failure: RefCell::new(None),
            scroll_target: Cell::new(None),
            virtual_position: RefCell::new(None),
        }
    }
}

#[derive(Props, Clone)]
pub struct AppProps {
    pub bridge: Rc<Bridge>,
}

impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.bridge, &other.bridge)
    }
}

#[allow(non_snake_case)]
pub fn App(props: AppProps) -> Element {
    let bridge = props.bridge;
    let scenario = bridge.scenario;
    let cells = use_hook(move || {
        if scenario == Scenario::Switches {
            (0..benchmark_common::SWITCH_COUNT)
                .map(|_| Signal::new(false))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    });
    let switches = use_hook(|| Rc::new(RefCell::new(Switches::default())));
    let dashboard =
        use_hook(move || (scenario == Scenario::Memory).then(|| Signal::new(Dashboard::default())));
    let virtual_position = use_signal(|| 0.0_f32);
    let virtual_bridge = Rc::clone(&bridge);
    use_hook(move || *virtual_bridge.virtual_position.borrow_mut() = Some(virtual_position));
    let task_bridge = Rc::clone(&bridge);
    let task_cells = cells.clone();
    let task_switches = Rc::clone(&switches);
    use_future(move || {
        let bridge = Rc::clone(&task_bridge);
        let mut cells = task_cells.clone();
        let switches = Rc::clone(&task_switches);
        async move {
            let mut reset_generation = 0;
            loop {
                async_io::Timer::after(Duration::from_millis(16)).await;
                let result = (|| -> std::io::Result<()> {
                    let mut harness = bridge.harness.borrow_mut();
                    let frame = harness.poll()?;
                    if frame.should_quit {
                        bridge.quit.set(true);
                        return Ok(());
                    }
                    if frame.reset_generation != reset_generation {
                        reset_generation = frame.reset_generation;
                        *switches.borrow_mut() = Switches::default();
                        for cell in &mut cells {
                            cell.set(false);
                        }
                        if let Some(mut dashboard) = dashboard {
                            dashboard.write().reset();
                        }
                        bridge.scroll_target.set(Some(0.0));
                    }
                    let applied = if scenario == Scenario::Switches {
                        if frame.running {
                            switches.borrow_mut().advance_changed(
                                frame.update_index,
                                harness.switch_mode(),
                                |index, value| cells[index].set(value),
                            );
                        }
                        switches.borrow().applied_updates()
                    } else if let Some(mut dashboard) = dashboard {
                        if frame.running {
                            dashboard.write().advance(frame);
                        }
                        let state = dashboard.peek();
                        harness.report_dashboard(&state)?;
                        state.applied_updates()
                    } else {
                        frame.update_index
                    };
                    if frame.running
                        && matches!(
                            scenario,
                            Scenario::Scroll | Scenario::VirtualScroll | Scenario::Memory
                        )
                    {
                        bridge
                            .scroll_target
                            .set(Some(f64::from(frame.scroll_offset)));
                    }
                    harness.report_applied(applied)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    *bridge.failure.borrow_mut() = Some(error.to_string());
                    bridge.quit.set(true);
                    break;
                }
                if bridge.quit.get() {
                    break;
                }
            }
        }
    });
    let styles = use_hook(|| {
        let font = STANDARD.encode(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        format!("@font-face {{font-family:Bench;src:url(data:font/ttf;base64,{font});}}\n{CSS}")
    });
    rsx! {
        style { "{styles}" }
        match scenario {
            Scenario::Startup => rsx! { Startup {} },
            Scenario::Scroll => rsx! { div { id: "benchmark-scroll", class: "full scroll", for index in 0..2000 { Row { key: "{index}", index } } } },
            Scenario::VirtualScroll => rsx! { VirtualRows { position: virtual_position } },
            Scenario::Switches => rsx! {
                div { class: "switch-grid",
                    for (index, value) in cells.into_iter().enumerate() { DummyCell { key: "{index}", index, value, switches: Rc::clone(&switches) } }
                }
            },
            Scenario::Memory => rsx! { if let Some(model) = dashboard { DashboardView { model } } },
        }
    }
}

#[component]
fn Row(index: usize) -> Element {
    let title = row_title(index);
    let body = row_body(index);
    rsx! { div { class: "row", div { "{title}" } div { class: "row-body", "{body}" } } }
}

#[component]
fn Startup() -> Element {
    let mut query = use_signal(String::new);
    let mut applied = use_signal(|| 0usize);
    rsx! {
        div { class: "full startup",
            h2 { "Benchmark startup" }
            p { "Prepared initial content" }
            input { value: "{query}", oninput: move |event| query.set(event.value()), placeholder: "Search records" }
            button { onclick: move |_| applied += 1, "Apply ({applied})" }
            div { class: "scroll", style: "flex:1;min-height:0", for index in 0..20 { Row { key: "{index}", index } } }
        }
    }
}

#[component]
fn VirtualRows(position: Signal<f32>) -> Element {
    let first = ((position() / 48.0).floor() as usize)
        .saturating_sub(2)
        .min(99_980);
    let end = (first + 22).min(100_000);
    let before = first * 48;
    let after = (100_000 - end) * 48;
    rsx! {
        div { id: "benchmark-scroll", class: "full scroll",
            div { style: "height:{before}px;flex-shrink:0" }
            for index in first..end { Row { key: "{index}", index } }
            div { style: "height:{after}px;flex-shrink:0" }
        }
    }
}

#[component]
fn DummyCell(index: usize, mut value: Signal<bool>, switches: Rc<RefCell<Switches>>) -> Element {
    let active = value();
    let color = if active { "#2f9e44" } else { "#868e96" };
    let text = if active { "ON" } else { "OFF" };
    rsx! {
        button {
            class: "dummy",
            style: "background:{color}",
            onclick: move |_| {
                let active = {
                    let mut switches = switches.borrow_mut();
                    switches.toggle(index);
                    switches.get(index)
                };
                value.set(active);
            },
            "{text}"
        }
    }
}

#[component]
fn DashboardView(mut model: Signal<Dashboard>) -> Element {
    let state = model.read().clone();
    let visible = state.visible_records();
    let selected = &state.fixtures.records[state.selected_record];
    let images = use_hook(|| {
        THUMBNAILS
            .iter()
            .map(|bytes| format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
            .collect::<Vec<_>>()
    });
    rsx! {
        div { class: "full dashboard",
            div { class: "header",
                span { "Dashboard" }
                input { value: "{state.query}", placeholder: "Search records", oninput: move |event| model.write().search(event.value()) }
            }
            div { class: "dashboard-body",
                div { class: "sidebar scroll",
                    for section in 0..20 {
                        button { class: "navigation", onclick: move |_| model.write().navigate(section),
                            if section == 0 { "All records" } else { "Section {section:02}" }
                        }
                    }
                    div { class: "thumbnails",
                        for (index, source) in images.into_iter().enumerate() {
                            img { key: "{index}", src: "{source}", width: "48", height: "48", alt: "Thumbnail {index}" }
                        }
                    }
                }
                div { id: "benchmark-scroll", class: "records scroll",
                    for index in visible {
                        div { key: "{index}", class: "record", style: if state.selected_record == index { "background:#e3edff" } else { "background:white" },
                            button { class: "record-text", onclick: move |_| model.write().select(index),
                                div { "{state.fixtures.records[index].title}" }
                                div { class: "record-body", "{state.fixtures.records[index].body}" }
                            }
                            button { class: "status", onclick: move |_| model.write().toggle(index), if state.active.get(index) { "ON" } else { "OFF" } }
                        }
                    }
                }
                div { class: "detail-column",
                    div { class: "detail scroll", h3 { "{selected.title}" } div { "{state.detail}" } }
                    div { class: "fields scroll",
                        for index in 0..20 {
                            label { class: "field",
                                span { "Field {index:02}" }
                                input { value: "{state.fields[index]}", oninput: move |event| model.write().edit(index, event.value()) }
                            }
                        }
                    }
                }
            }
        }
    }
}

const CSS: &str = r#"
html,body,main,#main { margin:0; padding:0; width:100%; height:100%; }
* { box-sizing:border-box; font-family:Bench,"DejaVu Sans",sans-serif; }
body { background:white; color:black; font-size:14px; }
.full { width:100%; height:100%; min-height:0; }
.scroll { overflow-y:auto; overflow-x:hidden; }
.row { height:48px; min-height:48px; padding:4px 8px; overflow:hidden; }
.row-body { font-size:12px; }
.startup { display:flex; flex-direction:column; padding:16px; gap:8px; }
.startup input { width:430px; }
input { border:1px solid #aaa; border-radius:0; padding:4px 6px; color:black; background:white; height:32px; font-size:14px; }
button { color:black; font-size:14px; }
.switch-grid { display:grid; grid-template-columns:repeat(30,30px); grid-template-rows:repeat(30,30px); gap:0; }
.dummy { display:flex; align-items:center; justify-content:center; width:30px; height:30px; padding:0; margin:0; border:0; border-radius:0; box-shadow:none; outline:none; appearance:none; color:white; font-family:Bench; font-size:12px; font-weight:400; line-height:1; text-align:center; }
.dashboard { display:flex; flex-direction:column; }
.header { display:flex; align-items:center; height:56px; min-height:56px; padding:8px; gap:12px; }
.header input { flex:1; }
.dashboard-body { display:flex; flex:1; min-height:0; }
.sidebar { width:180px; min-width:180px; padding:4px 8px; background:#f1f3f5; }
.navigation { display:block; width:100%; height:22px; padding:0; text-align:left; background:transparent; border:0; font-size:12px; }
.thumbnails { display:grid; grid-template-columns:48px 48px; gap:4px 12px; padding-top:6px; }
.records { width:600px; min-width:600px; }
.record { display:flex; align-items:center; height:64px; min-height:64px; padding:4px 8px; }
.record-text { flex:1; min-width:0; height:56px; border:0; background:transparent; text-align:left; padding:0; overflow:hidden; }
.record-body { font-size:12px; height:30px; overflow:hidden; }
.status { background:#868e96; color:white; border:0; width:48px; height:24px; margin-left:8px; }
.detail-column { flex:1; min-width:0; display:flex; flex-direction:column; padding:8px; }
.detail { height:300px; min-height:300px; line-height:1.4; }
.detail h3 { margin:0 0 8px; }
.fields { flex:1; min-height:0; padding-top:8px; }
.field { display:flex; align-items:center; gap:8px; height:36px; }
.field span { width:80px; min-width:80px; }
.field input { flex:1; min-width:0; }
"#;
