use benchmark_common::{Harness, HoverObserver, Scenario, hover};
use makepad_widgets::*;

use crate::{items::BenchItems, ui::Scene as SceneBehavior};

live_design! {
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;
    use crate::widgets::BenchWindow;

    pub HoverCell = {{HoverCell}} { draw_bg: {} }
    pub SceneTemplate = <BenchWindow> {main_window = {hide_caption_on_fullscreen: true, body = {
        padding: 0, spacing: 0,
        cells = <BenchItems> {
            flow: RightWrap,
            padding: 0, spacing: 0,
            template: <HoverCell> {}
        }
    }}}
}

#[derive(Live, LiveHook, Widget)]
pub struct HoverCell {
    #[walk]
    walk: Walk,
    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[rust]
    index: usize,
    #[rust]
    observer: Option<HoverObserver>,
}

impl Widget for HoverCell {
    fn handle_event(&mut self, context: &mut Cx, event: &Event, _: &mut Scope) {
        let inside = match event.hits(context, self.draw_bg.area()) {
            Hit::FingerHoverIn(_) => true,
            Hit::FingerHoverOut(_) => false,
            _ => return,
        };
        if let Some(observer) = &self.observer {
            if inside {
                observer.enter(self.index);
            } else {
                observer.leave(self.index);
            }
        }
        self.draw_bg.color = color(if inside {
            hover::HOVERED_COLOR
        } else {
            hover::NORMAL_COLOR
        });
        self.draw_bg.redraw(context);
    }

    fn draw_walk(&mut self, context: &mut Cx2d, _: &mut Scope, walk: Walk) -> DrawStep {
        self.draw_bg.draw_walk(context, walk);
        DrawStep::done()
    }
}

fn color((red, green, blue): (u8, u8, u8)) -> Vec4 {
    vec4(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    )
}

#[derive(Default)]
pub(crate) struct Scene;

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::Hover;

    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.window(id!(main_window))
            .borrow_mut()
            .ok_or("Missing Hover window")?
            .set_fullscreen(context);
        let container = ui.widget(id!(cells));
        let grid_size = f64::from(hover::GRID_SIZE);
        container.apply_over(context, live! {width: (grid_size), height: (grid_size)});
        container
            .borrow_mut::<BenchItems>()
            .ok_or("Missing hover grid")?
            .resize(context, hover::COUNT)?;
        Ok(())
    }

    fn observe_hover(
        &mut self,
        _: &mut Cx,
        ui: &WidgetRef,
        observer: HoverObserver,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container = ui.widget(id!(cells));
        let cells = container
            .borrow::<BenchItems>()
            .ok_or("Missing hover grid")?;
        for (index, cell) in cells.items.iter().enumerate() {
            let mut cell = cell.borrow_mut::<HoverCell>().ok_or("Missing hover cell")?;
            cell.index = index;
            cell.observer = Some(observer.clone());
            cell.walk.width = Size::Fixed(f64::from(hover::CELL_SIZE));
            cell.walk.height = Size::Fixed(f64::from(hover::CELL_SIZE));
            cell.draw_bg.color = color(hover::NORMAL_COLOR);
        }
        Ok(())
    }

    fn mark_ready(
        &self,
        context: &mut Cx,
        ui: &WidgetRef,
        harness: &mut Harness,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let area = ui.widget(id!(cells)).area();
        let pass_id = area
            .draw_list_id()
            .and_then(|draw_list| context.draw_lists[draw_list].pass_id)
            .ok_or("Makepad Hover draw pass unavailable")?;
        let window_id = context
            .get_pass_window_id(pass_id)
            .ok_or("Makepad Hover window unavailable")?;
        let window = &context.windows[window_id];
        if !window.is_created || !window.window_geom.is_fullscreen {
            return Ok(());
        }
        harness.mark_hover_ready(
            "native-draw-processed-and-following-event-loop-tick-not-presented",
            backend(context.os_type()),
            window.window_geom.dpi_factor,
            window.window_geom.inner_size.x as f32,
            window.window_geom.inner_size.y as f32,
        )?;
        Ok(())
    }
}

fn backend(os_type: &OsType) -> &'static str {
    // Makepad 1.0.0 sets LinuxWindow only in its X11 event loop.
    match os_type {
        OsType::LinuxWindow(_) => "x11",
        _ => "unspecified",
    }
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::backend;
    use makepad_widgets::OsType;

    #[test]
    fn backend_does_not_invent_x11_for_other_platforms() {
        assert_eq!(backend(&OsType::LinuxDirect), "unspecified");
        assert_eq!(backend(&OsType::Unknown), "unspecified");
        assert_eq!(backend(&OsType::Macos), "unspecified");
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "linux") {
        return Err("Makepad 1.0.0 Linux Hover is unsupported: the public fullscreen operation is not implemented by its X11 backend".into());
    }
    crate::ui::run()
}
