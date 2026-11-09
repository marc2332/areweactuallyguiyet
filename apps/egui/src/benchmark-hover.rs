use crate::ui::{self, ScenarioUi};
use benchmark_common::{
    FrameState, Harness, Scenario,
    hover::{self, HoverObserver},
};
use eframe::egui::{self, Vec2};

struct HoverGrid {
    observer: Option<HoverObserver>,
}

impl ScenarioUi for HoverGrid {
    const SCENARIO: Scenario = Scenario::Hover;

    fn new(_context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self { observer: None })
    }

    fn update(
        &mut self,
        _state: FrameState,
        harness: &mut Harness,
        _context: &egui::Context,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.observer
            .get_or_insert_with(|| harness.hover_observer());
        Ok(0)
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let background = hover::BACKGROUND;
        ui.painter().rect_filled(
            ui.max_rect(),
            0.0,
            egui::Color32::from_rgb(background.0, background.1, background.2),
        );
        let mut current = None;
        for index in 0..hover::COUNT {
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    hover::ORIGIN.0 + (index % hover::COLUMNS) as f32 * hover::CELL_SIZE,
                    hover::ORIGIN.1 + (index / hover::COLUMNS) as f32 * hover::CELL_SIZE,
                ),
                Vec2::splat(hover::CELL_SIZE),
            );
            let response = ui.interact(rect, ui.scope_id().with(index), egui::Sense::hover());
            let color = if response.hovered() {
                current = Some(index);
                hover::HOVERED_COLOR
            } else {
                hover::NORMAL_COLOR
            };
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgb(color.0, color.1, color.2),
            );
        }
        if let Some(observer) = &self.observer {
            observer.current(current);
        }
    }
}

pub(crate) fn fullscreen_viewport_ready(context: &egui::Context) -> bool {
    context.input(|input| {
        let viewport = input.viewport();
        viewport.fullscreen == Some(true)
            && viewport.native_pixels_per_point.is_some()
            && viewport.monitor_size.is_some_and(|size| {
                size.x > 0.0 && size.y > 0.0 && size == input.viewport_rect().size()
            })
    })
}

pub(crate) fn mark_ready(
    harness: &mut Harness,
    context: &egui::Context,
    frame: &eframe::Frame,
) -> Result<(), Box<dyn std::error::Error>> {
    use raw_window_handle::HasDisplayHandle;
    let backend = backend(frame.display_handle().ok().map(|handle| handle.as_raw()));
    harness.mark_hover_ready(
        "initial-paint-returned-following-update-not-presented",
        backend,
        f64::from(
            context
                .input(|input| input.viewport().native_pixels_per_point)
                .ok_or("egui native scale is unavailable")?,
        ),
        context.viewport_rect().width(),
        context.viewport_rect().height(),
    )?;
    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<HoverGrid>()
}

fn backend(handle: Option<raw_window_handle::RawDisplayHandle>) -> &'static str {
    use raw_window_handle::RawDisplayHandle;
    match handle {
        Some(RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_)) => "x11",
        Some(RawDisplayHandle::Wayland(_)) => "wayland",
        _ => "unspecified",
    }
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::{backend, fullscreen_viewport_ready};
    use eframe::egui;
    use raw_window_handle::{
        RawDisplayHandle, WaylandDisplayHandle, WebDisplayHandle, XcbDisplayHandle,
        XlibDisplayHandle,
    };
    use std::ptr::NonNull;

    #[test]
    fn fullscreen_readiness_waits_for_native_state_and_full_monitor_viewport() {
        let context = egui::Context::default();
        for (fullscreen, monitor, viewport, scale, expected) in [
            (
                Some(false),
                Some([1920., 1080.]),
                [1920., 1080.],
                Some(1.),
                false,
            ),
            (None, Some([1920., 1080.]), [1920., 1080.], Some(1.), false),
            (
                Some(true),
                Some([1920., 1080.]),
                [1280., 800.],
                Some(1.),
                false,
            ),
            (Some(true), None, [1920., 1080.], Some(1.), false),
            (
                Some(true),
                Some([1920., 1080.]),
                [1920., 1080.],
                None,
                false,
            ),
            (
                Some(true),
                Some([1920., 1080.]),
                [1920., 1080.],
                Some(1.),
                true,
            ),
        ] {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, viewport.into())),
                ..Default::default()
            };
            input.viewports.insert(
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    fullscreen,
                    monitor_size: monitor.map(Into::into),
                    native_pixels_per_point: scale,
                    ..Default::default()
                },
            );
            context.begin_pass(input);
            assert_eq!(fullscreen_viewport_ready(&context), expected);
            context.end_pass().textures_delta.clear();
        }
    }

    #[test]
    fn backend_accepts_wayland_and_x11_and_labels_missing_identity_unspecified() {
        assert_eq!(
            backend(Some(RawDisplayHandle::Xlib(XlibDisplayHandle::new(
                None, 0
            )))),
            "x11"
        );
        assert_eq!(
            backend(Some(RawDisplayHandle::Xcb(XcbDisplayHandle::new(None, 0)))),
            "x11"
        );
        assert_eq!(
            backend(Some(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                NonNull::dangling(),
            )))),
            "wayland"
        );
        assert_eq!(backend(None), "unspecified");
        assert_eq!(
            backend(Some(RawDisplayHandle::Web(WebDisplayHandle::new()))),
            "unspecified"
        );
    }
}
