use crate::ui::{self, Record, ScenarioUi};
use benchmark_common::{FrameState, Harness, Scenario};
use eframe::egui;

struct Startup {
    records: Vec<Record>,
    search: String,
    reset_generation: u64,
}

impl ScenarioUi for Startup {
    const SCENARIO: Scenario = Scenario::Startup;

    fn new(_context: &egui::Context) -> Result<Self, image::ImageError> {
        Ok(Self {
            records: (0..Scenario::Startup.row_count())
                .map(|index| Record::new(index, benchmark_common::row_body(index)))
                .collect(),
            search: String::new(),
            reset_generation: 0,
        })
    }

    fn update(
        &mut self,
        state: FrameState,
        _harness: &mut Harness,
        _context: &egui::Context,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if state.reset_generation != self.reset_generation {
            self.search.clear();
        }
        self.reset_generation = state.reset_generation;
        Ok(state.update_index)
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.heading("Benchmark startup");
            ui.label("Shared deterministic startup content");
            ui.text_edit_singleline(&mut self.search);
            if ui.button("Continue").clicked() {
                self.search.push('!');
            }
            for (index, record) in self.records.iter().enumerate() {
                ui::row(ui, record, index);
            }
        });
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<Startup>()
}
