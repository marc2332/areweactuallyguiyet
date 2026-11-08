use benchmark_common::{HEIGHT, Harness, ROW_HEIGHT, Scenario};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{column, scrollable, space},
};
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: virtual-scroll".into(),
        App::subscription,
    )
}
struct App {
    harness: Harness,
    scroll_id: iced::widget::Id,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
    scroll_offset: f32,
}
#[derive(Debug, Clone)]
enum Message {
    Tick,
    Scrolled(f32),
}
impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::VirtualScroll).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        (
            Self {
                harness,
                scroll_id: iced::widget::Id::unique(),
                applied_updates: 0,
                ready_sent: false,
                reset_generation: 0,
                scroll_offset: 0.0,
            },
            Task::none(),
        )
    }
    fn subscription(_: &Self) -> Subscription<Message> {
        time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scrolled(offset) => self.scroll_offset = offset,
            Message::Tick => match self.harness.poll() {
                Ok(state) => {
                    if state.should_quit {
                        return iced::exit();
                    }
                    if !self.ready_sent {
                        if let Err(error) = self.harness.mark_ready(
                            "initial widget tree constructed; first GUI event-loop tick",
                        ) {
                            eprintln!("Iced benchmark protocol failed: {error}");
                            std::process::exit(2);
                        }
                        self.ready_sent = true;
                    }
                    if state.reset_generation != self.reset_generation {
                        self.applied_updates = 0;
                        self.reset_generation = state.reset_generation;
                        self.scroll_offset = 0.0;
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
                        self.scroll_offset = state.scroll_offset;
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
        let count = Scenario::VirtualScroll.row_count();
        let first_visible = (self.scroll_offset / 48.0).floor().max(0.0) as usize;
        let visible_rows = (HEIGHT / ROW_HEIGHT).ceil() as usize;
        let start = first_visible.saturating_sub(5).min(count);
        let end = (first_visible + visible_rows + 5).min(count);
        let mut rows = column![]
            .spacing(0)
            .push(space().height(Length::Fixed((start * 48) as f32)));
        for index in start..end {
            rows = rows.push(crate::ui::row_widget(index));
        }
        rows = rows.push(space().height(Length::Fixed((count.saturating_sub(end) * 48) as f32)));
        scrollable(rows)
            .id(self.scroll_id.clone())
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
            .height(Length::Fill)
            .into()
    }
}
