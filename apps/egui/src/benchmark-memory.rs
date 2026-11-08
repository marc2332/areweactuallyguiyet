use crate::ui::{self, DummyToggle, ScenarioUi};
use benchmark_common::dashboard::{Dashboard, Fixtures, THUMBNAILS};
use benchmark_common::{FrameState, Harness, Scenario};
use eframe::egui::{self, TextureHandle, Vec2};

struct Memory {
    dashboard: Dashboard,
    thumbnails: Vec<TextureHandle>,
    reset_records_scroll: bool,
    state: FrameState,
    controlled: bool,
}

impl ScenarioUi for Memory {
    const SCENARIO: Scenario = Scenario::Memory;

    fn new(context: &egui::Context) -> Result<Self, image::ImageError> {
        let dashboard = Dashboard::new(std::sync::Arc::new(Fixtures::default()));
        let thumbnails = THUMBNAILS
            .iter()
            .enumerate()
            .map(|(index, bytes)| {
                let decoded = image::load_from_memory(bytes)?.to_rgba8();
                Ok(context.load_texture(
                    format!("dashboard-thumbnail-{index}"),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [decoded.width() as usize, decoded.height() as usize],
                        decoded.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                ))
            })
            .collect::<Result<Vec<_>, image::ImageError>>()?;
        Ok(Self {
            dashboard,
            thumbnails,
            reset_records_scroll: false,
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
        if state.reset_generation != self.state.reset_generation {
            self.dashboard.reset();
            self.reset_records_scroll = true;
        }
        self.state = state;
        self.controlled = harness.controlled();
        if state.running {
            self.dashboard.advance(state);
        }
        harness.report_dashboard(&self.dashboard)?;
        Ok(self.dashboard.applied_updates())
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let scripted_scroll = (self.controlled && self.state.running) || self.reset_records_scroll;
        ui.horizontal(|ui| {
            ui.set_height(benchmark_common::dashboard::HEADER_HEIGHT);
            ui.heading("Dashboard");
            let response = ui.add_sized(
                [430.0, 30.0],
                egui::TextEdit::singleline(&mut self.dashboard.query).hint_text("Search records"),
            );
            if response.changed() {
                self.dashboard.search(self.dashboard.query.clone());
            }
            ui.label(format!(
                "{} visible · selected {}",
                self.dashboard.visible_records().len(),
                self.dashboard.selected_record
            ));
        });
        ui.separator();
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(180.0, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("sections-and-thumbnails")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for section in 0..benchmark_common::dashboard::SECTION_COUNT {
                                let title = if section == 0 {
                                    "All records".to_owned()
                                } else {
                                    format!("Section {section}")
                                };
                                if ui
                                    .selectable_label(self.dashboard.section == section, title)
                                    .clicked()
                                {
                                    self.dashboard.navigate(section);
                                }
                            }
                            ui.add_space(8.0);
                            egui::Grid::new("dashboard-thumbnails")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for (index, texture) in self.thumbnails.iter().enumerate() {
                                        ui.image((texture.id(), Vec2::splat(48.0)));
                                        if index % 2 == 1 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(600.0, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let visible = self.dashboard.visible_records();
                    let mut records = egui::ScrollArea::vertical()
                        .id_salt("records")
                        .auto_shrink([false, false]);
                    if scripted_scroll {
                        records = records.vertical_scroll_offset(self.dashboard.scroll_offset);
                    }
                    records.show(ui, |ui| {
                        for index in visible {
                            let record = &self.dashboard.fixtures.records[index];
                            let title = record.title.clone();
                            let body = record.body.clone();
                            let active = self.dashboard.active.get(index);
                            ui.push_id(index, |ui| {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(600.0, 64.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let selected = ui.add_sized(
                                            [520.0, 64.0],
                                            egui::Button::new(
                                                egui::RichText::new(format!("{}\n{}", title, body))
                                                    .size(12.0),
                                            )
                                            .wrap()
                                            .selected(self.dashboard.selected_record == index),
                                        );
                                        if selected.clicked() {
                                            self.dashboard.select(index);
                                        }
                                        if ui
                                            .add_sized([64.0, 24.0], DummyToggle { on: active })
                                            .clicked()
                                        {
                                            self.dashboard.toggle(index);
                                        }
                                    },
                                );
                            });
                        }
                    });
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("detail")
                        .max_height(300.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading(
                                &self.dashboard.fixtures.records[self.dashboard.selected_record]
                                    .title,
                            );
                            ui.label(self.dashboard.detail.as_ref());
                        });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("dashboard-fields")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for index in 0..self.dashboard.fields.len() {
                                let mut value = self.dashboard.fields[index].clone();
                                let changed = ui
                                    .push_id(("dashboard-field", index), |ui| {
                                        ui.label(format!("Field {:02}", index + 1));
                                        ui.text_edit_singleline(&mut value).changed()
                                    })
                                    .inner;
                                if changed {
                                    self.dashboard.edit(index, value);
                                }
                            }
                        });
                },
            );
        });
        self.reset_records_scroll = false;
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    ui::run::<Memory>()
}

#[cfg(test)]
mod tests {
    use crate::selected_memory::Memory;
    use crate::ui::ScenarioUi;
    use eframe::egui;

    #[test]
    fn dashboard_fixtures_are_populated() {
        let context = egui::Context::default();
        let dashboard = Memory::new(&context).unwrap();
        assert_eq!(dashboard.dashboard.fixtures.records.len(), 200);
        assert_eq!(
            dashboard.dashboard.fixtures.records[0].body.chars().count(),
            160
        );
        assert_eq!(dashboard.dashboard.detail.chars().count(), 10_000);
        assert_eq!(dashboard.dashboard.fields.len(), 20);
        assert_eq!(dashboard.thumbnails.len(), 10);
    }
}
