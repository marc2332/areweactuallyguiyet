use vizia::prelude::*;

pub fn main() -> Result<(), ApplicationError> {
    Application::new(|cx| {
        cx.load_image(
            "dashboard-0.png",
            include_bytes!("../../../assets/images/dashboard-0.png"),
            ImageRetentionPolicy::Forever,
        );

        let notes = Signal::new("Project notes".to_string());
        let refresh_count = Signal::new(0_u32);
        let enabled = Signal::new(true);

        VStack::new(cx, |cx| {
            Label::new(cx, "Project dashboard")
                .width(Stretch(1.0))
                .height(Pixels(50.0))
                .padding_left(Pixels(24.0))
                .font_size(24.0)
                .alignment(Alignment::Left);

            HStack::new(cx, |cx| {
                Textbox::new(cx, notes)
                    .on_edit(move |_, text| notes.set(text))
                    .width(Pixels(360.0))
                    .height(Pixels(40.0));

                Button::new(cx, |cx| Label::new(cx, "Refresh"))
                    .on_press(move |_| refresh_count.update(|count| *count += 1))
                    .width(Pixels(160.0))
                    .height(Pixels(40.0))
                    .padding_right(Pixels(12.0));

                Switch::new(cx, enabled)
                    .on_toggle(move |_| enabled.update(|value| *value = !*value))
                    .width(Pixels(160.0))
                    .height(Pixels(40.0));
            })
            .width(Stretch(1.0))
            .height(Pixels(40.0))
            .padding_left(Pixels(24.0))
            .gap(Pixels(32.0));

            Label::new(
                cx,
                refresh_count.map(|count| format!("Refresh count: {count}")),
            )
            .width(Stretch(1.0))
            .height(Pixels(30.0))
            .padding_left(Pixels(24.0))
            .font_size(14.0)
            .alignment(Alignment::Left);

            ScrollView::new(cx, |cx| {
                for number in 1..=1000 {
                    HStack::new(cx, |cx| {
                        Image::new(cx, "dashboard-0.png")
                            .width(Pixels(64.0))
                            .height(Pixels(64.0));
                        VStack::new(cx, |cx| {
                            Label::new(cx, format!("Project {number}"))
                                .font_size(16.0)
                                .alignment(Alignment::Left);
                            Label::new(cx, "Project details and notes for this record")
                                .font_size(13.0)
                                .alignment(Alignment::Left);
                        })
                        .width(Stretch(1.0))
                        .height(Pixels(64.0))
                        .alignment(Alignment::Left);
                    })
                    .width(Stretch(1.0))
                    .height(Pixels(80.0))
                    .padding(Pixels(8.0))
                    .gap(Pixels(12.0))
                    .background_color(if number % 2 == 1 {
                        Color::rgb(245, 245, 245)
                    } else {
                        Color::rgb(230, 230, 230)
                    });
                }
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .padding_left(Pixels(24.0))
            .padding_right(Pixels(24.0));
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .background_color(Color::white())
        .color(Color::black())
        .gap(Pixels(20.0));
    })
    .title("Vizia Scroll 2000")
    .inner_size((1000, 700))
    .run()
}
