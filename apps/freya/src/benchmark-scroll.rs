use std::time::Duration;

use async_io::Timer;
use benchmark_common::Scenario;
use freya::prelude::*;

use crate::ui::{self, HarnessContext};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let context = HarnessContext::connect(Scenario::Scroll)?;
    let failure = context.failure.clone();
    ui::launch(app, context, failure)
}

fn app() -> Element {
    let context = use_hook(consume_root_context::<HarnessContext>);
    let platform = Platform::get();
    let controller = use_scroll_controller(Default::default);
    use_future(move || {
        let context = context.clone();
        let platform = platform.clone();
        let mut controller = controller;
        async move {
            let result = async {
                let mut observed_reset = 0;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let polled = context.harness.borrow_mut().poll()?;
                    if polled.should_quit {
                        break;
                    }
                    if observed_reset != polled.reset_generation {
                        controller.scroll_to_y(0);
                    }
                    if polled.running {
                        controller.scroll_to_y(-(polled.scroll_offset as i32));
                    }
                    context
                        .harness
                        .borrow_mut()
                        .report_applied(polled.update_index)?;
                    observed_reset = polled.reset_generation;
                }
                Ok::<_, std::io::Error>(())
            }
            .await;
            if let Err(error) = result {
                context.failure.borrow_mut().replace(error.to_string());
            }
            platform.exit();
        }
    });
    ScrollContent { controller }.into()
}

#[derive(PartialEq)]
struct ScrollContent {
    controller: ScrollController,
}
impl Component for ScrollContent {
    fn render(&self) -> impl IntoElement {
        rect().width(Size::fill()).height(Size::fill()).child(
            ScrollView::new_controlled(self.controller)
                .child(rect().children((0..2_000).map(ui::row))),
        )
    }
}
