use crate::ui::Scene as SceneBehavior;
use benchmark_common::{FrameState, Scenario, SwitchMode, row_body, row_title};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;
    use crate::widgets::BenchWindow;
    use crate::widgets::BenchRow;

    pub SceneTemplate = <BenchWindow> {main_window = {body = {
            virtual_list = <PortalList> {
                width: Fill, height: Fill, reuse_items: true,
                Row = <BenchRow> {}
            }
        }}}
}

#[derive(Default)]
pub(crate) struct Scene {}

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::VirtualScroll;
    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.portal_list(id!(virtual_list))
            .borrow_mut()
            .ok_or("Missing native virtual list")?
            .set_item_range(context, 0, 100_000);
        Ok(())
    }

    fn reset(
        &mut self,
        _context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.portal_list(id!(virtual_list))
            .set_first_id_and_scroll(0, 0.0);
        Ok(())
    }

    fn update(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        frame: FrameState,
        _mode: SwitchMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let offset = f64::from(frame.scroll_offset);
        let list = ui.portal_list(id!(virtual_list));
        list.set_first_id_and_scroll((offset / 48.0).floor() as usize, -(offset % 48.0));
        list.redraw(context);
        Ok(())
    }

    fn draw(&mut self, context: &mut Cx2d, ui: &mut WidgetRef) {
        let mut scope = Scope::empty();
        while let Some(widget) = ui.draw(context, &mut scope).step() {
            if let Some(mut list) = widget.as_portal_list().borrow_mut() {
                while let Some(index) = list.next_visible_item(context) {
                    let row = list.item(context, index, live_id!(Row));
                    row.label(id!(title))
                        .set_text_with(|text| *text = row_title(index));
                    row.label(id!(body))
                        .set_text_with(|text| *text = row_body(index));
                    row.draw_all(context, &mut scope);
                }
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run()
}
