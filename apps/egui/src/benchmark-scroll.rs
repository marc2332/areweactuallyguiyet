use crate::ui::{self, Record, ScenarioUi};
use benchmark_common::{FrameState, Harness, Scenario};
use eframe::egui;

struct Scroll {
    records: Vec<Record>,
    state: FrameState,
    controlled: bool,
}

impl ScenarioUi for Scroll {
    const SCENARIO: Scenario = Scenario::Scroll;

    fn new(_context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self {
            records: (0..Scenario::Scroll.row_count())
                .map(|index| Record::new(index, benchmark_common::row_body(index)))
                .collect(),
            state: FrameState::default(),
            controlled: false,
        })
    }

    fn update(
        &mut self,
        state: FrameState,
        harness: &mut Harness,
        _context: &egui::Context,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.state = state;
        self.controlled = harness.controlled();
        Ok(state.update_index)
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            self.list(ui);
        });
    }
}

impl Scroll {
    fn list(&self, ui: &mut egui::Ui) -> usize {
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink([false, false]);
        if self.controlled {
            scroll = scroll.vertical_scroll_offset(self.state.scroll_offset);
        }
        scroll.show(ui, |ui| {
            for (index, record) in self.records.iter().enumerate() {
                ui::row(ui, record, index);
            }
        });
        self.records.len()
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<Scroll>()
}

#[cfg(test)]
mod tests {
    use crate::selected_scroll::Scroll;
    use crate::ui::ScenarioUi;
    use eframe::egui::{self, Rect, Vec2};

    #[test]
    fn nonvirtualized_list_executes_every_row() {
        let context = egui::Context::default();
        let list = Scroll::new(&context).unwrap();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let mut rendered = 0;
        let mut output = context.run_ui(input, |ui| {
            rendered = list.list(ui);
        });
        assert_eq!(rendered, 2_000);
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
