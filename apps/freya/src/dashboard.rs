use freya::prelude::*;

pub fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("Freya Scroll 2000")
                .with_size(1000., 700.),
        ),
    );
}

fn app() -> impl IntoElement {
    let notes = use_state(|| String::from("Project notes"));
    let mut refresh_count = use_state(|| 0usize);
    let mut enabled = use_state(|| true);
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(24.)
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .height(Size::fill())
                .child(rect().height(Size::px(30.)).child(label().text("Project dashboard").font_size(24.)))
                .child(rect().height(Size::px(16.)))
                .child(
                    rect()
                        .horizontal()
                        .height(Size::px(40.))
                        .child(Input::new(notes).width(Size::px(360.)).height(Size::px(40.)))
                        .child(rect().width(Size::px(32.)))
                        .child(
                            Button::new()
                                .on_press(move |_| *refresh_count.write() += 1)
                                .width(Size::px(160.))
                                .height(Size::px(40.))
                                .child("Refresh"),
                        )
                        .child(rect().width(Size::px(48.)))
                        .child(
                            Button::new()
                                .on_press(move |_| {
                                    let next = !*enabled.read();
                                    *enabled.write() = next;
                                })
                                .width(Size::px(160.))
                                .height(Size::px(40.))
                                .child(if *enabled.read() { "Enabled" } else { "Disabled" }),
                        ),
                )
                .child(rect().height(Size::px(14.)))
                .child(rect().height(Size::px(24.)).child(label().text(format!("Refresh count: {}", *refresh_count.read()))))
                .child(rect().height(Size::px(32.)))
                .child(
                    ScrollView::new()
                        .height(Size::fill())
                        .child(rect().vertical().children((0..100).map(|index| {
                            let image_index = index % 10;
                            let bytes: &'static [u8] = match image_index {
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
                            rect()
                                .key(index)
                                .horizontal()
                                .height(Size::px(96.))
                                .width(Size::fill())
                                .padding(12.)
                                .background(if index % 2 == 0 { (245, 245, 245) } else { (230, 230, 230) })
                                .child(
                                    ImageViewer::new((image_index, bytes))
                                        .width(Size::px(64.))
                                        .height(Size::px(64.)),
                                )
                                .child(
                                    rect().vertical().expanded()
                                        .child(label().text(format!("Project {}", index + 1)).font_size(16.))
                                        .child(label().text("Project overview, recent activity, and notes for this workspace.")),
                                )
                        }))),
                ),
        )
}

#[cfg(test)]
mod tests {
    use crate::dashboard::app;
    use freya::prelude::Label;
    use freya_testing::TestingRunner;

    #[test]
    fn dashboard_controls_respond_at_fixed_click_coordinates() {
        let (mut runner, _) = TestingRunner::new(app, (1000., 700.).into(), |_| (), 1.);
        runner.sync_and_update();
        assert!(has_label(&runner, "Refresh count: 0"));
        assert!(has_label(&runner, "Enabled"));

        runner.click_cursor((496., 90.));
        assert!(has_label(&runner, "Refresh count: 1"));

        runner.click_cursor((704., 90.));
        assert!(has_label(&runner, "Disabled"));
    }

    fn has_label(runner: &TestingRunner, text: &str) -> bool {
        runner
            .find(|_, element| {
                let label = Label::try_downcast(element)?;
                (label.text.as_ref() == text).then_some(())
            })
            .is_some()
    }
}
