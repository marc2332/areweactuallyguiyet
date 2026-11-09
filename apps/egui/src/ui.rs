use benchmark_common::{FrameState, HEIGHT, Harness, WIDTH};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use eframe::egui::Vec2;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
#[cfg(any(feature = "switches", feature = "memory"))]
use eframe::egui::{Response, Sense, Widget};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

type Error = Box<dyn std::error::Error>;

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub struct Record {
    pub title: String,
    pub body: String,
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
impl Record {
    pub fn new(index: usize, body: String) -> Self {
        Self {
            title: benchmark_common::row_title(index),
            body,
        }
    }
}

#[cfg(any(feature = "switches", feature = "memory"))]
pub struct DummyToggle {
    pub on: bool,
}

#[cfg(any(feature = "switches", feature = "memory"))]
impl Widget for DummyToggle {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let size = ui.available_size_before_wrap();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let (red, green, blue) = if self.on {
            benchmark_common::SWITCH_ON_COLOR
        } else {
            benchmark_common::SWITCH_OFF_COLOR
        };
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(red, green, blue));
        let (red, green, blue) = benchmark_common::SWITCH_TEXT_COLOR;
        let text = if self.on { "ON" } else { "OFF" };
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(benchmark_common::SWITCH_FONT_SIZE),
            egui::Color32::from_rgb(red, green, blue),
        );
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            egui::Color32::WHITE,
        );
        response
    }
}

pub fn configure(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "benchmark".into(),
        FontData::from_static(include_bytes!("../../../assets/fonts/DejaVuSans.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "benchmark".into());
    context.set_fonts(fonts);
    context.all_styles_mut(|style| {
        style.animation_time = 0.0;
        style.spacing.item_spacing.y = 0.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    });
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row(ui: &mut egui::Ui, record: &Record, index: usize) {
    ui.push_id(index, |ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), benchmark_common::ROW_HEIGHT),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_height(benchmark_common::ROW_HEIGHT);
                ui.add(egui::Label::new(&record.title).truncate());
                ui.add(egui::Label::new(&record.body).truncate());
            },
        );
    });
}

pub trait ScenarioUi: Sized {
    const SCENARIO: benchmark_common::Scenario;
    fn new(context: &egui::Context) -> Result<Self, image::ImageError>;
    fn update(
        &mut self,
        state: FrameState,
        harness: &mut Harness,
        context: &egui::Context,
    ) -> Result<u64, Error>;
    fn render(&mut self, ui: &mut egui::Ui);
}

struct BenchmarkApp<S> {
    harness: Harness,
    state: FrameState,
    scenario: S,
    failure: Rc<RefCell<Option<String>>>,
    initial_painted: Arc<AtomicBool>,
    ready_sent: bool,
}

impl<S: ScenarioUi> BenchmarkApp<S> {
    fn new(
        harness: Harness,
        context: &egui::Context,
        failure: Rc<RefCell<Option<String>>>,
    ) -> Result<Self, image::ImageError> {
        configure(context);
        Ok(Self {
            harness,
            state: FrameState::default(),
            scenario: S::new(context)?,
            failure,
            initial_painted: Arc::new(AtomicBool::new(false)),
            ready_sent: false,
        })
    }

    fn fail(&mut self, context: &egui::Context, error: impl ToString) {
        *self.failure.borrow_mut() = Some(error.to_string());
        context.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl<S: ScenarioUi> eframe::App for BenchmarkApp<S> {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        let initial_frame_ready = self.initial_painted.load(Ordering::Acquire);
        #[cfg(feature = "hover")]
        let initial_frame_ready =
            initial_frame_ready && crate::selected_hover::fullscreen_viewport_ready(context);
        if !self.ready_sent && initial_frame_ready {
            #[cfg(feature = "hover")]
            let readiness = crate::selected_hover::mark_ready(&mut self.harness, context, _frame);
            #[cfg(not(feature = "hover"))]
            let readiness = self
                .harness
                .mark_ready("initial-paint-returned-following-update-not-presented");
            if let Err(error) = readiness {
                self.fail(context, error);
                return;
            }
            self.ready_sent = true;
        }
        match self.harness.poll() {
            Ok(state) => self.state = state,
            Err(error) => {
                self.fail(context, error);
                return;
            }
        }
        if self.state.should_quit {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        match self.scenario.update(self.state, &mut self.harness, context) {
            Ok(applied) => {
                if let Err(error) = self.harness.report_applied(applied) {
                    self.fail(context, error);
                }
            }
            Err(error) => self.fail(context, error),
        }
        if self.state.running && !cfg!(feature = "hover") {
            context.request_repaint();
        } else if self.harness.controlled() {
            context.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.scenario.render(ui);
        let can_mark_painted = !self.ready_sent;
        if can_mark_painted {
            let painted = self.initial_painted.clone();
            let context = ui.ctx().clone();
            ui.ctx()
                .layer_painter(egui::LayerId::new(
                    egui::Order::Debug,
                    egui::Id::new("initial-frame-ready"),
                ))
                .add(egui::PaintCallback {
                    rect: ui.max_rect(),
                    callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, _| {
                        painted.store(true, Ordering::Release);
                        context.request_repaint();
                    })),
                });
        }
    }
}

pub fn run<S: ScenarioUi>() -> Result<(), Error> {
    let harness = Harness::connect(S::SCENARIO)?;
    let failure = Rc::new(RefCell::new(None));
    let app_failure = failure.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("GUI benchmark: egui / {}", S::SCENARIO.id()))
            .with_inner_size([WIDTH, HEIGHT])
            .with_fullscreen(S::SCENARIO == benchmark_common::Scenario::Hover)
            .with_decorations(S::SCENARIO != benchmark_common::Scenario::Hover),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Rust GUI benchmark",
        options,
        Box::new(move |creation| {
            Ok(Box::new(BenchmarkApp::<S>::new(
                harness,
                &creation.egui_ctx,
                app_failure,
            )?))
        }),
    )?;
    if let Some(message) = failure.borrow_mut().take() {
        return Err(message.into());
    }
    Ok(())
}
