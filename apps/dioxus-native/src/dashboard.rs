use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus_native::{LogicalSize, WindowAttributes, prelude::*};

pub fn run() {
    dioxus_native::launch_cfg(
        app,
        vec![],
        vec![Box::new(
            WindowAttributes::default()
                .with_title("Dioxus Native Scroll 2000")
                .with_surface_size(LogicalSize::new(1000.0, 700.0)),
        )],
    );
}
fn app() -> Element {
    let mut refreshes = use_signal(|| 0usize);
    let mut enabled = use_signal(|| true);
    let mut notes = use_signal(|| String::from("Project notes"));
    rsx! { style { "{CSS}" }
        div { class: "page",
            h1 { "Project dashboard" }
            div { class: "toolbar",
                input { value: "{notes}", oninput: move |event| notes.set(event.value()) }
                button { onclick: move |_| refreshes += 1, "Refresh" }
                button { onclick: move |_| enabled.toggle(), if enabled() { "Enabled" } else { "Disabled" } }
            }
            div { class: "status", "Refresh count: {refreshes()}" }
            div { class: "list", for number in 1..=100 { article { key: "{number}", img { src: "{dashboard_image_src((number - 1) % 10)}", alt: "Project preview" } div { h2 { "Project {number}" } p { "Project overview, recent activity, and notes for this workspace." } } } } }
        }
    }
}
const CSS: &str = r#"html,body,main,#main{margin:0;padding:0;width:100%;height:100%}*{box-sizing:border-box;font-family:sans-serif}body{color:#172033;background:#f6f8fb;font-size:14px}.page{width:100%;height:100%;padding:24px;display:flex;flex-direction:column}h1{font-size:24px;margin:0 0 22px}.toolbar{display:flex;gap:32px;height:40px;flex:none}.toolbar input{width:360px;height:40px;padding:6px 10px;border:1px solid #aab4c4;background:white;color:#172033;font-size:14px}.toolbar button{width:160px;height:40px;font-size:14px;color:#172033}.status{height:48px;padding-top:12px;flex:none}.list{flex:1;min-height:0;overflow-y:auto;display:flex;flex-direction:column;gap:8px}article{height:96px;min-height:96px;display:flex;align-items:center;gap:16px;padding:12px;background:white;border:1px solid #e0e5ed}article img{width:64px;height:64px;object-fit:cover}h2{font-size:16px;margin:0 0 8px}p{margin:0;color:#586579}"#;

fn dashboard_image_src(index: usize) -> String {
    let bytes: &[u8] = match index {
        0 => include_bytes!("../../../assets/images/dashboard-0.png"),
        1 => include_bytes!("../../../assets/images/dashboard-1.png"),
        2 => include_bytes!("../../../assets/images/dashboard-2.png"),
        3 => include_bytes!("../../../assets/images/dashboard-3.png"),
        4 => include_bytes!("../../../assets/images/dashboard-4.png"),
        5 => include_bytes!("../../../assets/images/dashboard-5.png"),
        6 => include_bytes!("../../../assets/images/dashboard-6.png"),
        7 => include_bytes!("../../../assets/images/dashboard-7.png"),
        8 => include_bytes!("../../../assets/images/dashboard-8.png"),
        _ => include_bytes!("../../../assets/images/dashboard-9.png"),
    };
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}
