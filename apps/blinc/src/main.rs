use blinc_app::{prelude::*, windowed::WindowedApp};

fn main() -> Result<()> {
    WindowedApp::run(
        WindowConfig::new("Blinc Scroll 2000").size(1000, 700),
        |context| {
            let mut rows = div().flex_col().w_full().h_fit();
            for index in 0..2000 {
                let gray = if index % 2 == 0 {
                    245.0 / 255.0
                } else {
                    230.0 / 255.0
                };
                rows = rows.child(
                    div()
                        .w(960.0)
                        .h(48.0)
                        .flex_shrink_0()
                        .p_px(12.0)
                        .bg(Color::rgb(gray, gray, gray))
                        .child(
                            text(format!("Item {}", index + 1))
                                .size(14.0)
                                .color(Color::BLACK),
                        ),
                );
            }
            div()
                .w(context.width)
                .h(context.height)
                .child(scroll().w_full().h_full().child(rows))
        },
    )
}
