use std::time::Duration;

use async_io::Timer;
use benchmark_common::Scenario;
use freya::prelude::*;

use crate::ui::{self, HarnessContext};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let context = HarnessContext::connect(Scenario::Startup)?;
    let failure = context.failure.clone();
    ui::launch(app, context, failure)
}

fn app() -> Element {
    let context = use_hook(consume_root_context::<HarnessContext>);
    let platform = Platform::get();
    use_future(move || {
        let context = context.clone();
        let platform = platform.clone();
        async move {
            let result = async {
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let polled = context.harness.borrow_mut().poll()?;
                    if polled.should_quit {
                        break;
                    }
                    context
                        .harness
                        .borrow_mut()
                        .report_applied(polled.update_index)?;
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
    StartupContent.into()
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
            .child(ScrollView::new().child(rect().children((0..20).map(ui::row))))
    }
}
