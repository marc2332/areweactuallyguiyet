use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use benchmark_common::{FrameState, Harness, Scenario};
use zgui::{
    prelude::*,
    reactive::on_cleanup_local,
    runtime::{FrameProbe, Window},
};

pub struct Runtime {
    pub harness: RefCell<Harness>,
    failure: RefCell<Option<String>>,
}

impl Runtime {
    pub fn schedule(
        self: &Rc<Self>,
        mut tick: impl FnMut(&Runtime, FrameState) -> std::io::Result<u64> + 'static,
    ) {
        let runtime = Rc::clone(self);
        let windows = use_windows();
        let interval = set_interval(Duration::from_millis(16), move || {
            let result = (|| -> std::io::Result<bool> {
                let frame = runtime.harness.borrow_mut().poll()?;
                if frame.should_quit {
                    return Ok(false);
                }
                let applied = tick(&runtime, frame)?;
                runtime.harness.borrow_mut().report_applied(applied)?;
                Ok(true)
            })();
            match result {
                Ok(true) => {}
                Ok(false) => windows.quit(),
                Err(error) => {
                    *runtime.failure.borrow_mut() = Some(error.to_string());
                    windows.quit();
                }
            }
        });
        on_cleanup_local(move || drop(interval));
    }
}

impl FrameProbe for Runtime {
    fn frame_ended(&self, window: &Window) {
        if window.renderer().target().is_none() {
            return;
        }
        let boundary = "wgpu-configured-post-frame-probe-not-confirmed-presented";
        #[cfg(feature = "hover")]
        let result = {
            use raw_window_handle::RawWindowHandle;
            let surface = window.surface();
            if surface.fullscreen().is_none() {
                return;
            }
            let backend = match surface
                .gpu()
                .and_then(|native| native.window_handle().ok())
                .map(|handle| handle.as_raw())
            {
                Some(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "x11",
                Some(RawWindowHandle::Wayland(_)) => "wayland",
                _ => "unspecified",
            };
            self.harness.borrow_mut().mark_hover_ready(
                boundary,
                backend,
                surface.scale_factor(),
                (f64::from(surface.size().width.0) / surface.scale_factor()) as f32,
                (f64::from(surface.size().height.0) / surface.scale_factor()) as f32,
            )
        };
        #[cfg(not(feature = "hover"))]
        let result = self.harness.borrow_mut().mark_ready(boundary);
        if let Err(error) = result {
            *self.failure.borrow_mut() = Some(error.to_string());
        }
    }
}

pub fn run<V: IntoView + 'static>(
    scenario: Scenario,
    mut build: impl FnMut(Rc<Runtime>) -> V + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Rc::new(Runtime {
        harness: RefCell::new(Harness::connect(scenario)?),
        failure: RefCell::new(None),
    });
    let fonts = Fonts::shipped_only();
    fonts.register(
        Arc::new(include_bytes!("../../../assets/fonts/DejaVuSans.ttf").as_slice()),
        Some("DejaVu Sans"),
    )?;
    let build_runtime = Rc::clone(&runtime);
    #[cfg(feature = "hover")]
    let stylesheet = format!("{STYLE}{}", crate::selected_hover::style());
    #[cfg(not(feature = "hover"))]
    let stylesheet = STYLE;
    let app = app();
    #[cfg(feature = "hover")]
    let app = app
        .with_fullscreen(Some(FullscreenMode::Borderless))
        .with_decorations(Decorations::None);
    app.with_title("zgui benchmark")
        .with_size(benchmark_common::WIDTH, benchmark_common::HEIGHT)
        .with_fonts(fonts)
        .with_stylesheet(stylesheet)
        .with_probe(runtime.clone())
        .run(move || build(Rc::clone(&build_runtime)))?;
    if let Some(error) = runtime.failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row(index: usize) -> impl IntoView {
    let title = benchmark_common::row_title(index);
    let body = benchmark_common::row_body(index);
    view! {
        column(class = "benchmark-row") {
            text {{title}}
            text(class = "row-body") {{body}}
        }
    }
}

#[cfg(any(feature = "scroll", feature = "virtual-scroll", feature = "memory"))]
pub fn scroll_to(viewport: NodeRef, offset: f32) {
    use zgui::geom::{DevicePx, Point};

    viewport.scroll_to(
        ScrollTarget::Offset(Point::new(
            DevicePx(0.0),
            DevicePx(offset * viewport.scale()),
        )),
        ScrollBehavior::Instant,
    );
}

const STYLE: &str = "
:root { font-family: 'DejaVu Sans'; font-size: 14px; color: #000; background: #fff; }
column, row, scroll { box-sizing: border-box; }
.full { width: 100%; height: 100%; min-height: 0; min-width: 0; }
.benchmark-row { width: 100%; height: 48px; min-height: 48px; flex-shrink: 0; padding: 4px 8px; }
.row-body { font-size: 12px; }
.startup { padding: 16px; gap: 8px; }
.startup-list { width: 100%; flex: 1; min-height: 0; }
.startup-input { width: 430px; height: 32px; }
.apply { padding: 6px 12px; background: #eee; }
.switch-grid { gap: 0; }
.switch-row { height: 30px; flex-shrink: 0; gap: 0; }
.switch-cell { width: 30px; height: 30px; flex-shrink: 0; align-items: center; justify-content: center; color: #fff; font-size: 12px; background: #868e96; }
.switch-cell.on { background: #2f9e44; }
.dashboard-header { height: 56px; min-height: 56px; padding: 8px; gap: 12px; align-items: center; }
.search { flex: 1; height: 32px; }
.dashboard-main { width: 100%; flex: 1; min-height: 0; }
.sidebar { width: 180px; min-width: 180px; height: 100%; padding: 8px; background: #f1f3f5; }
.navigation { height: 22px; min-height: 22px; font-size: 12px; }
.thumbnails { gap: 4px; }
.thumbnail-row { gap: 8px; }
.thumbnail { width: 48px; height: 48px; }
.records { width: 600px; min-width: 600px; height: 100%; }
.record { width: 100%; height: 64px; min-height: 64px; flex-shrink: 0; padding: 8px; align-items: center; }
.record.selected { background: #e3edff; }
.record-copy { flex: 1; min-width: 0; height: 56px; }
.record-body { font-size: 12px; }
.record-toggle { width: 48px; height: 24px; flex-shrink: 0; align-items: center; justify-content: center; color: #fff; background: #868e96; }
.properties { flex: 1; min-width: 0; height: 100%; padding: 8px; }
.detail { width: 100%; height: 300px; min-height: 300px; }
.detail-title { font-size: 18px; }
.detail-body { width: 100%; white-space: normal; }
.form { width: 100%; flex: 1; min-height: 0; }
.form-row { width: 100%; height: 36px; min-height: 36px; align-items: center; gap: 8px; }
.field-label { width: 80px; }
.form-input { flex: 1; min-width: 0; height: 32px; border: 1px solid #aaa; }
";
