use benchmark_common::{
    FrameState, HEIGHT, Harness, ROW_HEIGHT, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE,
    SWITCH_OFF_COLOR, SWITCH_ON_COLOR, Scenario, Switches, WIDTH,
    dashboard::{Dashboard, THUMBNAILS},
    row_body, row_title,
};
use rune::{
    App, AppSpec, Color, Cx, El, GrowthSpec, Image, ImageCache, ScrollSpec, TextEdit, TextStyle,
    Tx, col, memo, row, text,
};
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

pub type Error = Box<dyn std::error::Error>;

pub struct Benchmark {
    pub scenario: Scenario,
    pub harness: Harness,
    pub frame: FrameState,
    pub switches: Switches,
    pub dashboard: Option<Dashboard>,
    pub search: TextEdit,
    pub fields: Vec<TextEdit>,
    pub thumbnails: Vec<Image>,
    pub visible_rows: usize,
    initialized: bool,
    failure: Rc<RefCell<Option<String>>>,
}

impl Benchmark {
    fn new(scenario: Scenario) -> Result<Self, Error> {
        let harness = Harness::connect(scenario)?;
        let dashboard = (scenario == Scenario::Memory).then(Dashboard::default);
        let fields = dashboard.as_ref().map_or_else(Vec::new, |dashboard| {
            dashboard
                .fields
                .iter()
                .map(|value| TextEdit::new(value.clone()))
                .collect()
        });
        let thumbnails = if dashboard.is_some() {
            let cache = ImageCache::new(Arc::new(|| {}));
            let images = THUMBNAILS
                .iter()
                .map(|bytes| cache.load(Arc::<[u8]>::from(*bytes)))
                .collect::<Vec<_>>();
            for image in &images {
                image.wait().map_err(|error| error.to_string())?;
            }
            images
        } else {
            Vec::new()
        };
        Ok(Self {
            scenario,
            harness,
            frame: FrameState::default(),
            switches: Switches::default(),
            dashboard,
            search: TextEdit::new(""),
            fields,
            thumbnails,
            visible_rows: 0,
            initialized: false,
            failure: Rc::default(),
        })
    }

    fn synchronize_editors(&mut self) {
        if let Some(dashboard) = &self.dashboard {
            if self.search.text != dashboard.query {
                self.search.replace_text(dashboard.query.clone());
            }
            for (editor, value) in self.fields.iter_mut().zip(&dashboard.fields) {
                if editor.text != *value {
                    editor.replace_text(value.clone());
                }
            }
        }
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
        let mut changed = reset;
        transaction.instant();
        if reset {
            self.initialized = true;
            self.switches = Switches::default();
            if let Some(dashboard) = &mut self.dashboard {
                dashboard.reset();
            }
            self.synchronize_editors();
            transaction.scroll_to_y("records", 0.0);
        }
        if frame.running {
            match self.scenario {
                Scenario::Switches if updated => {
                    self.switches
                        .advance(frame.update_index, self.harness.switch_mode());
                    changed = true;
                }
                Scenario::Memory => {
                    if let Some(dashboard) = &mut self.dashboard {
                        dashboard.advance(frame);
                    }
                    self.synchronize_editors();
                    transaction.scroll_to_y("records", frame.scroll_offset);
                    changed |= updated;
                }
                Scenario::Scroll | Scenario::VirtualScroll => {
                    transaction.scroll_to_y("records", frame.scroll_offset);
                }
                _ => {}
            }
        }
        self.frame = frame;
        let applied = match self.scenario {
            Scenario::Switches => self.switches.applied_updates(),
            Scenario::Memory => self
                .dashboard
                .as_ref()
                .map_or(0, Dashboard::applied_updates),
            _ => frame.update_index,
        };
        self.harness.report_applied(applied)?;
        if let Some(dashboard) = &self.dashboard {
            self.harness.report_dashboard(dashboard)?;
        }
        Ok(changed)
    }
}

pub fn label(value: impl Into<String>, size: f32) -> El {
    text(value)
        .font(TextStyle::new(size, 400.0))
        .color(Color::BLACK)
}

pub fn color((red, green, blue): (u8, u8, u8)) -> Color {
    Color {
        r: rune::color::srgb_decode(f32::from(red) / 255.0),
        g: rune::color::srgb_decode(f32::from(green) / 255.0),
        b: rune::color::srgb_decode(f32::from(blue) / 255.0),
        a: 1.0,
    }
}

