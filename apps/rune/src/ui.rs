use benchmark_common::{FrameState, HEIGHT, Harness, Scenario, WIDTH};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{ROW_HEIGHT, row_body, row_title};
use rune::{App, AppSpec, Color, Cx, El, Tx, col};
#[cfg(any(
    feature = "startup",
    feature = "scroll",
    feature = "virtual-scroll",
    feature = "memory"
))]
use rune::{TextStyle, text};
use std::{cell::RefCell, rc::Rc, time::Duration};

pub type Error = Box<dyn std::error::Error>;

pub trait ScenarioUi: Sized + 'static {
    const SCENARIO: Scenario;

    fn new() -> Result<Self, Error>;
    fn reset(&mut self, transaction: &mut Tx);
    fn update(
        &mut self,
        frame: FrameState,
        harness: &Harness,
        transaction: &mut Tx,
        updated: bool,
    ) -> bool;
    fn applied_updates(&self, frame: FrameState) -> u64 {
        frame.update_index
    }
    fn report(&self, _harness: &mut Harness) -> Result<(), Error> {
        Ok(())
    }
    fn view(state: &Benchmark<Self>) -> El;
}

pub struct Benchmark<S: ScenarioUi> {
    pub harness: Harness,
    pub frame: FrameState,
    pub content: S,
    initialized: bool,
    #[cfg(feature = "hover")]
    pub hover_observer: benchmark_common::HoverObserver,
    #[cfg(feature = "hover")]
    native_scale: std::cell::Cell<Option<f64>>,
    #[cfg(feature = "hover")]
    native_viewport: Option<(f32, f32)>,
    #[cfg(feature = "hover")]
    expected_viewport: Option<(f32, f32)>,
    #[cfg(feature = "hover")]
    fullscreen_requested: bool,
    #[cfg(feature = "hover")]
    native_backend: Option<&'static str>,
    failure: Rc<RefCell<Option<String>>>,
}

impl<S: ScenarioUi> Benchmark<S> {
    fn new() -> Result<Self, Error> {
        let harness = Harness::connect(S::SCENARIO)?;
        Ok(Self {
            #[cfg(feature = "hover")]
            hover_observer: harness.hover_observer(),
            #[cfg(feature = "hover")]
            native_scale: std::cell::Cell::new(None),
            #[cfg(feature = "hover")]
            native_viewport: None,
            #[cfg(feature = "hover")]
            expected_viewport: expected_hover_viewport(
                harness.controlled(),
                std::env::var("GUI_BENCH_HOVER_WIDTH").ok().as_deref(),
                std::env::var("GUI_BENCH_HOVER_HEIGHT").ok().as_deref(),
            )?,
            #[cfg(feature = "hover")]
            fullscreen_requested: false,
            #[cfg(feature = "hover")]
            native_backend: None,
            harness,
            frame: FrameState::default(),
            content: S::new()?,
            initialized: false,
            failure: Rc::default(),
        })
    }

    #[cfg(feature = "hover")]
    fn try_hover_ready(&mut self) -> std::io::Result<bool> {
        let (Some(backend), Some(scale), Some((width, height))) = (
            self.native_backend,
            self.native_scale.get(),
            self.native_viewport,
        ) else {
            return Ok(false);
        };
        if !self.fullscreen_requested
            || self
                .expected_viewport
                .is_some_and(|size| size != (width, height))
        {
            return Ok(false);
        }
        self.harness.mark_hover_ready(
            "post-fullscreen-request-drawn-frame-returned-and-root-layout-observed-no-display-confirmation",
            backend,
            scale,
            width,
            height,
        )?;
        Ok(true)
    }

    fn advance(&mut self, transaction: &mut Tx) -> Result<bool, Error> {
        if self.harness.controlled() {
            transaction.wake_after(Duration::from_millis(16));
        }
        let frame = self.harness.poll()?;
        if frame.should_quit {
            transaction.quit();
            return Ok(false);
        }
        let reset = !self.initialized || frame.reset_generation != self.frame.reset_generation;
        let updated = frame.update_index != self.frame.update_index;
        transaction.instant();
        if reset {
            self.initialized = true;
            self.content.reset(transaction);
        }
        let mut changed = reset;
        if frame.running {
            changed |= self
                .content
                .update(frame, &self.harness, transaction, updated);
        }
        self.frame = frame;
        self.harness
            .report_applied(self.content.applied_updates(frame))?;
        self.content.report(&mut self.harness)?;
        Ok(changed)
    }
}

