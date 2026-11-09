#[cfg(not(feature = "hover"))]
use base64::{Engine, engine::general_purpose::STANDARD};
use benchmark_common::{Harness, row_body, row_title};
use dioxus_native::prelude::*;
use std::cell::{Cell, RefCell};

pub struct Bridge {
    pub harness: RefCell<Harness>,
    pub quit: Cell<bool>,
    pub failure: RefCell<Option<String>>,
}
impl Bridge {
    pub fn new(harness: Harness) -> Self {
        Self {
            harness: RefCell::new(harness),
            quit: Cell::new(false),
            failure: RefCell::new(None),
        }
    }
    pub fn fail(&self, error: std::io::Error) {
        *self.failure.borrow_mut() = Some(error.to_string());
        self.quit.set(true);
    }
}

#[cfg(not(feature = "hover"))]
pub fn styles(scenario_css: &str) -> String {
    let font = STANDARD.encode(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
    format!(
        "@font-face {{font-family:Bench;src:url(data:font/ttf;base64,{font});}}\n{CSS}\n{scenario_css}"
    )
}

#[component]
pub fn Row(index: usize) -> Element {
    let title = row_title(index);
    let body = row_body(index);
    rsx! { div { class: "row", div { "{title}" } div { class: "row-body", "{body}" } } }
}

#[cfg(not(feature = "hover"))]
const CSS: &str = r#"
html,body,main,#main { margin:0; padding:0; width:100%; height:100%; }
* { box-sizing:border-box; font-family:Bench,"DejaVu Sans",sans-serif; }
body { background:white; color:black; font-size:14px; }
.full { width:100%; height:100%; min-height:0; }
.scroll { overflow-y:auto; overflow-x:hidden; }
.row { height:48px; min-height:48px; padding:4px 8px; overflow:hidden; }
.row-body { font-size:12px; }
input { border:1px solid #aaa; border-radius:0; padding:4px 6px; color:black; background:white; height:32px; font-size:14px; }
button { color:black; font-size:14px; }
"#;
