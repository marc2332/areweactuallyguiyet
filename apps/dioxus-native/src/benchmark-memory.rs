use crate::{
    host::{Adapter, Windows},
    ui::{self, Bridge},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use benchmark_common::Scenario;
use benchmark_common::dashboard::{Dashboard, THUMBNAILS};
use blitz_dom::NodeId;
use dioxus_native::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<ScenarioAdapter>()
}

struct ScenarioAdapter;
impl Adapter for ScenarioAdapter {
    type State = MemoryState;
    const SCENARIO: Scenario = Scenario::Memory;
    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State) {
        let state = MemoryState {
            target: Rc::new(Cell::new(None)),
            node: None,
        };
        let target = Rc::clone(&state.target);
        (
            VirtualDom::new_with_props(App, AppProps { bridge, target }),
            state,
        )
    }
    fn update_scroll(state: &mut Self::State, windows: &mut Windows) {
        crate::scroll::apply::<false>(windows, &mut state.node, &state.target, |_, _| false);
    }
}
struct MemoryState {
    target: Rc<Cell<Option<f64>>>,
    node: Option<NodeId>,
}

#[derive(Props, Clone)]
struct AppProps {
    bridge: Rc<Bridge>,
    target: Rc<Cell<Option<f64>>>,
}
impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.bridge, &other.bridge)
    }
}

#[allow(non_snake_case)]
fn App(props: AppProps) -> Element {
    let bridge = props.bridge;
    let mut dashboard = use_signal(Dashboard::default);
    let target = Rc::clone(&props.target);
    let task_bridge = Rc::clone(&bridge);
    use_future(move || {
        let bridge = Rc::clone(&task_bridge);
        let target = Rc::clone(&target);
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
                        dashboard.write().reset();
                        target.set(Some(0.0));
                    }
                    if frame.running {
                        dashboard.write().advance(frame);
                    }
                    let state = dashboard.peek();
                    harness.report_dashboard(&state)?;
                    let applied = state.applied_updates();
                    if frame.running {
                        target.set(Some(f64::from(frame.scroll_offset)));
                    }
                    harness.report_applied(applied)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    bridge.fail(error);
                    break;
                }
                if bridge.quit.get() {
                    break;
                }
            }
        }
    });
    let styles = use_hook(|| ui::styles(CSS));
    rsx! { style { "{styles}" }
        DashboardView { model: dashboard }
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
