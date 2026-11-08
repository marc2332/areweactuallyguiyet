use crate::{
    host::Adapter,
    ui::{self, Bridge},
};
use benchmark_common::Scenario;
use benchmark_common::Switches;
use dioxus_native::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<ScenarioAdapter>()
}

struct ScenarioAdapter;
impl Adapter for ScenarioAdapter {
    type State = ();
    const SCENARIO: Scenario = Scenario::Switches;
    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State) {
        let state = ();
        (VirtualDom::new_with_props(App, AppProps { bridge }), state)
    }
}

#[derive(Props, Clone)]
struct AppProps {
    bridge: Rc<Bridge>,
}
impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.bridge, &other.bridge)
    }
}

#[allow(non_snake_case)]
fn App(props: AppProps) -> Element {
    let bridge = props.bridge;
    let cells = use_hook(|| {
        (0..benchmark_common::SWITCH_COUNT)
            .map(|_| Signal::new(false))
            .collect::<Vec<_>>()
    });
    let switches = use_hook(|| Rc::new(RefCell::new(Switches::default())));
    let task_switches = Rc::clone(&switches);
    let task_cells = cells.clone();
    let task_bridge = Rc::clone(&bridge);
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
                    }
                    if frame.running {
                        switches.borrow_mut().advance_changed(
                            frame.update_index,
                            harness.switch_mode(),
                            |index, value| cells[index].set(value),
                        );
                    }
                    let applied = switches.borrow().applied_updates();
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
        div { class: "switch-grid", for (index, value) in cells.into_iter().enumerate() { DummyCell { key: "{index}", index, value, switches: Rc::clone(&switches) } } }
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

const CSS: &str = r#"
.switch-grid { display:grid; grid-template-columns:repeat(30,30px); grid-template-rows:repeat(30,30px); gap:0; }
.dummy { display:flex; align-items:center; justify-content:center; width:30px; height:30px; padding:0; margin:0; border:0; border-radius:0; box-shadow:none; outline:none; appearance:none; color:white; font-family:Bench; font-size:12px; font-weight:400; line-height:1; text-align:center; }
"#;
