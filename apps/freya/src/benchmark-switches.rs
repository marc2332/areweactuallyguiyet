use std::time::Duration;

use async_io::Timer;
use benchmark_common::{Scenario, Switches};
use freya::prelude::*;

use crate::ui::{self, HarnessContext};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let harness = HarnessContext::connect(Scenario::Switches)?;
    let failure = harness.failure.clone();
    ui::launch(app, harness, failure)
}

#[derive(Clone)]
struct SwitchContext {
    harness: HarnessContext,
    cells: State<Vec<State<bool>>>,
    model: State<Switches>,
}

fn app() -> Element {
    let context = use_hook(|| {
        let context = SwitchContext {
            harness: consume_root_context::<HarnessContext>(),
            cells: State::create(
                (0..benchmark_common::SWITCH_COUNT)
                    .map(|_| State::create(false))
                    .collect(),
            ),
            model: State::create(Switches::default()),
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
                context
                    .harness
                    .harness
                    .borrow_mut()
                    .mark_ready("ui-built")?;
                let mut observed_reset = 0;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let polled = context.harness.harness.borrow_mut().poll()?;
                    if polled.should_quit {
                        break;
                    }
                    let mut model = context.model.write();
                    let cells = context.cells.peek();
                    if observed_reset != polled.reset_generation {
                        for mut cell in cells.iter().copied() {
                            cell.set_if_modified(false);
                        }
                        *model = Switches::default();
                    }
                    if model.applied_updates() != polled.update_index {
                        model.advance_changed(
                            polled.update_index,
                            context.harness.harness.borrow().switch_mode(),
                            |index, value| {
                                let mut cell = cells[index];
                                cell.set(value);
                            },
                        );
                    }
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
    SwitchContent.into()
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

#[cfg(test)]
mod tests {
    use crate::{selected_switches::app, ui::HarnessContext};
    use benchmark_common::{SWITCH_COUNT, Scenario};
    use freya::prelude::*;
    use freya_testing::TestingRunner;

    #[test]
    fn mounts_switch_state_inside_runtime_and_toggles_cell() {
        let context = HarnessContext::connect(Scenario::Switches).unwrap();
        let (mut runner, _) = TestingRunner::new(
            app,
            (1280., 800.).into(),
            move |runner| runner.provide_root_context(|| context),
            1.,
        );
        runner.sync_and_update();
        let labels = runner.find_many(|_, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "OFF")
                .map(|_| ())
        });
        assert_eq!(labels.len(), SWITCH_COUNT);
        runner.click_cursor((15., 15.));
        runner.sync_and_update();
        let enabled = runner.find_many(|_, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "ON")
                .map(|_| ())
        });
        assert_eq!(enabled.len(), 1);
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
