use crate::{
    host::{Adapter, Windows},
    ui::{self, Bridge},
};
use benchmark_common::Scenario;
use blitz_dom::NodeId;
use dioxus_native::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<ScenarioAdapter>()
}

struct ScenarioAdapter;
impl Adapter for ScenarioAdapter {
    type State = ScrollState;
    const SCENARIO: Scenario = Scenario::Scroll;
    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State) {
        let state = ScrollState {
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
struct ScrollState {
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
                        target.set(Some(0.0));
                    }
                    let applied = frame.update_index;
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
    let styles = use_hook(|| ui::styles(""));
    rsx! { style { "{styles}" }
        div { id: "benchmark-scroll", class: "full scroll", for index in 0..2000 { ui::Row { key: "{index}", index } } }
    }
}
