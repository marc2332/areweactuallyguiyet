#[path = "../../scroll_config.rs"]
mod scroll_config;

use zgui::prelude::*;

const STYLE: &str = ":root { font-family: sans-serif; font-size: 14px; color: #000; background: #fff; } scroll, column { box-sizing: border-box; } .viewport { width: 100%; height: 100%; min-width: 0; min-height: 0; } .rows { width: 100%; flex-shrink: 0; } .item { width: 960px; min-width: 960px; height: 48px; min-height: 48px; flex-shrink: 0; padding: 12px; background: #f5f5f5; } .item.alternate { background: #e6e6e6; }";

fn item(index: usize) -> impl IntoView {
    let class = if index % 2 == 0 {
        "item"
    } else {
        "item alternate"
    };
    let label = format!("Item {}", index + 1);
    view! { column(class = class) { text {{label}} } }
}

pub fn main() -> Result<(), zgui::Error> {
    app().with_title("zgui Scroll").with_size(1000.0, 700.0).with_stylesheet(STYLE).run(|| view! {
        scroll(class = "viewport") {
            column(class = "rows") { for index in || 0..scroll_config::row_count(), key = |index: &usize| *index { {item(index)} } }
        }
    })
}
