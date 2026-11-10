use whisker::prelude::*;
use whisker_desktop::DesktopAppConfig;

pub fn run() -> Result<(), whisker_desktop::DesktopAppError> {
    let mut config = DesktopAppConfig::new("Whisker Scroll 2000");
    config.width = 1000.0;
    config.height = 700.0;
    whisker_desktop::run(config, app)
}

fn app() -> Element {
    ScrollView::builder()
        .style(Css::new().width(percent(100)).height(percent(100)))
        .body(|body| {
            body.push(
                View::builder()
                    .style(
                        Css::new()
                            .display(Display::Flex)
                            .flex_direction(FlexDirection::Column)
                            .width(px(960))
                            .flex_shrink(0.0),
                    )
                    .body(|rows| {
                        for number in 1..=2000 {
                            rows.push(
                                View::builder()
                                    .style(
                                        Css::new()
                                            .width(px(960))
                                            .height(px(48))
                                            .flex_shrink(0.0)
                                            .padding(px(12))
                                            .background_color(Color::hex(if number % 2 == 1 {
                                                0xf5f5f5
                                            } else {
                                                0xe6e6e6
                                            })),
                                    )
                                    .body(|row| {
                                        row.push(
                                            Text::builder()
                                                .value(format!("Item {number}"))
                                                .style(
                                                    Css::new()
                                                        .font_size(px(14))
                                                        .color(Color::hex(0x000000)),
                                                )
                                                .build(),
                                        );
                                    })
                                    .build(),
                            );
                        }
                    })
                    .build(),
            );
        })
        .build()
}
