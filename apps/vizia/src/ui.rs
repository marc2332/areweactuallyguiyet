#[cfg(any(feature = "startup", feature = "scroll"))]
use benchmark_common::ROW_HEIGHT;
use benchmark_common::{Harness, Scenario};
use std::{cell::RefCell, rc::Rc, time::Duration};
use vizia::prelude::*;

pub trait Adapter {
    type State: Clone + 'static;
    const SCENARIO: Scenario;
    fn state() -> Self::State;
    fn build(cx: &mut Context, state: &Self::State);
    fn update(
        state: &Self::State,
        frame: benchmark_common::FrameState,
        harness: &mut Harness,
        reset: bool,
    ) -> std::io::Result<u64>;
}

pub fn run<A: Adapter>() -> Result<(), Box<dyn std::error::Error>> {
    let harness = Rc::new(RefCell::new(Some(Harness::connect(A::SCENARIO)?)));
    let failure = Rc::new(RefCell::new(None::<String>));
    let timer_harness = harness.clone();
    let timer_failure = failure.clone();
    let application = Application::new(move |cx| {
        #[cfg(not(feature = "vizia-main"))]
        cx.add_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        #[cfg(feature = "vizia-main")]
        cx.load_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        let state = A::state();
        VStack::new(cx, |cx| A::build(cx, &state))
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())]);
        let reset_generation = Rc::new(RefCell::new(0_u64));
        let ready_sent = std::cell::Cell::new(false);
        let observed = reset_generation.clone();
        let polling = cx.add_timer(Duration::from_millis(16), None, move |cx, _| {
            let result = (|| -> std::io::Result<()> {
                let mut borrowed = timer_harness.borrow_mut();
                let Some(harness) = borrowed.as_mut() else {
                    return Ok(());
                };
                if !ready_sent.get() {
                    harness.mark_ready("event-loop-after-renderer-init")?;
                    ready_sent.set(true);
                }
                let frame = harness.poll()?;
                let reset = {
                    let mut generation = observed.borrow_mut();
                    if *generation == frame.reset_generation {
                        false
                    } else {
                        *generation = frame.reset_generation;
                        true
                    }
                };
                let applied = A::update(&state, frame, harness, reset)?;
                harness.report_applied(applied)?;
                if frame.should_quit {
                    cx.emit(WindowEvent::WindowClose);
                }
                Ok(())
            })();
            if let Err(error) = result {
                *timer_failure.borrow_mut() = Some(error.to_string());
                cx.emit(WindowEvent::WindowClose);
            }
        });
        cx.start_timer(polling);
    })
    .inner_size((1280, 800));
    let event_loop_result = application.run();
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    event_loop_result?;
    Ok(())
}

#[cfg(any(feature = "startup", feature = "scroll"))]
pub fn build_rows(cx: &mut Context, count: usize) {
    for index in 0..count {
        VStack::new(cx, |cx| {
            Label::new(cx, benchmark_common::row_title(index));
            Label::new(cx, benchmark_common::row_body(index));
        })
        .height(Pixels(ROW_HEIGHT));
    }
}
