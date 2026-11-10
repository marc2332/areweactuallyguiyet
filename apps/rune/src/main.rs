use rune::{App, AppSpec, Color, Cx, El, ScrollSpec, TextStyle, col, color::srgb_decode, text};

fn view(_: &(), _: &mut Cx<()>) -> El {
    col()
        .key("rows")
        .w_full()
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .children((0..2000).map(|index| {
            let gray = srgb_decode(if index % 2 == 0 {
                245.0 / 255.0
            } else {
                230.0 / 255.0
            });
            col()
                .w(960.0)
                .h(48.0)
                .padding(12.0)
                .fill(Color {
                    r: gray,
                    g: gray,
                    b: gray,
                    a: 1.0,
                })
                .child(
                    text(format!("Item {}", index + 1))
                        .font(TextStyle::new(14.0, 400.0))
                        .color(Color::BLACK),
                )
        }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new(AppSpec::new(
        "Rune Scroll 2000",
        (1000.0, 700.0),
        || (),
        view,
    ))
    .try_run()?;
    Ok(())
}
