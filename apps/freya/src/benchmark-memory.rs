use std::time::Duration;

use async_io::Timer;
use benchmark_common::Scenario;
use freya::prelude::*;

use crate::ui::{self, HarnessContext};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let harness = HarnessContext::connect(Scenario::Memory)?;
    let failure = harness.failure.clone();
    ui::launch(app, harness, failure)
}

#[derive(Clone)]
struct DashboardContext {
    harness: HarnessContext,
    model: State<benchmark_common::dashboard::Dashboard>,
    inputs: State<Vec<State<String>>>,
    query: State<String>,
}

fn app() -> Element {
    let context = use_hook(|| {
        let context = DashboardContext {
            harness: consume_root_context::<HarnessContext>(),
            model: State::create(benchmark_common::dashboard::Dashboard::default()),
            inputs: State::create(
                (0..benchmark_common::dashboard::FIELD_COUNT)
                    .map(|index| State::create(format!("Initial value {index:02}")))
                    .collect(),
            ),
            query: State::create(String::new()),
        };
        provide_root_context(context.clone());
        context
    });
    let platform = Platform::get();
    use_future(move || {
        let mut context = context.clone();
        let platform = platform.clone();
        async move {
            let result = async {
                let mut observed_reset = 0;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let polled = context.harness.harness.borrow_mut().poll()?;
                    if polled.should_quit {
                        break;
                    }
                    let reset = observed_reset != polled.reset_generation;
                    if reset {
                        context.model.write().reset();
                    }
                    if polled.running {
                        context.model.write().advance(polled);
                    }
                    let model = context.model.peek();
                    for (index, mut input) in context.inputs.peek().iter().copied().enumerate() {
                        input.set_if_modified(model.fields[index].clone());
                    }
                    context.query.set_if_modified(model.query.clone());
                    context
                        .harness
                        .harness
                        .borrow_mut()
                        .report_dashboard(&model)?;
                    context
                        .harness
                        .harness
                        .borrow_mut()
                        .report_applied(model.applied_updates())?;
                    observed_reset = polled.reset_generation;
                }
                Ok::<_, std::io::Error>(())
            }
            .await;
            if let Err(error) = result {
                context
                    .harness
                    .failure
                    .borrow_mut()
                    .replace(error.to_string());
            }
            platform.exit();
        }
    });
    DashboardView.into()
}

#[cfg(test)]
mod tests {
    use crate::{selected_memory::app, ui::HarnessContext};
    use benchmark_common::{Scenario, dashboard::RECORD_COUNT};
    use freya::prelude::*;
    use freya_testing::TestingRunner;

    #[test]
    fn mounts_dashboard_state_inside_runtime() {
        let context = HarnessContext::connect(Scenario::Memory).unwrap();
        let (mut runner, _) = TestingRunner::new(
            app,
            (1280., 800.).into(),
            move |runner| runner.provide_root_context(|| context),
            1.,
        );
        runner.sync_and_update();
        let toggles = runner.find_many(|_, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "OFF")
                .map(|_| ())
        });
        assert_eq!(toggles.len(), RECORD_COUNT);
        assert!(
            runner
                .find(|_, element| {
                    Label::try_downcast(element)
                        .filter(|label| label.text == "Field 19")
                        .map(|_| ())
                })
                .is_some()
        );
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
