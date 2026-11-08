use std::{cell::RefCell, rc::Rc, time::Duration};

use async_io::Timer;
use benchmark_common::{Harness, Scenario, Switches, row_body, row_title};
use freya::prelude::*;

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    let harness = Rc::new(RefCell::new(Harness::connect(scenario)?));
    let failure = Rc::new(RefCell::new(None::<String>));
    let context = BenchmarkContext {
        scenario,
        harness,
        failure: failure.clone(),
    };
    launch(
        LaunchConfig::new()
            .with_default_font("DejaVu Sans")
            .with_font(
                "DejaVu Sans",
                Bytes::from_static(include_bytes!("../../../assets/fonts/DejaVuSans.ttf")),
            )
            .with_window(WindowConfig::new(app).with_root_context(context).with_size(
                f64::from(benchmark_common::WIDTH),
                f64::from(benchmark_common::HEIGHT),
            )),
    );
    match failure.borrow_mut().take() {
        Some(message) => Err(message.into()),
        None => Ok(()),
    }
}

#[derive(Clone)]
struct BenchmarkContext {
    scenario: Scenario,
    harness: Rc<RefCell<Harness>>,
    failure: Rc<RefCell<Option<String>>>,
}

#[derive(Clone, Copy)]
struct SwitchContext {
    cells: State<Vec<State<bool>>>,
    model: State<Switches>,
}

impl SwitchContext {
    fn new() -> Self {
        Self {
            cells: State::create(
                (0..benchmark_common::SWITCH_COUNT)
                    .map(|_| State::create(false))
                    .collect(),
            ),
            model: State::create(Switches::default()),
        }
    }
}

#[derive(Clone, Copy)]
struct DashboardContext {
    model: State<benchmark_common::dashboard::Dashboard>,
    inputs: State<Vec<State<String>>>,
    query: State<String>,
}

impl DashboardContext {
    fn new() -> Self {
        Self {
            model: State::create(benchmark_common::dashboard::Dashboard::default()),
            inputs: State::create(
                (0..benchmark_common::dashboard::FIELD_COUNT)
                    .map(|index| State::create(format!("Initial value {index:02}")))
                    .collect(),
            ),
            query: State::create(String::new()),
        }
    }
}

fn app() -> Element {
    let context = use_hook(consume_root_context::<BenchmarkContext>);
    let scenario = context.scenario;
    let platform = Platform::get();
    let scroll_controller = use_scroll_controller(Default::default);
    let switches = use_hook(move || {
        (scenario == Scenario::Switches).then(|| {
            let context = SwitchContext::new();
            provide_root_context(context);
            context
        })
    });
    let dashboard = use_hook(move || {
        (scenario == Scenario::Memory).then(|| {
            let context = DashboardContext::new();
            provide_root_context(context);
            context
        })
    });
    use_future(move || {
        let context = context.clone();
        let platform = platform.clone();
        async move {
            if let Err(error) = context.poll(scroll_controller, switches, dashboard).await {
                context.failure.borrow_mut().replace(error.to_string());
            }
            platform.exit();
        }
    });
    if scenario == Scenario::Memory {
        return DashboardView.into();
    }
    if scenario == Scenario::Switches {
        return SwitchContent.into();
    }
    match scenario {
        Scenario::Scroll => ScrollContent {
            controller: scroll_controller,
        }
        .into(),
        Scenario::VirtualScroll => VirtualContent {
            controller: scroll_controller,
        }
        .into(),
        _ => StartupContent.into(),
    }
}

impl BenchmarkContext {
    async fn poll(
        &self,
        mut scroll_controller: ScrollController,
        mut switches: Option<SwitchContext>,
        mut dashboard: Option<DashboardContext>,
    ) -> std::io::Result<()> {
        self.harness.borrow_mut().mark_ready("ui-built")?;
        let mut observed_reset = 0;
        loop {
            Timer::after(Duration::from_millis(16)).await;
            let polled = self.harness.borrow_mut().poll()?;
            if polled.should_quit {
                break;
            }
            let reset = observed_reset != polled.reset_generation;
            if matches!(self.scenario, Scenario::Scroll | Scenario::VirtualScroll) {
                if reset {
                    scroll_controller.scroll_to_y(0);
                }
                if polled.running {
                    scroll_controller.scroll_to_y(-(polled.scroll_offset as i32));
                }
            }
            if let Some(dashboard) = dashboard.as_mut() {
                if reset {
                    dashboard.model.write().reset();
                }
                if polled.running {
                    dashboard.model.write().advance(polled);
                }
                let model = dashboard.model.peek();
                for (index, mut input) in dashboard.inputs.peek().iter().copied().enumerate() {
                    input.set_if_modified(model.fields[index].clone());
                }
                dashboard.query.set_if_modified(model.query.clone());
                self.harness.borrow_mut().report_dashboard(&model)?;
                self.harness
                    .borrow_mut()
                    .report_applied(model.applied_updates())?;
            } else if let Some(switches) = switches.as_mut() {
                let mut model = switches.model.write();
                let cells = switches.cells.peek();
                if reset {
                    for mut cell in cells.iter().copied() {
                        cell.set_if_modified(false);
                    }
                    *model = Switches::default();
                }
                if model.applied_updates() != polled.update_index {
                    model.advance_changed(
                        polled.update_index,
                        self.harness.borrow().switch_mode(),
                        |index, value| {
                            let mut cell = cells[index];
                            cell.set(value);
                        },
                    );
                }
                self.harness
                    .borrow_mut()
                    .report_applied(model.applied_updates())?;
            } else {
                self.harness
                    .borrow_mut()
                    .report_applied(polled.update_index)?;
            }
            observed_reset = polled.reset_generation;
        }
        Ok(())
    }
}

