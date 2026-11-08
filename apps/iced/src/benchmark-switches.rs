use benchmark_common::{
    Harness, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE, SWITCH_OFF_COLOR,
    SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario, Switches,
};
use iced::{
    Element, Length, Subscription, Task, time,
    widget::{column, container, lazy, mouse_area, row, text},
};
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        App::new,
        App::update,
        App::view,
        |_| "Iced benchmark: switches".into(),
        App::subscription,
    )
}
struct App {
    harness: Harness,
    switches: Switches,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
}
#[derive(Debug, Clone)]
enum Message {
    Tick,
    Switch(usize),
}
impl App {
    fn new() -> (Self, Task<Message>) {
        let harness = Harness::connect(Scenario::Switches).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        (
            Self {
                harness,
                switches: Switches::default(),
                applied_updates: 0,
                ready_sent: false,
                reset_generation: 0,
            },
            Task::none(),
        )
    }
    fn subscription(_: &Self) -> Subscription<Message> {
        time::every(Duration::from_millis(33)).map(|_| Message::Tick)
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Switch(index) => {
                if index < SWITCH_COUNT {
                    self.switches.toggle(index);
                }
            }
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
                        self.switches = Switches::default();
                        self.reset_generation = state.reset_generation;
                    }
                    if state.running {
                        self.switches
                            .advance(state.update_index, self.harness.switch_mode());
                        self.applied_updates = self.switches.applied_updates();
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
        let mut grid = column![].spacing(0);
        for row_index in 0..SWITCH_ROWS {
            let mut switch_row = row![].spacing(0);
            for column_index in 0..SWITCH_COLUMNS {
                let index = row_index * SWITCH_COLUMNS + column_index;
                let is_on = self.switches.get(index);
                switch_row = switch_row.push(lazy((index, is_on), move |&(index, is_on)| {
                    cell(index, is_on)
                }));
            }
            grid = grid.push(switch_row);
        }
        grid.into()
    }
}
fn cell(index: usize, is_on: bool) -> Element<'static, Message> {
    let (red, green, blue) = if is_on {
        SWITCH_ON_COLOR
    } else {
        SWITCH_OFF_COLOR
    };
    let (text_red, text_green, text_blue) = SWITCH_TEXT_COLOR;
    mouse_area(
        container(
            text(if is_on { "ON" } else { "OFF" })
                .size(SWITCH_FONT_SIZE)
                .color(iced::Color::from_rgb8(text_red, text_green, text_blue)),
        )
        .center_x(Length::Fixed(SWITCH_CELL_SIZE))
        .center_y(Length::Fixed(SWITCH_CELL_SIZE))
        .style(move |_| container::Style {
            background: Some(iced::Color::from_rgb8(red, green, blue).into()),
            ..Default::default()
        }),
    )
    .on_press(Message::Switch(index))
    .into()
}
