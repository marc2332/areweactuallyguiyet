use crate::{
    host::{Adapter, Windows},
    ui::{self, Bridge},
};
use benchmark_common::Scenario;
use blitz_dom::NodeId;
use dioxus_native::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<ScenarioAdapter>()
}

struct ScenarioAdapter;
impl Adapter for ScenarioAdapter {
    type State = VirtualState;
    const SCENARIO: Scenario = Scenario::VirtualScroll;
    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State) {
        let state = VirtualState {
            target: Rc::new(Cell::new(None)),
            position: Rc::new(RefCell::new(None)),
            node: None,
        };
        let target = Rc::clone(&state.target);
        let position = Rc::clone(&state.position);
        (
            VirtualDom::new_with_props(
                App,
                AppProps {
                    bridge,
                    target,
                    position,
                },
            ),
            state,
        )
    }
    fn update_scroll(state: &mut Self::State, windows: &mut Windows) {
        crate::scroll::apply::<true>(windows, &mut state.node, &state.target, |doc, offset| {
            if let Some(mut position) = *state.position.borrow() {
                if *position.peek() != offset {
                    doc.vdom.in_scope(ScopeId::ROOT, || position.set(offset));
                    return true;
                }
            }
            false
        });
    }
}
struct VirtualState {
    target: Rc<Cell<Option<f64>>>,
    node: Option<NodeId>,
    position: Rc<RefCell<Option<Signal<f32>>>>,
}

#[derive(Props, Clone)]
struct AppProps {
    bridge: Rc<Bridge>,
    target: Rc<Cell<Option<f64>>>,
    position: Rc<RefCell<Option<Signal<f32>>>>,
}
impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.bridge, &other.bridge)
    }
}

#[allow(non_snake_case)]
fn App(props: AppProps) -> Element {
    let bridge = props.bridge;
    let virtual_position = use_signal(|| 0.0_f32);
    let position = Rc::clone(&props.position);
    use_hook(move || *position.borrow_mut() = Some(virtual_position));
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
        VirtualRows { position: virtual_position }
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
            for index in first..end { ui::Row { key: "{index}", index } }
            div { style: "height:{after}px;flex-shrink:0" }
        }
    }
}