#[derive(PartialEq)]
struct ScrollContent {
    controller: ScrollController,
}
impl Component for ScrollContent {
    fn render(&self) -> impl IntoElement {
        rect().width(Size::fill()).height(Size::fill()).child(
            ScrollView::new_controlled(self.controller).child(rect().children((0..2_000).map(row))),
        )
    }
}

#[derive(PartialEq)]
struct VirtualContent {
    controller: ScrollController,
}
impl Component for VirtualContent {
    fn render(&self) -> impl IntoElement {
        VirtualScrollView::new_controlled(|item, _| row(item.index), self.controller)
            .length(100_000usize)
            .item_size(benchmark_common::ROW_HEIGHT)
    }
}

fn row(index: usize) -> Element {
    rect()
        .key(index)
        .height(Size::px(benchmark_common::ROW_HEIGHT))
        .padding(4.)
        .child(label().text(row_title(index)))
        .child(label().text(row_body(index)))
        .into()
}

#[derive(PartialEq)]
struct SwitchContent;
impl Component for SwitchContent {
    fn render(&self) -> impl IntoElement {
        let context = use_hook(consume_root_context::<SwitchContext>);
        let cells = context.cells.peek();
        rect().width(Size::fill()).height(Size::fill()).children(
            (0..benchmark_common::SWITCH_ROWS).map(|row| {
                rect().key(row).horizontal().height(Size::px(30.)).children(
                    (0..benchmark_common::SWITCH_COLUMNS).map(|column| {
                        let index = row * benchmark_common::SWITCH_COLUMNS + column;
                        rect()
                            .key(index)
                            .width(Size::px(30.))
                            .height(Size::px(30.))
                            .child(SwitchCell {
                                index,
                                enabled: cells[index],
                            })
                    }),
                )
            }),
        )
    }
}

#[derive(PartialEq)]
struct SwitchCell {
    index: usize,
    enabled: State<bool>,
}
impl Component for SwitchCell {
    fn render(&self) -> impl IntoElement {
        let index = self.index;
        let mut enabled = self.enabled;
        let value = *enabled.read();
        let SwitchContext { mut model, .. } = use_hook(consume_root_context::<SwitchContext>);
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(if value {
                benchmark_common::SWITCH_ON_COLOR
            } else {
                benchmark_common::SWITCH_OFF_COLOR
            })
            .cross_align(Alignment::Center)
            .main_align(Alignment::Center)
            .on_press(move |_| {
                enabled.toggle();
                model.write().toggle(index);
            })
            .child(
                label()
                    .font_family("DejaVu Sans")
                    .font_size(benchmark_common::SWITCH_FONT_SIZE)
                    .color(benchmark_common::SWITCH_TEXT_COLOR)
                    .text(if value { "ON" } else { "OFF" }),
            )
    }
}

#[derive(PartialEq)]
struct StartupContent;
impl Component for StartupContent {
    fn render(&self) -> impl IntoElement {
        let text = use_state(String::new);
        rect()
            .padding(16.)
            .child("Benchmark startup")
            .child("Prepared initial content")
            .child(Input::new(text))
            .child(Button::new().child("Continue"))
            .child(ScrollView::new().child(rect().children((0..20).map(row))))
    }
}

#[derive(PartialEq)]
struct DashboardView;
impl Component for DashboardView {
    fn render(&self) -> impl IntoElement {
        let DashboardContext {
            model: dashboard,
            mut query,
            ..
        } = use_hook(consume_root_context::<DashboardContext>);
        let mut search_model = dashboard;
        let search = WritableSetter::new(query, move |value: String| {
            query.set(value.clone());
            search_model.write().search(value);
        });
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .font_family("DejaVu Sans")
            .child(
                rect()
                    .height(Size::px(benchmark_common::dashboard::HEADER_HEIGHT))
                    .horizontal()
                    .padding(8.)
                    .child("Dashboard")
                    .child(
                        Input::new(search)
                            .placeholder("Search records")
                            .width(Size::fill()),
                    ),
            )
            .child(
                rect()
                    .height(Size::fill())
                    .horizontal()
                    .child(Sidebar)
                    .child(RecordList)
                    .child(DetailPane),
            )
    }
}

