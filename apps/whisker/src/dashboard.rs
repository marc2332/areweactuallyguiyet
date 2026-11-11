use whisker::css::{FontWeight, PositionKind};
use whisker::prelude::*;
use whisker_desktop::DesktopAppConfig;
use whisker_image::{Image, ImageMode};
use whisker_input::Input;

pub fn run() -> Result<(), whisker_desktop::DesktopAppError> {
    let mut config = DesktopAppConfig::new("Whisker Scroll 2000");
    config.width = 1000.0;
    config.height = 700.0;
    config = config
        .with_module(
            whisker_input::__whisker_element_module_definition(),
            whisker_input_desktop::__whisker_module_definition(),
        )
        .with_module(
            whisker_image::__whisker_element_module_definition(),
            whisker_image_desktop::__whisker_module_definition(),
        );
    whisker_desktop::run(config, app)
}

#[whisker::main]
fn app() -> Element {
    let notes = signal(String::from("Project notes"));
    let refresh_count = signal(0_u32);
    let enabled = signal(true);

    render! {
        View(style: Css::new().width(percent(100)).height(percent(100)).background_color(Color::hex(0xffffff))) {
            Text(value: "Project dashboard", style: Css::new().position(PositionKind::Absolute).left(px(24)).top(px(24)).font_size(px(24)).font_weight(FontWeight::Bold).color(Color::hex(0x202124)))
            View(style: Css::new().position(PositionKind::Absolute).left(px(24)).top(px(70)).width(px(360)).height(px(40))) {
                Input(text: notes, placeholder: "Project notes", style: Css::new().width(px(360)).height(px(40)).padding(px(8)).background_color(Color::hex(0xf1f3f4)))
            }
            View(on_tap: move |_| refresh_count.update(|count| *count += 1), style: Css::new().position(PositionKind::Absolute).left(px(416)).top(px(70)).width(px(160)).height(px(40)).justify_content(JustifyContent::Center).align_items(AlignItems::Center).background_color(Color::hex(0x1a73e8))) {
                Text(value: "Refresh", style: Css::new().color(Color::hex(0xffffff)).font_size(px(14)))
            }
            View(on_tap: move |_| enabled.update(|value| *value = !*value), style: Css::new().position(PositionKind::Absolute).left(px(624)).top(px(70)).width(px(160)).height(px(40)).justify_content(JustifyContent::Center).align_items(AlignItems::Center).background_color(Color::hex(0xe8eaed))) {
                Text(value: computed(move || if enabled.get() { "Enabled".to_owned() } else { "Disabled".to_owned() }), style: Css::new().color(Color::hex(0x202124)).font_size(px(14)))
            }
            Text(value: computed(move || format!("Status: {} refreshes · {}", refresh_count.get(), if enabled.get() { "enabled" } else { "disabled" })), style: Css::new().position(PositionKind::Absolute).left(px(24)).top(px(130)).font_size(px(14)).color(Color::hex(0x5f6368)))
            ScrollView(axis: ScrollAxis::Vertical, style: Css::new().position(PositionKind::Absolute).left(px(24)).right(px(24)).top(px(180)).bottom(px(16))) {
                View(style: Css::new().display(Display::Flex).flex_direction(FlexDirection::Column).gap(px(8)).width(percent(100))) {
                    ForEach(each: || (1..=1000).collect::<Vec<usize>>(), key: |number: &usize| *number, children: |number: ReadSignal<usize>| {
                        let number = number.get_untracked();
                        render! {
                            View(style: Css::new().display(Display::Flex).flex_direction(FlexDirection::Row).align_items(AlignItems::Center).gap(px(16)).width(percent(100)).height(px(88)).padding(px(12)).background_color(Color::hex(if number % 2 == 0 { 0xf5f5f5 } else { 0xffffff }))) {
                                Image(src: "assets/images/dashboard-0.png", mode: ImageMode::AspectFit, style: Css::new().width(px(64)).height(px(64)).flex_shrink(0.0))
                                View(style: Css::new().display(Display::Flex).flex_direction(FlexDirection::Column).gap(px(6))) {
                                    Text(value: format!("Project {number}"), style: Css::new().font_size(px(16)).font_weight(FontWeight::Bold).color(Color::hex(0x202124)))
                                    Text(value: format!("Project {number} overview and planning notes."), style: Css::new().font_size(px(13)).color(Color::hex(0x5f6368)))
                                }
                            }
                        }
                    })
                }
            }
        }
    }
}
