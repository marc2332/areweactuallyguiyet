use iced::{
    Color, Element, Length, Theme,
    widget::{column, container, scrollable, text},
};

fn view(_state: &()) -> Element<'_, ()> {
    let mut rows = column![].spacing(0).width(Length::Fill);
    for index in 0..2_000 {
        let background = if index % 2 == 0 {
            Color::from_rgb8(245, 245, 245)
        } else {
            Color::from_rgb8(230, 230, 230)
        };
        rows = rows.push(
            container(text(format!("Item {}", index + 1)).size(14))
                .width(960)
                .height(48)
                .padding(12)
                .style(move |_| container::Style {
                    background: Some(background.into()),
                    text_color: Some(Color::BLACK),
                    ..Default::default()
                }),
        );
    }
    scrollable(rows)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    iced::application(|| (), |_: &mut (), _: ()| {}, view)
        .title("Iced Scroll 2000")
        .theme(Theme::Light)
        .window_size(iced::Size::new(1000.0, 700.0))
        .run()
}
