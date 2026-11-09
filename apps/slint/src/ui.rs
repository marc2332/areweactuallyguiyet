use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
    time::Duration,
};

#[cfg(not(feature = "hover"))]
use benchmark_common::Scenario;
use benchmark_common::{FrameState, Harness};
use slint::ComponentHandle;

#[cfg(not(feature = "hover"))]
pub fn run<W, F>(scenario: Scenario, window: W, update: F) -> Result<(), Box<dyn std::error::Error>>
where
    W: ComponentHandle + 'static,
    F: FnMut(&W, FrameState, &mut Harness, bool) -> io::Result<u64> + 'static,
{
    run_with_harness(window, Harness::connect(scenario)?, update)
}

pub fn run_with_harness<W, F>(
    window: W,
    harness: Harness,
    mut update: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    W: ComponentHandle + 'static,
    F: FnMut(&W, FrameState, &mut Harness, bool) -> io::Result<u64> + 'static,
{
    let harness = Rc::new(RefCell::new(harness));
    let failure = Rc::new(RefCell::new(None::<String>));
    let ready = Rc::new(Cell::new(false));
    let ready_harness = Rc::clone(&harness);
    let ready_failure = Rc::clone(&failure);
    #[cfg(feature = "hover")]
    let ready_window = window.as_weak();
    window.window().set_rendering_notifier(move |state, _| {
        if matches!(state, slint::RenderingState::AfterRendering) && !ready.get() {
            let result = (|| -> io::Result<()> {
                #[cfg(feature = "hover")]
                {
                    use slint::winit_030::{
                        WinitWindowAccessor,
                        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
                    };
                    let window = ready_window
                        .upgrade()
                        .ok_or_else(|| io::Error::other("Slint window destroyed"))?;
                    let fullscreen = window
                        .window()
                        .with_winit_window(|native| {
                            let size = native.inner_size();
                            native.fullscreen().is_some()
                                && size.width > 0
                                && size.height > 0
                                && size.width == window.window().size().width
                                && size.height == window.window().size().height
                                && native.scale_factor()
                                    == f64::from(window.window().scale_factor())
                                && native
                                    .current_monitor()
                                    .is_some_and(|monitor| size == monitor.size())
                        })
                        .unwrap_or(false);
                    if !fullscreen {
                        return Ok(());
                    }
                    ready.set(true);
                    let backend = window
                        .window()
                        .with_winit_window(|native| {
                            match native.window_handle().ok().map(|handle| handle.as_raw()) {
                                Some(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "x11",
                                Some(RawWindowHandle::Wayland(_)) => "wayland",
                                _ => "unspecified",
                            }
                        })
                        .unwrap_or("unspecified");
                    let scale = f64::from(window.window().scale_factor());
                    return ready_harness.borrow_mut().mark_hover_ready(
                        "initial-render-returned-not-presented",
                        backend,
                        scale,
                        (f64::from(window.window().size().width) / scale) as f32,
                        (f64::from(window.window().size().height) / scale) as f32,
                    );
                }
                #[cfg(not(feature = "hover"))]
                ready.set(true);
                #[cfg(not(feature = "hover"))]
                ready_harness
                    .borrow_mut()
                    .mark_ready("initial-render-returned-not-presented")
            })();
            if let Err(error) = result {
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
