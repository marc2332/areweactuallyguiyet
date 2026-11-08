use crate::ui::{self, Record, ScenarioUi};
use benchmark_common::{FrameState, Harness, ROW_HEIGHT, Scenario};
use eframe::egui;

struct VirtualScroll {
    records: Vec<Record>,
    state: FrameState,
    controlled: bool,
}

impl ScenarioUi for VirtualScroll {
    const SCENARIO: Scenario = Scenario::VirtualScroll;

    fn new(_context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self {
            records: (0..Scenario::VirtualScroll.row_count())
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

impl VirtualScroll {
    fn list(&self, ui: &mut egui::Ui) -> usize {
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink([false, false]);
        if self.controlled {
            scroll = scroll.vertical_scroll_offset(self.state.scroll_offset);
        }
        scroll
            .show_rows(ui, ROW_HEIGHT, self.records.len(), |ui, range| {
                let rendered = range.len();
                for index in range {
                    ui::row(ui, &self.records[index], index);
                }
                rendered
            })
            .inner
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<VirtualScroll>()
}

#[cfg(test)]
mod tests {
    use crate::selected_virtual_scroll::VirtualScroll;
    use crate::ui::ScenarioUi;
    use eframe::egui::{self, Rect, Vec2};

    #[test]
    fn virtualized_list_renders_only_visible_rows() {
        let context = egui::Context::default();
        let list = VirtualScroll::new(&context).unwrap();
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
        assert_eq!(list.records.len(), 100_000);
        assert!(rendered > 0 && rendered < 100);
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
