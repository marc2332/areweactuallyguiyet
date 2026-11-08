use crate::{
    host::Adapter,
    ui::{self, Bridge},
};
use benchmark_common::Scenario;
use dioxus_native::prelude::*;
use std::{rc::Rc, time::Duration};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::host::run::<ScenarioAdapter>()
}

struct ScenarioAdapter;
impl Adapter for ScenarioAdapter {
    type State = ();
    const SCENARIO: Scenario = Scenario::Startup;
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
    let task_bridge = Rc::clone(&bridge);
    use_future(move || {
        let bridge = Rc::clone(&task_bridge);
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
                    }
                    let applied = frame.update_index;
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
        Startup {}
    }
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
            div { class: "scroll", style: "flex:1;min-height:0", for index in 0..20 { ui::Row { key: "{index}", index } } }
        }
    }
}

const CSS: &str = r#"
.startup { display:flex; flex-direction:column; padding:16px; gap:8px; }
.startup input { width:430px; }
"#;