#[cfg(any(
    feature = "startup",
    feature = "scroll",
    feature = "virtual-scroll",
    feature = "memory"
))]
pub fn label(value: impl Into<String>, size: f32) -> El {
    text(value)
        .font(TextStyle::new(size, 400.0))
        .color(Color::BLACK)
}

#[cfg(any(feature = "switches", feature = "memory", feature = "hover"))]
pub fn color((red, green, blue): (u8, u8, u8)) -> Color {
    Color {
        r: rune::color::srgb_decode(f32::from(red) / 255.0),
        g: rune::color::srgb_decode(f32::from(green) / 255.0),
        b: rune::color::srgb_decode(f32::from(blue) / 255.0),
        a: 1.0,
    }
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn record(index: usize) -> El {
    col()
        .key(format!("record-{index}"))
        .w_full()
        .h(ROW_HEIGHT)
        .child(label(row_title(index), 14.0))
        .child(label(row_body(index), 14.0))
}

fn view<S: ScenarioUi>(state: &Benchmark<S>, context: &mut Cx<Benchmark<S>>) -> El {
    #[cfg(feature = "hover")]
    state.native_scale.set(Some(f64::from(context.env().scale)));
    let _ = context;
    let root = col()
        .w_full()
        .h_full()
        .fill(Color::WHITE)
        .clip_rounded(0.0)
        .instant_changes()
        .child(S::view(state));
    #[cfg(feature = "hover")]
    return root.on_layout_tx(
        |state: &mut Benchmark<S>, [_, _, width, height], transaction| {
            state.native_viewport = Some((width, height));
            if !state.fullscreen_requested {
                state.fullscreen_requested = true;
                transaction.toggle_full_screen();
            }
            if let Err(error) = state.try_hover_ready() {
                *state.failure.borrow_mut() = Some(error.to_string());
                transaction.quit();
            }
            transaction.live_only();
        },
    );
    #[cfg(not(feature = "hover"))]
    root
}

fn poll<S: ScenarioUi>(state: &mut Benchmark<S>, _: f64, transaction: &mut Tx) -> bool {
    match state.advance(transaction) {
        Ok(changed) => changed,
        Err(error) => {
            *state.failure.borrow_mut() = Some(error.to_string());
            transaction.quit();
            false
        }
    }
}

fn native_initializer<S: ScenarioUi>() -> Benchmark<S> {
    eprintln!("Rune benchmark requires its native initializer");
    std::process::exit(1);
}

pub fn run<S: ScenarioUi>() -> Result<(), Error> {
    let state = Benchmark::<S>::new()?;
    let failure = state.failure.clone();
    let spec = AppSpec {
        fonts: &[include_bytes!("../../../assets/fonts/DejaVuSans.ttf")],
        families: ("DejaVu Sans", "DejaVu Sans"),
        poll: Some(poll::<S>),
        ..AppSpec::new(
            "GUI benchmark: rune",
            (WIDTH, HEIGHT),
            native_initializer::<S>,
            view::<S>,
        )
    };
    let app = App::new(spec).args(Vec::new()).init(move || state);
    #[cfg(feature = "hover")]
    let native_window = Rc::new(RefCell::new(None::<rune::WindowHandle>));
    #[cfg(feature = "hover")]
    let app = {
        let native_window = native_window.clone();
        app.on_window(move |window| *native_window.borrow_mut() = Some(window.clone()))
    };
    let handle = app.handle();
    let mut ready_sent = false;
    #[cfg(feature = "hover")]
    let mut initial_drawn = false;
    app.on_frame_encoded(move |frame| {
        if ready_sent || frame.outcome() != rune::FrameOutcome::Drawn {
            return;
        }
        #[cfg(feature = "hover")]
        if !initial_drawn {
            initial_drawn = true;
            handle.update(|_, transaction| transaction.live_only());
            return;
        }
        ready_sent = true;
        #[cfg(feature = "hover")]
        let backend = native_window
            .borrow()
            .as_ref()
            .and_then(|window| {
                window
                    .with_raw_handle(|handle| match handle.as_raw() {
                        rune::raw_window_handle::RawWindowHandle::Xlib(_)
                        | rune::raw_window_handle::RawWindowHandle::Xcb(_) => "x11",
                        rune::raw_window_handle::RawWindowHandle::Wayland(_) => "wayland",
                        _ => "unspecified",
                    })
                    .ok()
            })
            .unwrap_or("unspecified");
        handle.update(move |state, transaction| {
            #[cfg(feature = "hover")]
            let result = {
                state.native_backend = Some(backend);
                state.try_hover_ready().map(|_| ())
            };
            #[cfg(not(feature = "hover"))]
            let result = state
                .harness
                .mark_ready("native-first-drawn-frame-returned-no-display-confirmation");
            if let Err(error) = result {
                *state.failure.borrow_mut() = Some(error.to_string());
                transaction.quit();
            }
            transaction.live_only();
        });
    })
    .try_run()?;
    let error = failure.borrow_mut().take();
    if let Some(error) = error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(feature = "hover")]
fn expected_hover_viewport(
    controlled: bool,
    width: Option<&str>,
    height: Option<&str>,
) -> std::io::Result<Option<(f32, f32)>> {
    if !controlled && width.is_none() && height.is_none() {
        return Ok(None);
    }
    let dimension = |value: Option<&str>| {
        value
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 740)
    };
    match (dimension(width), dimension(height)) {
        (Some(width), Some(height)) => Ok(Some((width as f32, height as f32))),
        _ => Err(std::io::Error::other(
            "Rune fullscreen Hover requires runner-provided GUI_BENCH_HOVER_WIDTH and GUI_BENCH_HOVER_HEIGHT from the actual sole output",
        )),
    }
}

