use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, image, row, scrollable, text, text_input, toggler},
};

#[derive(Debug, Clone)]
enum Message {
    Notes(String),
    Refresh,
    Toggle(bool),
}
struct Dashboard {
    notes: String,
    refresh_count: usize,
    enabled: bool,
    images: Vec<image::Handle>,
}

fn update(state: &mut Dashboard, message: Message) {
    match message {
        Message::Notes(value) => state.notes = value,
        Message::Refresh => state.refresh_count += 1,
        Message::Toggle(value) => state.enabled = value,
    }
}

fn view(state: &Dashboard) -> Element<'_, Message> {
    let toolbar = column![
        text("Project dashboard").size(24),
        row![
            text_input("", &state.notes)
                .on_input(Message::Notes)
                .width(360)
                .size(16),
            Space::new().width(Length::Fixed(32.0)),
            button("Refresh")
                .on_press(Message::Refresh)
                .width(160)
                .height(40),
            Space::new().width(Length::Fixed(48.0)),
            toggler(state.enabled)
                .label(if state.enabled { "Enabled" } else { "Disabled" })
                .on_toggle(Message::Toggle)
                .width(160),
        ]
        .spacing(0),
        text(format!("Refresh count: {}", state.refresh_count)),
    ]
    .spacing(17)
    .padding(24)
    .height(180);
    let records = (0..100).fold(column![].spacing(0), |list, index| {
        list.push(
            container(
                row![
                    image::Image::new(state.images[index % state.images.len()].clone())
                        .width(64)
                        .height(64),
                    column![
                        text(format!("Project {}", index + 1)),
                        text("A project record with descriptive dashboard information.").size(14)
                    ],
                ]
                .spacing(16)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .padding(12),
        )
    });
    column![toolbar, scrollable(records).height(Length::Fill)].into()
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    for bytes in [
        include_bytes!("../../../assets/images/dashboard-0.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-1.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-2.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-3.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-4.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-5.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-6.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-7.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-8.png").as_slice(),
        include_bytes!("../../../assets/images/dashboard-9.png").as_slice(),
    ] {
        let decoded = ::image::load_from_memory(bytes)?.to_rgba8();
        images.push(image::Handle::from_rgba(
            decoded.width(),
            decoded.height(),
            decoded.into_raw(),
        ));
    }
    iced::application(
        move || Dashboard {
            notes: "Project notes".into(),
            refresh_count: 0,
            enabled: false,
            images: images.clone(),
        },
        update,
        view,
    )
    .title("Iced Scroll 2000")
    .theme(Theme::Light)
    .window_size(iced::Size::new(1000.0, 700.0))
    .run()?;
    Ok(())
}
