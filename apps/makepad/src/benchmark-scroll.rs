use crate::items::BenchItems;
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
            scroller = <ScrollYView> {width: Fill, height: Fill, rows = <BenchItems> {template: <BenchRow> {}}}
        }}}
}

#[derive(Default)]
pub(crate) struct Scene {}

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::Scroll;
    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container = ui.widget(id!(rows));
        let mut rows = container
            .borrow_mut::<BenchItems>()
            .ok_or("Missing native row container")?;
        rows.resize(context, Scenario::Scroll.row_count())?;
        for (index, row) in rows.items.iter().enumerate() {
            row.label(id!(title)).set_text(context, &row_title(index));
            row.label(id!(body)).set_text(context, &row_body(index));
        }
        Ok(())
    }

    fn reset(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.view(id!(scroller))
            .set_scroll_pos(context, dvec2(0.0, 0.0));
        Ok(())
    }

    fn update(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        frame: FrameState,
        _mode: SwitchMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.view(id!(scroller))
            .set_scroll_pos(context, dvec2(0.0, f64::from(frame.scroll_offset)));
        ui.view(id!(scroller)).redraw(context);
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run()
}
