use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use benchmark_common::{Harness, Scenario};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{row_body, row_title};
use floem::{AnyView, ViewId, prelude::*, window::WindowConfig};

pub struct Runtime {
    pub harness: RefCell<Harness>,
    pub failure: RefCell<Option<String>>,
    ready: Cell<bool>,
}

pub fn run(
    scenario: Scenario,
    build: impl FnOnce(Rc<Runtime>) -> AnyView + 'static,
    tick: impl Fn(&Runtime, benchmark_common::FrameState) -> std::io::Result<u64> + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    floem::text::FONT_SYSTEM
        .lock()
        .db_mut()
        .load_font_data(include_bytes!("../../../assets/fonts/DejaVuSans.ttf").to_vec());
    let runtime = Rc::new(Runtime {
        harness: RefCell::new(Harness::connect(scenario)?),
        failure: RefCell::new(None),
        ready: Cell::new(false),
    });
    let app_runtime = Rc::clone(&runtime);
    let tick: Rc<dyn Fn(&Runtime, benchmark_common::FrameState) -> std::io::Result<u64>> =
        Rc::new(tick);
    let config = WindowConfig::default()
        .size((1280.0, 800.0))
        .title("Floem benchmark")
        .apply_default_theme(false);
    #[cfg(feature = "hover")]
    let config = config
        .fullscreen(floem::window::Fullscreen::Borderless(None))
        .undecorated(true);
    floem::Application::new()
        .window(
            move |_| {
                schedule(Rc::clone(&app_runtime), Rc::clone(&tick));
                stack((
                    build(Rc::clone(&app_runtime)),
                    ReadyProbe {
                        id: ViewId::new(),
                        runtime: app_runtime,
                    }
                    .style(|style| style.absolute().width(1.0).height(1.0))
                    .pointer_events(|| !cfg!(feature = "hover")),
                ))
                .style(|style| {
                    style
                        .size_full()
                        .font_family("DejaVu Sans".to_owned())
                        .font_size(14.0)
                        .color(Color::BLACK)
                        .background(Color::WHITE)
                })
            },
            Some(config),
        )
        .run();
    if let Some(error) = runtime.failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

fn schedule(
    runtime: Rc<Runtime>,
    tick: Rc<dyn Fn(&Runtime, benchmark_common::FrameState) -> std::io::Result<u64>>,
) {
    floem::action::exec_after(Duration::from_millis(16), move |_| {
        let result = (|| -> std::io::Result<bool> {
            let frame = runtime.harness.borrow_mut().poll()?;
            if frame.should_quit {
                return Ok(false);
            }
            let applied = tick(&runtime, frame)?;
            #[cfg(not(feature = "hover"))]
            runtime.harness.borrow_mut().report_applied(applied)?;
            #[cfg(feature = "hover")]
            let _ = applied;
            Ok(true)
        })();
        match result {
            Ok(true) => schedule(runtime, tick),
            Ok(false) => floem::quit_app(),
            Err(error) => {
                *runtime.failure.borrow_mut() = Some(error.to_string());
                floem::quit_app();
            }
        }
    });
}

type PaintRenderer = <floem::context::PaintCx<'static> as std::ops::Deref>::Target;
struct ReadyProbe {
    id: ViewId,
    runtime: Rc<Runtime>,
}
impl View for ReadyProbe {
    fn id(&self) -> ViewId {
        self.id
    }
    fn paint(&mut self, context: &mut floem::context::PaintCx) {
        let boundary = match &**context {
            PaintRenderer::Uninitialized { .. } => return,
            PaintRenderer::Vger(_) => "vger-initial-paint-returned-following-tick-not-presented",
            PaintRenderer::TinySkia(_) => {
                "tiny-skia-initial-paint-returned-following-tick-not-presented"
            }
        };
        #[cfg(feature = "hover")]
        {
            use floem::WindowIdExt;
            let Some(window) = self.id.window_id() else {
                return;
            };
            let Some(monitor) = window.monitor_bounds() else {
                return;
            };
            let Some(layout) = window.root_view().and_then(|root| root.get_layout()) else {
                return;
            };
            if f64::from(layout.size.width) != monitor.width()
                || f64::from(layout.size.height) != monitor.height()
            {
                return;
            }
        }
        if !self.runtime.ready.get() {
            self.runtime.ready.set(true);
            let runtime = self.runtime.clone();
            #[cfg(feature = "hover")]
            let id = self.id;
            floem::action::exec_after(Duration::ZERO, move |_| {
                #[cfg(not(feature = "hover"))]
                let result = runtime.harness.borrow_mut().mark_ready(boundary);
                #[cfg(feature = "hover")]
                let result = (|| {
                    use floem::WindowIdExt;
                    let window = id.window_id().ok_or_else(|| {
                        std::io::Error::other("Floem Hover grid has no native window")
                    })?;
                    let layout = window
                        .root_view()
                        .and_then(|root| root.get_layout())
                        .ok_or_else(|| {
                            std::io::Error::other("Floem native root layout unavailable")
                        })?;
                    runtime.harness.borrow_mut().mark_hover_ready(
                        boundary,
                        "unspecified",
                        window.scale(),
                        layout.size.width,
                        layout.size.height,
                    )
                })();
                if let Err(error) = result {
                    *runtime.failure.borrow_mut() = Some(error.to_string());
                    floem::quit_app();
                }
            });
        }
    }
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row(index: usize) -> impl IntoView {
    let title = row_title(index);
    let body = row_body(index);
    v_stack((
        label(move || title.clone()),
        label(move || body.clone()).style(|style| style.font_size(12.0)),
    ))
    .style(|style| {
        style
            .width_full()
            .height(48.0)
            .min_height(48.0)
            .padding_horiz(8.0)
            .padding_vert(4.0)
            .flex_shrink(0.0)
    })
}
