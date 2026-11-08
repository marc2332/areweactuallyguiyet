use benchmark_common::{Harness, Scenario};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{button, column, row, scrollable, text, text_input},
};
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: startup".into(),
        App::subscription,
    )
}

struct App {
    harness: Harness,
    scroll_id: iced::widget::Id,
    input: String,
    message: String,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    InitialFrame,
    Input(String),
    Button,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::Startup).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        (
            Self {
                harness,
                scroll_id: iced::widget::Id::unique(),
                input: String::new(),
                message: String::from("Ready"),
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
            Message::Input(value) => self.input = value,
            Message::Button => self.message = format!("Submitted: {}", self.input),
            Message::Tick => match self.harness.poll() {
                Ok(state) => {
                    if state.should_quit {
                        return iced::exit();
                    }
                    if state.reset_generation != self.reset_generation {
                        self.applied_updates = 0;
                        self.reset_generation = state.reset_generation;
                        return iced::widget::operation::scroll_to(
                            self.scroll_id.clone(),
                            iced::widget::operation::AbsoluteOffset {
                                x: None,
                                y: Some(0.0),
                            },
                        );
                    }
                    if state.running {
                        self.applied_updates = state.update_index;
                    }
                    if let Err(error) = self.harness.report_applied(self.applied_updates) {
                        eprintln!("Iced benchmark protocol failed: {error}");
                        std::process::exit(2);
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
        let mut rows = column![].spacing(0);
        for index in 0..Scenario::Startup.row_count() {
            rows = rows.push(crate::ui::row_widget(index));
        }
        let header = column![
            text("Iced startup benchmark").size(28),
            text("Shared deterministic startup content"),
            row![
                text_input("Type here", &self.input).on_input(Message::Input),
                button("Submit").on_press(Message::Button),
                text(&self.message)
            ]
            .spacing(12)
        ]
        .spacing(10)
        .padding(16);
        column![
            header,
            scrollable(rows)
                .id(self.scroll_id.clone())
                .height(Length::Fill)
        ]
        .spacing(8)
        .into()
    }
}
