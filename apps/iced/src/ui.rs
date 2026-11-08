use benchmark_common::{
    HEIGHT, Harness, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE,
    SWITCH_OFF_COLOR, SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario, Switches, WIDTH,
    dashboard::{
        Dashboard, FIELD_COUNT, HEADER_HEIGHT, RECORD_HEIGHT, RECORDS_WIDTH, SECTION_COUNT,
        SIDEBAR_WIDTH, THUMBNAILS,
    },
    row_body, row_title,
};
use iced::{
    Element, Length, Subscription, Task, Theme, time,
    widget::{
        button, column, container, image, mouse_area, row, scrollable, space, text, text_input,
    },
};
use std::time::Duration;

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    iced::application(move || App::new(scenario), App::update, App::view)
        .title(|app: &App| format!("Iced benchmark: {}", app.scenario.id()))
        .theme(|_app: &App| Theme::Light)
        .subscription(|app: &App| app.subscription())
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            ..Default::default()
        })
        .font(include_bytes!("../../../assets/fonts/DejaVuSans.ttf").as_slice())
        .default_font(iced::Font::with_name("DejaVu Sans"))
        .run()?;
    Ok(())
}

struct App {
    scenario: Scenario,
    harness: Harness,
    switches: Switches,
    scroll_id: iced::widget::Id,
    input: String,
    message: String,
    applied_updates: u64,
    ready_sent: bool,
    reset_generation: u64,
    scroll_offset: f32,
    dashboard: Option<Dashboard>,
    detail_scroll_id: iced::widget::Id,
    thumbnails: Option<Vec<image::Handle>>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Input(String),
    Button,
    Switch(usize),
    Scrolled(f32),
    DashboardSearch(String),
    DashboardNavigate(usize),
    DashboardSelect(usize),
    DashboardEdit(usize, String),
    DashboardToggle(usize),
}

