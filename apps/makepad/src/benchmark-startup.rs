use crate::items::BenchItems;
use crate::ui::Scene as SceneBehavior;
use benchmark_common::{Scenario, row_body, row_title};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;
    use crate::widgets::BenchWindow;
    use crate::widgets::BenchText;
    use crate::widgets::BenchInput;
    use crate::widgets::BenchButton;
    use crate::widgets::BenchRow;

    pub SceneTemplate = <BenchWindow> {main_window = {body = {
            flow: Down, padding: 16, spacing: 8,
            <BenchText> {text: "Benchmark startup", height: 24, draw_text: {text_style: {font_size: 15}}}
            <BenchText> {text: "Prepared initial content", height: 20}
            <BenchInput> {width: 430}
            <BenchButton> {width: 120, height: 32, text: "Apply"}
            <ScrollYView> {width: Fill, height: Fill, rows = <BenchItems> {template: <BenchRow> {}}}
        }}}
}

#[derive(Default)]
pub(crate) struct Scene {}

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::Startup;
    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let container = ui.widget(id!(rows));
        let mut rows = container
            .borrow_mut::<BenchItems>()
            .ok_or("Missing native row container")?;
        rows.resize(context, Scenario::Startup.row_count())?;
        for (index, row) in rows.items.iter().enumerate() {
            row.label(id!(title)).set_text(context, &row_title(index));
            row.label(id!(body)).set_text(context, &row_body(index));
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run()
}
