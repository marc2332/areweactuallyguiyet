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
    failure: Rc<RefCell<Option<String>>>,
}

impl<S: ScenarioUi> Benchmark<S> {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            harness: Harness::connect(S::SCENARIO)?,
            frame: FrameState::default(),
            content: S::new()?,
            initialized: false,
            failure: Rc::default(),
        })
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

#[cfg(any(feature = "switches", feature = "memory"))]
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

fn view<S: ScenarioUi>(state: &Benchmark<S>, _: &mut Cx<Benchmark<S>>) -> El {
    col()
        .w_full()
        .h_full()
        .fill(Color::WHITE)
        .clip_rounded(0.0)
        .instant_changes()
        .child(S::view(state))
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
    let handle = app.handle();
    let mut ready_sent = false;
    app.on_frame_encoded(move |frame| {
        if ready_sent || frame.outcome() != rune::FrameOutcome::Drawn {
            return;
        }
        ready_sent = true;
        handle.update(|state, transaction| {
            if let Err(error) = state
                .harness
                .mark_ready("native-first-drawn-frame-returned-no-display-confirmation")
            {
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
