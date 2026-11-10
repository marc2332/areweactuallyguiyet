use dioxus_native::{LogicalSize, WindowAttributes, prelude::*};

pub fn run(title: &str) {
    dioxus_native::launch_cfg(
        app,
        vec![],
        vec![Box::new(
            WindowAttributes::default()
                .with_title(title)
                .with_surface_size(LogicalSize::new(1000.0, 700.0)),
        )],
    );
}

fn app() -> Element {
    rsx! {
        style { "{CSS}" }
        div {
            class: "scroll",
            for number in 1..=2000 {
                div {
                    key: "{number}",
                    class: "row",
                    background: if number % 2 == 1 { "#f5f5f5" } else { "#e6e6e6" },
                    "Item {number}"
                }
            }
        }
    }
}

const CSS: &str = r#"
html, body, main, #main { margin: 0; padding: 0; width: 100%; height: 100%; }
* { box-sizing: border-box; }
body { color: black; background: white; font: 14px sans-serif; }
.scroll { width: 100%; height: 100%; overflow-y: auto; overflow-x: hidden; }
.row { width: 960px; min-width: 960px; height: 48px; min-height: 48px; padding: 12px; font-size: 14px; }
"#;
