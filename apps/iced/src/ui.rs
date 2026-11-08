#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{row_body, row_title};
use iced::{Element, Subscription, Task, Theme};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use iced::{
    Length,
    widget::{column, container, text},
};
use std::fmt::Debug;

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row_widget<Message: 'static>(index: usize) -> Element<'static, Message> {
    container(column![text(row_title(index)), text(row_body(index)).size(14)].spacing(2))
        .height(Length::Fixed(48.0))
        .padding([4, 12])
        .into()
}

pub fn run<State, Message>(
    init: impl Fn() -> (State, Task<Message>) + 'static,
    update: impl Fn(&mut State, Message) -> Task<Message> + 'static,
    view: impl for<'a> Fn(&'a State) -> Element<'a, Message> + 'static,
    title: impl for<'a> Fn(&'a State) -> String + 'static,
    subscription: impl for<'a> Fn(&'a State) -> Subscription<Message> + 'static,
) -> Result<(), Box<dyn std::error::Error>>
where
    State: 'static,
    Message: Debug + Send + 'static,
{
    iced::application(init, update, view)
        .title(title)
        .theme(|_: &State| Theme::Light)
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            ..Default::default()
        })
        .font(include_bytes!("../../../assets/fonts/DejaVuSans.ttf").as_slice())
        .default_font(iced::Font::with_name("DejaVu Sans"))
        .run()?;
    Ok(())
}
