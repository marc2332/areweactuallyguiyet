#[path = "../../scroll_config.rs"]
mod scroll_config;

use vizia::prelude::*;

pub fn main() -> Result<(), ApplicationError> {
    Application::new(|cx| {
        ScrollView::new(cx, |cx| {
            for number in 1..=scroll_config::row_count() {
                Label::new(cx, format!("Item {number}"))
                    .height(Pixels(48.0))
                    .width(Pixels(960.0))
                    .padding(Pixels(12.0))
                    .alignment(Alignment::Left)
                    .font_size(14.0)
                    .color(Color::rgb(0, 0, 0))
                    .background_color(if number % 2 == 1 {
                        Color::rgb(245, 245, 245)
                    } else {
                        Color::rgb(230, 230, 230)
                    });
            }
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .title("Vizia Scroll")
    .inner_size((1000, 700))
    .run()
}
