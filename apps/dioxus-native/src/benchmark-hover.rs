use std::{rc::Rc, time::Duration};

use benchmark_common::{HoverObserver, Scenario, hover};
use dioxus_native::prelude::*;

use crate::{host::Adapter, ui::Bridge};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<HoverScenario>()
}

struct HoverScenario;

impl Adapter for HoverScenario {
    type State = ();
    const SCENARIO: Scenario = Scenario::Hover;

    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State) {
        (VirtualDom::new_with_props(App, AppProps { bridge }), ())
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
    use_context_provider(|| bridge.harness.borrow().hover_observer());
    use_future(move || {
        let bridge = Rc::clone(&bridge);
        async move {
            loop {
                async_io::Timer::after(Duration::from_millis(16)).await;
                let result = bridge.harness.borrow_mut().poll();
                match result {
                    Ok(frame) if frame.should_quit => {
                        bridge.quit.set(true);
                        break;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        bridge.fail(error);
                        break;
                    }
                }
            }
        }
    });
    let styles = use_hook(|| {
        let background = hover::BACKGROUND;
        format!(
            "html,body,main,#main {{margin:0;padding:0;width:100%;height:100%;background:rgb({},{},{});}} .hover-grid {{display:grid;grid-template-columns:repeat({},{}px);grid-template-rows:repeat({},{}px);gap:0;width:{}px;height:{}px;}}",
            background.0,
            background.1,
            background.2,
            hover::COLUMNS,
            hover::CELL_SIZE,
            hover::ROWS,
            hover::CELL_SIZE,
            hover::GRID_SIZE,
            hover::GRID_SIZE,
        )
    });
    rsx! {
        style { "{styles}" }
        div { class: "hover-grid",
            for index in 0..hover::COUNT {
                HoverCell { key: "{index}", index }
            }
        }
    }
}

#[component]
fn HoverCell(index: usize) -> Element {
    let mut hovered = use_signal(|| false);
    let observer = use_context::<HoverObserver>();
    let leave_observer = observer.clone();
    let color = if hovered() {
        hover::HOVERED_COLOR
    } else {
        hover::NORMAL_COLOR
    };
    let size = hover::CELL_SIZE;
    rsx! {
        div {
            style: "width:{size}px;height:{size}px;background:rgb({color.0},{color.1},{color.2})",
            onmouseenter: move |_| {
                hovered.set(true);
                observer.enter(index);
            },
            onmouseleave: move |_| {
                hovered.set(false);
                leave_observer.leave(index);
            },
        }
    }
}