#[cfg(all(test, feature = "hover"))]
mod tests {
    use crate::{
        selected_hover::HoverGrid,
        ui::{Benchmark, expected_hover_viewport},
    };

    #[test]
    fn hover_readiness_waits_for_post_request_drawn_and_full_output_layout_in_either_order() {
        let mut state = Benchmark::<HoverGrid>::new().unwrap();
        state.expected_viewport = Some((1920.0, 1080.0));
        state.native_scale.set(Some(1.0));
        state.native_backend = Some("wayland");
        state.native_viewport = Some((1280.0, 800.0));
        assert!(!state.try_hover_ready().unwrap());
        state.fullscreen_requested = true;
        assert!(!state.try_hover_ready().unwrap());
        state.native_viewport = Some((1920.0, 1080.0));
        assert!(state.try_hover_ready().unwrap());

        let mut state = Benchmark::<HoverGrid>::new().unwrap();
        state.expected_viewport = Some((1920.0, 1080.0));
        state.native_scale.set(Some(1.0));
        state.fullscreen_requested = true;
        state.native_viewport = Some((1920.0, 1080.0));
        assert!(!state.try_hover_ready().unwrap());
        state.native_backend = Some("wayland");
        assert!(state.try_hover_ready().unwrap());

        let mut state = Benchmark::<HoverGrid>::new().unwrap();
        state.expected_viewport = Some((1280.0, 800.0));
        state.native_scale.set(Some(1.0));
        state.native_viewport = Some((1280.0, 800.0));
        state.fullscreen_requested = true;
        assert!(!state.try_hover_ready().unwrap());
        state.native_backend = Some("wayland");
        assert!(state.try_hover_ready().unwrap());
    }

    #[test]
    fn hover_expected_viewport_requires_runner_dimensions_without_guessing() {
        assert_eq!(expected_hover_viewport(false, None, None).unwrap(), None);
        assert_eq!(
            expected_hover_viewport(true, Some("1280"), Some("800")).unwrap(),
            Some((1280.0, 800.0))
        );
        for (width, height) in [
            (None, None),
            (Some("1920"), None),
            (Some("NaN"), Some("1080")),
            (Some("1920"), Some("740")),
        ] {
            assert!(expected_hover_viewport(true, width, height).is_err());
        }
    }
}
