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
    #[cfg(not(feature = "hover"))]
    fn build(cx: &mut Context, state: &Self::State);
    #[cfg(feature = "hover")]
    fn build(cx: &mut Context, state: &Self::State, observer: benchmark_common::HoverObserver);
    #[cfg(not(feature = "hover"))]
    fn update(
        state: &Self::State,
        frame: benchmark_common::FrameState,
        harness: &mut Harness,
        reset: bool,
    ) -> std::io::Result<u64>;
}

struct InitialFrameProbe {
    painted: Rc<Cell<bool>>,
    #[cfg(feature = "hover")]
    viewport: Rc<Cell<Option<(f32, f32)>>>,
}

impl View for InitialFrameProbe {
    fn draw(&self, _cx: &mut DrawContext, _canvas: &Canvas) {
        #[cfg(feature = "hover")]
        {
            let bounds = _cx.with_current(Entity::root(), |context| context.bounds());
            self.viewport.set(Some((bounds.w, bounds.h)));
        }
        self.painted.set(true);
    }
}

pub fn run<A: Adapter>() -> Result<(), Box<dyn std::error::Error>> {
    let harness = Harness::connect(A::SCENARIO)?;
    #[cfg(feature = "hover")]
    let observer = harness.hover_observer();
    let harness = Rc::new(RefCell::new(Some(harness)));
    let failure = Rc::new(RefCell::new(None::<String>));
    let timer_harness = harness.clone();
    let timer_failure = failure.clone();
    let ready_harness = harness.clone();
    let ready_failure = failure.clone();
    let ready_sent = Cell::new(false);
    #[cfg(feature = "hover")]
    let fullscreen_requested = Cell::new(false);
    let painted = Rc::new(Cell::new(false));
    let probe_painted = painted.clone();
    #[cfg(feature = "hover")]
    let viewport = Rc::new(Cell::new(None));
    #[cfg(feature = "hover")]
    let probe_viewport = viewport.clone();
    let application = Application::new(move |cx| {
        #[cfg(not(feature = "vizia-main"))]
        cx.add_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        #[cfg(feature = "vizia-main")]
        cx.load_font_mem(include_bytes!("../../../assets/fonts/DejaVuSans.ttf"));
        let state = A::state();
        InitialFrameProbe {
            painted: probe_painted,
            #[cfg(feature = "hover")]
            viewport: probe_viewport,
        }
        .build(cx, |cx| {
            let content = VStack::new(cx, |cx| {
                #[cfg(not(feature = "hover"))]
                A::build(cx, &state);
                #[cfg(feature = "hover")]
                A::build(cx, &state, observer);
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())]);
            #[cfg(feature = "hover")]
            content.background_color(Color::rgb(255, 255, 255));
            #[cfg(not(feature = "hover"))]
            let _ = content;
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0));
        #[cfg(not(feature = "hover"))]
        let reset_generation = Rc::new(RefCell::new(0_u64));
        #[cfg(not(feature = "hover"))]
        let observed = reset_generation.clone();
        let polling = cx.add_timer(Duration::from_millis(16), None, move |cx, _| {
            let result = (|| -> std::io::Result<()> {
                let mut borrowed = timer_harness.borrow_mut();
                let Some(harness) = borrowed.as_mut() else {
                    return Ok(());
                };
                let frame = harness.poll()?;
                #[cfg(not(feature = "hover"))]
                let reset = {
                    let mut generation = observed.borrow_mut();
                    if *generation == frame.reset_generation {
                        false
                    } else {
                        *generation = frame.reset_generation;
                        true
                    }
                };
                #[cfg(not(feature = "hover"))]
                let applied = A::update(&state, frame, harness, reset)?;
                #[cfg(not(feature = "hover"))]
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
    .decorations(!cfg!(feature = "hover"))
    .on_idle(move |cx| {
        #[cfg(feature = "hover")]
        if !fullscreen_requested.get() {
            let Some(window) = EventContext::new(cx).window() else {
                return;
            };
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            fullscreen_requested.set(true);
            return;
        }
        if ready_sent.get() || !painted.get() {
            return;
        }
        #[cfg(feature = "hover")]
        let Some(window) = EventContext::new(cx).window() else {
            return;
        };
        #[cfg(feature = "hover")]
        {
            let Some(monitor) = window.current_monitor() else {
                return;
            };
            let size = window.inner_size();
            if window.fullscreen().is_none()
                || size != monitor.size()
                || size.width == 0
                || size.height == 0
            {
                return;
            }
            if viewport.get() != Some((size.width as f32, size.height as f32)) {
                return;
            }
        }
        let mut borrowed = ready_harness.borrow_mut();
        let Some(harness) = borrowed.as_mut() else {
            return;
        };
        let boundary = "initial-draw-returned-following-idle-not-presented";
        #[cfg(not(feature = "hover"))]
        let result = harness.mark_ready(boundary);
        #[cfg(feature = "hover")]
        let result = viewport
            .get()
            .ok_or_else(|| std::io::Error::other("Vizia actual root layout unavailable"))
            .and_then(|(width, height)| {
                let scale = window.scale_factor();
                harness.mark_hover_ready(
                    boundary,
                    "unspecified",
                    scale,
                    (f64::from(width) / scale) as f32,
                    (f64::from(height) / scale) as f32,
                )
            });
        match result {
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
