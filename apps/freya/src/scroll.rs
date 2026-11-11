#[path = "../../scroll_config.rs"]
mod scroll_config;

use freya::prelude::*;

pub fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("Freya Scroll")
                .with_size(1000., 700.),
        ),
    );
}

fn app() -> impl IntoElement {
    ScrollView::new().child(
        rect().children((0..scroll_config::row_count()).map(|index| {
            rect()
                .key(index)
                .width(Size::px(960.))
                .height(Size::px(48.))
                .padding(12.)
                .background(if index % 2 == 0 {
                    (245, 245, 245)
                } else {
                    (230, 230, 230)
                })
                .child(
                    label()
                        .text(format!("Item {}", index + 1))
                        .font_size(14.)
                        .color(Color::BLACK),
                )
        })),
    )
}
