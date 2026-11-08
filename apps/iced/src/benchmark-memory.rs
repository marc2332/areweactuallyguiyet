use benchmark_common::{
    Harness, Scenario,
    dashboard::{
        Dashboard, FIELD_COUNT, HEADER_HEIGHT, RECORD_HEIGHT, RECORDS_WIDTH, SECTION_COUNT,
        SIDEBAR_WIDTH, THUMBNAILS,
    },
};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{button, column, image, row, scrollable, text, text_input},
};
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: memory".into(),
        App::subscription,
    )
}
struct App {
    harness: Harness,
    dashboard: Dashboard,
    thumbnails: Vec<image::Handle>,
    scroll_id: iced::widget::Id,
    detail_scroll_id: iced::widget::Id,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
}
#[derive(Debug, Clone)]
enum Message {
    Tick,
    InitialFrame,
    Search(String),
    Navigate(usize),
    Select(usize),
    Edit(usize, String),
    Toggle(usize),
}
impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::Memory).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        let thumbnails = THUMBNAILS
            .iter()
            .map(|bytes| image::Handle::from_bytes(bytes.to_vec()))
            .collect();
        (
            Self {
                harness,
                dashboard: Dashboard::default(),
                thumbnails,
                scroll_id: iced::widget::Id::unique(),
                detail_scroll_id: iced::widget::Id::unique(),
                applied_updates: 0,
                ready_sent: false,
                reset_generation: 0,
            },
            Task::none(),
        )
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(Duration::from_millis(16)).map(|_| Message::Tick),
            if self.ready_sent {
                Subscription::none()
            } else {
                iced::window::frames().map(|_| Message::InitialFrame)
            },
        ])
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InitialFrame => {
                if !self.ready_sent {
                    if let Err(error) = self.harness.mark_ready("initial-draw-returned-not-presented") {
                        eprintln!("Iced benchmark protocol failed: {error}");
                        return iced::exit();
                    }
                    self.ready_sent = true;
                }
            }
            Message::Search(query) => self.dashboard.search(query),
            Message::Navigate(section) => self.dashboard.navigate(section),
            Message::Select(index) => self.dashboard.select(index),
            Message::Edit(index, value) => self.dashboard.edit(index, value),
            Message::Toggle(index) => self.dashboard.toggle(index),
            Message::Tick => match self.harness.poll() {
                Ok(state) => {
                    if state.should_quit {
                        return iced::exit();
                    }
                    if state.reset_generation != self.reset_generation {
                        self.applied_updates = 0;
                        self.dashboard.reset();
                        self.reset_generation = state.reset_generation;
                        return Task::batch([
                            iced::widget::operation::scroll_to(
                                self.scroll_id.clone(),
                                iced::widget::operation::AbsoluteOffset {
                                    x: None,
                                    y: Some(0.0),
                                },
                            ),
                            iced::widget::operation::scroll_to(
                                self.detail_scroll_id.clone(),
                                iced::widget::operation::AbsoluteOffset {
                                    x: None,
                                    y: Some(0.0),
                                },
                            ),
                        ]);
                    }
                    if state.running {
                        self.dashboard.advance(state);
                        self.applied_updates = self.dashboard.applied_updates();
                        if let Err(error) = self.harness.report_dashboard(&self.dashboard) {
                            eprintln!("Iced benchmark protocol failed: {error}");
                            std::process::exit(2);
                        }
                    }
                    if let Err(error) = self.harness.report_applied(self.applied_updates) {
                        eprintln!("Iced benchmark protocol failed: {error}");
                        std::process::exit(2);
                    }
                    if state.running {
                        return iced::widget::operation::scroll_to(
                            self.scroll_id.clone(),
                            iced::widget::operation::AbsoluteOffset {
                                x: None,
                                y: Some(state.scroll_offset),
                            },
                        );
                    }
                }
                Err(error) => {
                    eprintln!("Iced benchmark protocol failed: {error}");
                    std::process::exit(2);
                }
            },
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        let dashboard = &self.dashboard;
        let header = row![
            text("Iced dashboard").size(24),
            text_input("Search records", &dashboard.query).on_input(Message::Search)
        ]
        .spacing(16)
        .padding(12)
        .height(Length::Fixed(HEADER_HEIGHT));
        let mut sections = column![].spacing(0);
        for section in 0..SECTION_COUNT {
            let label = if section == 0 {
                "All records".to_owned()
            } else {
                format!("Section {section:02}")
            };
            let selected = dashboard.section == section;
            sections = sections.push(
                button(text(label).size(12))
                    .height(Length::Fixed(22.0))
                    .padding(2)
                    .on_press(Message::Navigate(section))
                    .width(Length::Fill)
                    .style(if selected {
                        button::primary
                    } else {
                        button::secondary
                    }),
            );
        }
        let mut thumbnails = column![].spacing(4);
        for pair in self.thumbnails.chunks(2) {
            let mut thumb_row = row![].spacing(4);
            for handle in pair {
                thumb_row = thumb_row.push(
                    image(handle.clone())
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(48.0)),
                );
            }
            thumbnails = thumbnails.push(thumb_row);
        }
        let sidebar = scrollable(
            column![text("Sections"), sections, text("Thumbnails"), thumbnails]
                .spacing(8)
                .padding(8),
        )
        .height(Length::Fill)
        .width(Length::Fixed(SIDEBAR_WIDTH));
        let mut records = column![].spacing(2);
        for index in dashboard.visible_records() {
            let record = &dashboard.fixtures.records[index];
            let selected = index == dashboard.selected_record;
            let status = if dashboard.active.get(index) {
                "ON"
            } else {
                "OFF"
            };
            let title = if selected {
                format!("▶ {}", record.title)
            } else {
                record.title.clone()
            };
            records = records.push(
                row![
                    button(column![text(title), text(&record.body).size(12)].spacing(2))
                        .on_press(Message::Select(index))
                        .width(Length::Fill),
                    button(text(status)).on_press(Message::Toggle(index))
                ]
                .spacing(4)
                .height(Length::Fixed(RECORD_HEIGHT))
                .padding(2),
            );
        }
        let record_pane = scrollable(records)
            .id(self.scroll_id.clone())
            .height(Length::Fill)
            .width(Length::Fixed(RECORDS_WIDTH));
        let mut fields = column![text("Editable properties")].spacing(6);
        for index in 0..FIELD_COUNT {
            fields = fields.push(
                row![
                    text(format!("Field {index:02}")).width(Length::Fixed(90.0)),
                    text_input("Value", &dashboard.fields[index])
                        .on_input(move |value| Message::Edit(index, value))
                ]
                .spacing(6),
            );
        }
        let detail = scrollable(text(dashboard.detail.as_ref()).size(14))
            .id(self.detail_scroll_id.clone())
            .height(Length::Fixed(300.0));
        let fields = scrollable(fields)
            .id(iced::widget::Id::new("dashboard-fields"))
            .height(Length::Fill);
        let right = column![
            text(format!("Record {:03}", dashboard.selected_record)),
            detail,
            fields
        ]
        .spacing(8)
        .padding(8)
        .width(Length::Fill);
        column![
            header,
            row![sidebar, record_pane, right]
                .spacing(4)
                .height(Length::Fill)
        ]
        .spacing(0)
        .into()
    }
}