impl App {
    fn new(scenario: Scenario) -> (Self, Task<Message>) {
        let harness = Harness::connect(scenario).unwrap_or_else(|error| {
            eprintln!("Iced benchmark control connection failed: {error}");
            std::process::exit(2);
        });
        (
            Self {
                scenario,
                harness,
                switches: Switches::default(),
                scroll_id: iced::widget::Id::unique(),
                input: String::new(),
                message: String::from("Ready"),
                applied_updates: 0,
                ready_sent: false,
                reset_generation: 0,
                scroll_offset: 0.0,
                dashboard: (scenario == Scenario::Memory).then(Dashboard::default),
                detail_scroll_id: iced::widget::Id::unique(),
                thumbnails: (scenario == Scenario::Memory).then(|| {
                    THUMBNAILS
                        .iter()
                        .map(|bytes| image::Handle::from_bytes(bytes.to_vec()))
                        .collect()
                }),
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let interval = if self.scenario == Scenario::Switches {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(16)
        };
        time::every(interval).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Input(value) => self.input = value,
            Message::Button => self.message = format!("Submitted: {}", self.input),
            Message::Switch(index) => {
                if index < SWITCH_COUNT {
                    self.switches.toggle(index);
                }
            }
            Message::Scrolled(offset) => self.scroll_offset = offset,
            Message::DashboardSearch(query) => {
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.search(query);
                }
            }
            Message::DashboardNavigate(section) => {
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.navigate(section);
                }
            }
            Message::DashboardSelect(index) => {
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.select(index);
                }
            }
            Message::DashboardEdit(index, value) => {
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.edit(index, value);
                }
            }
            Message::DashboardToggle(index) => {
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.toggle(index);
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
                        if let Some(dashboard) = &mut self.dashboard {
                            dashboard.reset();
                        }
                        self.reset_generation = state.reset_generation;
                        if self.scenario != Scenario::Switches {
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
                    }
                    if state.running {
                        if self.scenario == Scenario::Switches {
                            self.switches
                                .advance(state.update_index, self.harness.switch_mode());
                            self.applied_updates = self.switches.applied_updates();
                        } else if let Some(dashboard) = &mut self.dashboard {
                            dashboard.advance(state);
                            self.applied_updates = dashboard.applied_updates();
                            if let Err(error) = self.harness.report_dashboard(dashboard) {
                                eprintln!("Iced benchmark protocol failed: {error}");
                                std::process::exit(2);
                            }
                        } else {
                            self.applied_updates = state.update_index;
                        }
                        if self.scenario == Scenario::VirtualScroll {
                            self.scroll_offset = state.scroll_offset;
                        }
                        if matches!(
                            self.scenario,
                            Scenario::Scroll | Scenario::VirtualScroll | Scenario::Memory
                        ) {
                            if let Err(error) = self.harness.report_applied(self.applied_updates) {
                                eprintln!("Iced benchmark protocol failed: {error}");
                                std::process::exit(2);
                            }
                            return iced::widget::operation::scroll_to(
                                self.scroll_id.clone(),
                                iced::widget::operation::AbsoluteOffset {
                                    x: None,
                                    y: Some(state.scroll_offset),
                                },
                            );
                        }
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
        match self.scenario {
            Scenario::Scroll | Scenario::VirtualScroll => self.row_view(),
            Scenario::Switches => self.switch_view(),
            Scenario::Memory => self.dashboard_view(),
            _ => {
                let header = column![
                    text(format!("Iced {} benchmark", self.scenario.id())).size(28),
                    text("Shared deterministic startup content"),
                    row![
                        text_input("Type here", &self.input).on_input(Message::Input),
                        button("Submit").on_press(Message::Button),
                        text(&self.message)
                    ]
                    .spacing(12),
                ]
                .spacing(10)
                .padding(16);
                column![header, self.row_view()].spacing(8).into()
            }
        }
    }

    fn dashboard_view(&self) -> Element<'_, Message> {
        let Some(dashboard) = &self.dashboard else {
            return column![].into();
        };
        let header = row![
            text("Iced dashboard").size(24),
            text_input("Search records", &dashboard.query).on_input(Message::DashboardSearch),
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
                    .on_press(Message::DashboardNavigate(section))
                    .width(Length::Fill)
                    .style(if selected {
                        button::primary
                    } else {
                        button::secondary
                    }),
            );
        }
        let mut thumbnails = column![].spacing(4);
        if let Some(handles) = &self.thumbnails {
            for pair in handles.chunks(2) {
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
        }
        let sidebar = scrollable(
            column![text("Sections"), sections, text("Thumbnails"), thumbnails]
                .spacing(8)
                .padding(8),
        )
        .height(Length::Fill)
        .width(Length::Fixed(SIDEBAR_WIDTH));

        let visible = dashboard.visible_records();
        let mut records = column![].spacing(2);
        for index in visible {
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
                        .on_press(Message::DashboardSelect(index))
                        .width(Length::Fill),
                    button(text(status)).on_press(Message::DashboardToggle(index)),
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
                        .on_input(move |value| Message::DashboardEdit(index, value)),
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

    fn row_view(&self) -> Element<'_, Message> {
        let count = self.scenario.row_count();
        let mut rows = column![].spacing(0);
        if self.scenario == Scenario::VirtualScroll {
            let first_visible = (self.scroll_offset / 48.0).floor().max(0.0) as usize;
            let visible_rows = (HEIGHT / benchmark_common::ROW_HEIGHT).ceil() as usize;
            let start = first_visible.saturating_sub(5).min(count);
            let end = (first_visible + visible_rows + 5).min(count);
            rows = rows.push(space().height(Length::Fixed((start * 48) as f32)));
            for index in start..end {
                rows = rows.push(row_widget(index));
            }
            let remaining = count.saturating_sub(end);
            rows = rows.push(space().height(Length::Fixed((remaining * 48) as f32)));
        } else {
            for index in 0..count {
                rows = rows.push(row_widget(index));
            }
        }
        scrollable(rows)
            .id(self.scroll_id.clone())
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
            .height(Length::Fill)
            .into()
    }

    fn switch_view(&self) -> Element<'_, Message> {
        let mut grid = column![].spacing(0);
        for row_index in 0..SWITCH_ROWS {
            let mut switch_row = row![].spacing(0);
            for column_index in 0..SWITCH_COLUMNS {
                let index = row_index * SWITCH_COLUMNS + column_index;
                let is_on = self.switches.get(index);
                switch_row = switch_row.push(iced::widget::lazy(
                    (index, is_on),
                    move |&(index, is_on)| switch_cell(index, is_on),
                ));
            }
            grid = grid.push(switch_row);
        }
        grid.into()
    }
}

fn switch_cell(index: usize, is_on: bool) -> Element<'static, Message> {
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

fn row_widget(index: usize) -> Element<'static, Message> {
    container(column![text(row_title(index)), text(row_body(index)).size(14)].spacing(2))
        .height(Length::Fixed(48.0))
        .padding([4, 12])
        .into()
}
