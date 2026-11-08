use crate::ui::{self, DummyToggle, ScenarioUi};
use benchmark_common::{
    FrameState, Harness, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_ROWS, Scenario,
    Switches,
};
use eframe::egui::{self, Vec2};

struct SwitchGrid {
    switches: Switches,
    state: FrameState,
}

impl ScenarioUi for SwitchGrid {
    const SCENARIO: Scenario = Scenario::Switches;

    fn new(_context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self {
            switches: Switches::default(),
            state: FrameState::default(),
        })
    }

    fn update(
        &mut self,
        state: FrameState,
        harness: &mut Harness,
        _context: &egui::Context,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if state.reset_generation != self.state.reset_generation {
            self.switches = Switches::default();
        }
        self.state = state;
        self.switches
            .advance_changed(state.update_index, harness.switch_mode(), |_, _| {});
        Ok(self.switches.applied_updates())
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let cell_size = Vec2::splat(SWITCH_CELL_SIZE);
        let grid_size = Vec2::new(
            SWITCH_COLUMNS as f32 * SWITCH_CELL_SIZE,
            SWITCH_ROWS as f32 * SWITCH_CELL_SIZE,
        );
        let grid_origin = ui.min_rect().min;
        for index in 0..SWITCH_COUNT {
            let column = index % SWITCH_COLUMNS;
            let row = index / SWITCH_COLUMNS;
            let rect = egui::Rect::from_min_size(
                grid_origin + Vec2::new(column as f32 * cell_size.x, row as f32 * cell_size.y),
                cell_size,
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                if ui
                    .push_id(index, |ui| {
                        ui.add(DummyToggle {
                            on: self.switches.get(index),
                        })
                    })
                    .inner
                    .clicked()
                {
                    self.switches.toggle(index);
                }
            });
        }
        ui.advance_cursor_after_rect(egui::Rect::from_min_size(grid_origin, grid_size));
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<SwitchGrid>()
}

#[cfg(test)]
mod tests {
    use crate::selected_switches::SwitchGrid;
    use crate::ui::ScenarioUi;
    use eframe::egui;

    #[test]
    fn switch_state_and_grid_are_populated() {
        let context = egui::Context::default();
        let mut switches = SwitchGrid::new(&context).unwrap();
        switches
            .switches
            .advance(1, benchmark_common::SwitchMode::All);
        let mut output = context.run_ui(egui::RawInput::default(), |ui| switches.render(ui));
        assert!((0..900).all(|index| switches.switches.get(index)));
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