#[derive(PartialEq)]
struct Sidebar;
impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let DashboardContext {
            model: dashboard, ..
        } = use_hook(consume_root_context::<DashboardContext>);
        rect()
            .width(Size::px(benchmark_common::dashboard::SIDEBAR_WIDTH))
            .height(Size::fill())
            .child(
                ScrollView::new()
                    .height(Size::px(440.))
                    .child(
                        rect().children((0..benchmark_common::dashboard::SECTION_COUNT).map(
                            |section| {
                                let mut model = dashboard;
                                let active = dashboard.read().section == section;
                                rect()
                                    .key(section)
                                    .height(Size::px(22.))
                                    .on_press(move |_| model.write().navigate(section))
                                    .background(if active {
                                        (220, 230, 245)
                                    } else {
                                        (255, 255, 255)
                                    })
                                    .child(format!("Section {section}"))
                            },
                        )),
                    ),
            )
            .child(rect().children((0..5).map(|row| {
                rect().key(row).horizontal().children((0..2).map(|column| {
                    let index = row * 2 + column;
                    let source: ImageSource = (
                        format!("dashboard-{index}"),
                        benchmark_common::dashboard::THUMBNAILS[index],
                    )
                        .into();
                    ImageViewer::new(source)
                        .width(Size::px(48.))
                        .height(Size::px(48.))
                }))
            })))
    }
}

#[derive(PartialEq)]
struct RecordList;
impl Component for RecordList {
    fn render(&self) -> impl IntoElement {
        let DashboardContext {
            model: dashboard, ..
        } = use_hook(consume_root_context::<DashboardContext>);
        let mut controller = use_scroll_controller(Default::default);
        let mut previous_offset = use_state(|| 0.0);
        let (visible, fixtures, selected, scroll_offset) = {
            let model = dashboard.read();
            (
                model.visible_records(),
                model.fixtures.clone(),
                model.selected_record,
                model.scroll_offset,
            )
        };
        if *previous_offset.peek() != scroll_offset {
            controller.scroll_to_y(-(scroll_offset as i32));
            previous_offset.set(scroll_offset);
        }
        rect()
            .width(Size::px(benchmark_common::dashboard::RECORDS_WIDTH))
            .height(Size::fill())
            .child(
                ScrollView::new_controlled(controller).child(rect().children(
                    visible.into_iter().map(|index| {
                        let mut dashboard = dashboard;
                        let record = &fixtures.records[index];
                        rect()
                            .key(index)
                            .height(Size::px(benchmark_common::dashboard::RECORD_HEIGHT))
                            .horizontal()
                            .background(if index == selected {
                                (225, 235, 250)
                            } else {
                                (255, 255, 255)
                            })
                            .on_press(move |_| dashboard.write().select(index))
                            .child(
                                rect()
                                    .width(Size::px(
                                        benchmark_common::dashboard::RECORDS_WIDTH - 48.,
                                    ))
                                    .child(
                                        label()
                                            .width(Size::fill())
                                            .font_family("DejaVu Sans")
                                            .font_size(14.)
                                            .max_lines(1)
                                            .text(record.title.clone()),
                                    )
                                    .child(
                                        label()
                                            .width(Size::fill())
                                            .font_family("DejaVu Sans")
                                            .font_size(12.)
                                            .max_lines(2)
                                            .text(record.body.clone()),
                                    ),
                            )
                            .child(RowToggle { index })
                    }),
                )),
            )
    }
}

#[derive(PartialEq)]
struct RowToggle {
    index: usize,
}
impl Component for RowToggle {
    fn render(&self) -> impl IntoElement {
        let DashboardContext {
            model: mut dashboard,
            ..
        } = use_hook(consume_root_context::<DashboardContext>);
        let index = self.index;
        let enabled = dashboard.read().active.get(index);
        rect()
            .width(Size::px(48.))
            .height(Size::px(28.))
            .background(if enabled {
                (47, 158, 68)
            } else {
                (134, 142, 150)
            })
            .font_family("DejaVu Sans")
            .font_size(12.)
            .color((255, 255, 255))
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .on_press(move |event: Event<PressEventData>| {
                event.stop_propagation();
                dashboard.write().toggle(index);
            })
            .child(if enabled { "ON" } else { "OFF" })
    }
}

#[derive(PartialEq)]
struct DetailPane;
impl Component for DetailPane {
    fn render(&self) -> impl IntoElement {
        let DashboardContext {
            model: dashboard,
            inputs,
            ..
        } = use_hook(consume_root_context::<DashboardContext>);
        let detail = dashboard.read().detail.to_string();
        rect()
            .expanded()
            .height(Size::fill())
            .child(
                ScrollView::new()
                    .height(Size::px(300.))
                    .child(paragraph().font_family("DejaVu Sans").span(detail)),
            )
            .child(
                ScrollView::new().height(Size::fill()).child(
                    rect().children(inputs.peek().iter().copied().enumerate().map(
                        |(index, value)| {
                            let mut model_value = value;
                            let mut dashboard = dashboard;
                            let setter = WritableSetter::new(value, move |text: String| {
                                model_value.set(text.clone());
                                dashboard.write().edit(index, text);
                            });
                            rect()
                                .key(index)
                                .height(Size::px(42.))
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .child(
                                    label()
                                        .width(Size::px(100.))
                                        .text(format!("Field {index:02}")),
                                )
                                .child(Input::new(setter).width(Size::fill()))
                        },
                    )),
                ),
            )
    }
}
