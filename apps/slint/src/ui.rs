use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
    time::Duration,
};

use benchmark_common::{FrameState, Harness, Scenario};
use slint::ComponentHandle;

pub fn run<W, F>(
    scenario: Scenario,
    window: W,
    mut update: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    W: ComponentHandle + 'static,
    F: FnMut(&W, FrameState, &mut Harness, bool) -> io::Result<u64> + 'static,
{
    let harness = Rc::new(RefCell::new(Harness::connect(scenario)?));
    let failure = Rc::new(RefCell::new(None::<String>));
    let ready = Rc::new(Cell::new(false));
    let ready_harness = Rc::clone(&harness);
    let ready_failure = Rc::clone(&failure);
    window.window().set_rendering_notifier(move |state, _| {
        if matches!(state, slint::RenderingState::AfterRendering) && !ready.replace(true) {
            if let Err(error) = ready_harness
                .borrow_mut()
                .mark_ready("initial-render-returned-not-presented")
            {
                *ready_failure.borrow_mut() = Some(error.to_string());
                let _ = slint::quit_event_loop();
            }
        }
    })?;
    let timer = slint::Timer::default();
    let weak = window.as_weak();
    let timer_failure = Rc::clone(&failure);
    let observed_reset = Cell::new(0);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(window) = weak.upgrade() else { return };
            let result = (|| -> io::Result<()> {
                let mut harness = harness.borrow_mut();
                let frame = harness.poll()?;
                if frame.should_quit {
                    slint::quit_event_loop().map_err(io::Error::other)?;
                    return Ok(());
                }
                let reset =
                    observed_reset.replace(frame.reset_generation) != frame.reset_generation;
                let applied = update(&window, frame, &mut harness, reset)?;
                harness.report_applied(applied)?;
                Ok(())
            })();
            if let Err(error) = result {
                *timer_failure.borrow_mut() = Some(error.to_string());
                let _ = slint::quit_event_loop();
            }
        },
    );
    window.run()?;
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}