fn record(index: usize) -> El {
    col()
        .key(format!("record-{index}"))
        .w_full()
        .h(ROW_HEIGHT)
        .child(label(row_title(index), 14.0))
        .child(label(row_body(index), 14.0))
}

fn list(state: &Benchmark, virtualized: bool) -> El {
    let content = if virtualized {
        col()
            .key("rows")
            .w_full()
            .growth(GrowthSpec::default())
            .virtual_rows(
                state.scenario.row_count(),
                ROW_HEIGHT,
                |_: &Benchmark, index| format!("record-{index}").into(),
                |_: &Benchmark, index| record(index),
            )
            .virtual_rows_version(0_u64)
            .on_visible_rows(|state: &mut Benchmark, range| {
                state.visible_rows = range.len();
                false
            })
    } else {
        memo("retained-records", state.scenario.row_count(), || {
            col()
                .w_full()
                .children((0..state.scenario.row_count()).map(record))
        })
    };
    col()
        .key("records")
        .w_full()
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(content)
}

fn dummy(index: usize, on: bool) -> El {
    row()
        .key(format!("dummy-{index}"))
        .size(30.0, 30.0)
        .center()
        .fill(color(if on {
            SWITCH_ON_COLOR
        } else {
            SWITCH_OFF_COLOR
        }))
        .role("switch")
        .label(format!("Dummy {index}"))
        .checked(on)
        .focusable(true)
        .on_press_tx(move |state: &mut Benchmark, transaction| {
            state.switches.toggle(index);
            transaction.instant();
        })
        .child(
            text(if on { "ON" } else { "OFF" })
                .font(TextStyle::new(SWITCH_FONT_SIZE, 400.0))
                .color(Color::WHITE),
        )
}

fn switches(state: &Benchmark) -> El {
    col()
        .key("dummy-grid")
        .size(900.0, 900.0)
        .children((0..SWITCH_COUNT / SWITCH_COLUMNS).map(|row_index| {
            row().h(30.0).children((0..SWITCH_COLUMNS).map(|column| {
                let index = row_index * SWITCH_COLUMNS + column;
                let on = state.switches.get(index);
                memo(format!("dummy-cache-{index}"), on, || dummy(index, on))
            }))
        }))
}

fn view(state: &Benchmark, _: &mut Cx<Benchmark>) -> El {
    let content = match state.scenario {
        Scenario::Memory => crate::dashboard::view(state),
        Scenario::Switches => switches(state),
        Scenario::VirtualScroll => list(state, true),
        Scenario::Scroll => list(state, false),
        Scenario::Startup => col()
            .w_full()
            .h_full()
            .child(
                row()
                    .w_full()
                    .h(56.0)
                    .center_y()
                    .gap(12.0)
                    .child(label("Dashboard", 18.0))
                    .child(
                        rune::text_input(&state.search, TextStyle::new(14.0, 400.0))
                            .key("search")
                            .grow(1.0)
                            .h(36.0)
                            .single_line()
                            .on_edit(|state: &mut Benchmark| &mut state.search),
                    ),
            )
            .child(
                col()
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .child(list(state, false)),
            ),
    };
    col()
        .w_full()
        .h_full()
        .fill(Color::WHITE)
        .clip_rounded(0.0)
        .instant_changes()
        .child(content)
}

fn poll(state: &mut Benchmark, _: f64, transaction: &mut Tx) -> bool {
    match state.advance(transaction) {
        Ok(changed) => changed,
        Err(error) => {
            *state.failure.borrow_mut() = Some(error.to_string());
            transaction.quit();
            false
        }
    }
}

fn native_initializer() -> Benchmark {
    eprintln!("Rune benchmark requires its native initializer");
    std::process::exit(1);
}

pub fn run(scenario: Scenario) -> Result<(), Error> {
    let state = Benchmark::new(scenario)?;
    let failure = state.failure.clone();
    let spec = AppSpec {
        fonts: &[include_bytes!("../../../assets/fonts/DejaVuSans.ttf")],
        families: ("DejaVu Sans", "DejaVu Sans"),
        poll: Some(poll),
        ..AppSpec::new(
            "GUI benchmark: rune",
            (WIDTH, HEIGHT),
            native_initializer,
            view,
        )
    };
    let app = App::new(spec).args(Vec::new()).init(move || state);
    let handle = app.handle();
    app.on_ready(move |_| {
        handle.update(|state, transaction| {
            if let Err(error) = state
                .harness
                .mark_ready("native-first-frame-ready-not-presented")
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
