#[cfg(any(feature = "startup", feature = "scroll"))]
use benchmark_common::ROW_HEIGHT;
use benchmark_common::{Harness, Scenario};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
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

struct InitialFrameProbe {
    painted: Rc<Cell<bool>>,
}

impl View for InitialFrameProbe {
    fn draw(&self, _cx: &mut DrawContext, _canvas: &Canvas) {
        self.painted.set(true);
    }
}

pub fn run<A: Adapter>() -> Result<(), Box<dyn std::error::Error>> {
    let harness = Rc::new(RefCell::new(Some(Harness::connect(A::SCENARIO)?)));
    let failure = Rc::new(RefCell::new(None::<String>));
    let timer_harness = harness.clone();
    let timer_failure = failure.clone();
    let ready_harness = harness.clone();
    let ready_failure = failure.clone();
    let ready_sent = Cell::new(false);
    let painted = Rc::new(Cell::new(false));
    let probe_painted = painted.clone();
    let application = Application::new(move |cx| {
        #[cfg(not(feature = "vizia-main"))]
        cx.add_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        #[cfg(feature = "vizia-main")]
        cx.load_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        let state = A::state();
        InitialFrameProbe {
            painted: probe_painted,
        }
        .build(cx, |cx| {
            VStack::new(cx, |cx| A::build(cx, &state))
                .width(Stretch(1.0))
                .height(Stretch(1.0))
                .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())]);
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0));
        let reset_generation = Rc::new(RefCell::new(0_u64));
        let observed = reset_generation.clone();
        let polling = cx.add_timer(Duration::from_millis(16), None, move |cx, _| {
            let result = (|| -> std::io::Result<()> {
                let mut borrowed = timer_harness.borrow_mut();
                let Some(harness) = borrowed.as_mut() else {
                    return Ok(());
                };
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
    .inner_size((1280, 800))
    .on_idle(move |cx| {
        if ready_sent.get() || !painted.get() {
            return;
        }
        let mut borrowed = ready_harness.borrow_mut();
        let Some(harness) = borrowed.as_mut() else {
            return;
        };
        match harness.mark_ready("initial-draw-returned-following-idle-not-presented") {
            Ok(()) => ready_sent.set(true),
            Err(error) => {
                *ready_failure.borrow_mut() = Some(error.to_string());
                cx.emit(WindowEvent::WindowClose);
            }
        }
    });
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
