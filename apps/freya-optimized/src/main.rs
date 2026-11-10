use freya::prelude::*;

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("Freya Scroll 2000")
                .with_size(1000., 700.),
        ),
    );
}

fn app() -> impl IntoElement {
    ScrollView::new().child(rect().children((0..2000).map(|index| {
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
    })))
}
