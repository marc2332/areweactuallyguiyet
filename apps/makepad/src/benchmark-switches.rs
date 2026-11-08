use crate::items::BenchItems;
use crate::ui::Scene as SceneBehavior;
use benchmark_common::{FrameState, Scenario, SwitchMode, Switches};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;
    use crate::widgets::BenchWindow;
    use crate::widgets::BenchText;

    pub SceneTemplate = <BenchWindow> {main_window = {body = {
            cells = <BenchItems> {
                width: 900, height: 900, flow: RightWrap,
                template: <View> {
                    width: 30, height: 30, padding: 0, margin: 0,
                    cursor: Hand, show_bg: true, draw_bg: {color: #868e96},
                    align: {x: 0.5, y: 0.5},
                    value = <BenchText> {
                        width: Fit, height: Fit,
                        align: {x: 0.5, y: 0.5},
                        draw_text: {color: #ffffff, text_style: {font_size: 9}}, text: "OFF"
                    }
                }
            }
        }}}
}

#[derive(Default)]
pub(crate) struct Scene {
    switches: Switches,
}

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::Switches;
    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.widget(id!(cells))
            .borrow_mut::<BenchItems>()
            .ok_or("Missing native cell container")?
            .resize(context, 900)?;
        Ok(())
    }

    fn reset(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.switches = Switches::default();
        let container = ui.widget(id!(cells));
        if let Some(cells) = container.borrow::<BenchItems>() {
            for cell in &cells.items {
                Self::set_cell(context, cell, false);
            }
        }
        Ok(())
    }

    fn update(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        frame: FrameState,
        mode: SwitchMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container = ui.widget(id!(cells));
        if let Some(cells) = container.borrow::<BenchItems>() {
            self.switches
                .advance_changed(frame.update_index, mode, |index, active| {
                    Self::set_cell(context, &cells.items[index], active)
                });
        }
        Ok(())
    }

    fn applied(&self, _frame: FrameState) -> u64 {
        self.switches.applied_updates()
    }

    fn actions(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        actions: &Actions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container = ui.widget(id!(cells));
        if let Some(cells) = container.borrow::<BenchItems>() {
            for (index, cell) in cells.items.iter().enumerate() {
                if cell
                    .as_view()
                    .finger_up(actions)
                    .is_some_and(|event| event.is_over)
                {
                    self.switches.toggle(index);
                    Self::set_cell(context, cell, self.switches.get(index));
                }
            }
        }
        Ok(())
    }
}

impl Scene {
    fn set_cell(context: &mut Cx, cell: &WidgetRef, active: bool) {
        let color = if active {
            vec4(47.0 / 255.0, 158.0 / 255.0, 68.0 / 255.0, 1.0)
        } else {
            vec4(134.0 / 255.0, 142.0 / 255.0, 150.0 / 255.0, 1.0)
        };
        cell.apply_over(context, live! {draw_bg: {color: (color)}});
        cell.label(id!(value))
            .set_text(context, if active { "ON" } else { "OFF" });
        cell.redraw(context);
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run()
}
