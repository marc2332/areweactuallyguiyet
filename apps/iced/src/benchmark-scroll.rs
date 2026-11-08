use benchmark_common::{Harness, Scenario};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{column, scrollable},
};
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: scroll".into(),
        App::subscription,
    )
}
struct App {
    harness: Harness,
    scroll_id: iced::widget::Id,
    scroll_offset: f32,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
}
#[derive(Debug, Clone)]
enum Message {
    Tick,
    InitialFrame,
    Scrolled(f32),
}
impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::Scroll).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        (
            Self {
                harness,
                scroll_id: iced::widget::Id::unique(),
                scroll_offset: 0.0,
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
            Message::Scrolled(offset) => self.scroll_offset = offset,
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
        let mut rows = column![].spacing(0);
        for index in 0..Scenario::Scroll.row_count() {
            rows = rows.push(crate::ui::row_widget(index));
        }
        scrollable(rows)
            .id(self.scroll_id.clone())
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
            .height(Length::Fill)
            .into()
    }
}
