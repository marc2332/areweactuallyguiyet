use benchmark_common::{
    FrameState, HEIGHT, Harness, ROW_HEIGHT, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT,
    SWITCH_FONT_SIZE, SWITCH_OFF_COLOR, SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario,
    Switches, WIDTH,
    dashboard::{Dashboard, Fixtures, THUMBNAILS},
    row_body, row_title,
};
use eframe::egui::{
    self, FontData, FontDefinitions, FontFamily, Response, Sense, TextureHandle, Vec2, Widget,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

type Error = Box<dyn std::error::Error>;

struct Record {
    title: String,
    body: String,
}

struct DummyToggle {
    on: bool,
}

impl Widget for DummyToggle {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let size = ui.available_size_before_wrap();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let (red, green, blue) = if self.on {
            SWITCH_ON_COLOR
        } else {
            SWITCH_OFF_COLOR
        };
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(red, green, blue));
        let (red, green, blue) = SWITCH_TEXT_COLOR;
        let text = if self.on { "ON" } else { "OFF" };
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(SWITCH_FONT_SIZE),
            egui::Color32::from_rgb(red, green, blue),
        );
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            egui::Color32::WHITE,
        );
        response
    }
}

struct BenchmarkApp {
    scenario: Scenario,
    harness: Harness,
    state: FrameState,
    switches: Switches,
    search: String,
    records: Vec<Record>,
    dashboard: Option<Dashboard>,
    thumbnails: Vec<TextureHandle>,
    failure: Rc<RefCell<Option<String>>>,
    built_once: bool,
    reset_records_scroll: bool,
}

impl BenchmarkApp {
    fn new(
        scenario: Scenario,
        harness: Harness,
        context: &egui::Context,
        failure: Rc<RefCell<Option<String>>>,
    ) -> Result<Self, image::ImageError> {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "benchmark".into(),
            FontData::from_static(include_bytes!("../../../assets/fonts/DejaVuSans.ttf")).into(),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "benchmark".into());
        context.set_fonts(fonts);
        context.all_styles_mut(|style| {
            style.animation_time = 0.0;
            style.spacing.item_spacing.y = 0.0;
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        });
        let records = (0..if scenario == Scenario::Memory {
            0
        } else {
            scenario.row_count()
        })
            .map(|index| Record {
                title: row_title(index),
                body: if scenario == Scenario::Memory {
                    format!("{} {}", row_body(index), row_body(index))
                        .chars()
                        .take(160)
                        .collect()
                } else {
                    row_body(index)
                },
            })
            .collect();
        let (dashboard, thumbnails) = if scenario == Scenario::Memory {
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
            (Some(dashboard), thumbnails)
        } else {
            (None, Vec::new())
        };
        Ok(Self {
            scenario,
            harness,
            state: FrameState::default(),
            switches: Switches::default(),
            search: String::new(),
            records,
            dashboard,
            thumbnails,
            failure,
            built_once: false,
            reset_records_scroll: false,
        })
    }

    fn fail(&mut self, context: &egui::Context, error: impl ToString) {
        *self.failure.borrow_mut() = Some(error.to_string());
        context.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn row(ui: &mut egui::Ui, record: &Record, index: usize) {
        ui.push_id(index, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), ROW_HEIGHT),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_height(ROW_HEIGHT);
                    ui.add(egui::Label::new(&record.title).truncate());
                    ui.add(egui::Label::new(&record.body).truncate());
                },
            );
        });
    }

    fn list(&self, ui: &mut egui::Ui, virtualized: bool) -> usize {
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink([false, false]);
        if self.harness.controlled() {
            scroll = scroll.vertical_scroll_offset(self.state.scroll_offset);
        }
        if virtualized {
            scroll
                .show_rows(ui, ROW_HEIGHT, self.records.len(), |ui, range| {
                    let rendered = range.len();
                    for index in range {
                        Self::row(ui, &self.records[index], index);
                    }
                    rendered
                })
                .inner
        } else {
            scroll.show(ui, |ui| {
                for (index, record) in self.records.iter().enumerate() {
                    Self::row(ui, record, index);
                }
            });
            self.records.len()
        }
    }

    fn switch_grid(&mut self, ui: &mut egui::Ui) {
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

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let scripted_scroll =
            (self.harness.controlled() && self.state.running) || self.reset_records_scroll;
        let Some(dashboard) = self.dashboard.as_mut() else {
            self.fail(ui.ctx(), "Memory dashboard was not initialized");
            return;
        };
        ui.horizontal(|ui| {
            ui.set_height(benchmark_common::dashboard::HEADER_HEIGHT);
            ui.heading("Dashboard");
            let response = ui.add_sized(
                [430.0, 30.0],
                egui::TextEdit::singleline(&mut dashboard.query).hint_text("Search records"),
            );
            if response.changed() {
                dashboard.search(dashboard.query.clone());
            }
            ui.label(format!(
                "{} visible · selected {}",
                dashboard.visible_records().len(),
                dashboard.selected_record
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
                                    .selectable_label(dashboard.section == section, title)
                                    .clicked()
                                {
                                    dashboard.navigate(section);
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
                    let visible = dashboard.visible_records();
                    let mut records = egui::ScrollArea::vertical()
                        .id_salt("records")
                        .auto_shrink([false, false]);
                    if scripted_scroll {
                        records = records.vertical_scroll_offset(dashboard.scroll_offset);
                    }
                    records.show(ui, |ui| {
                        for index in visible {
                            let record = &dashboard.fixtures.records[index];
                            let title = record.title.clone();
                            let body = record.body.clone();
                            let active = dashboard.active.get(index);
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
                                            .selected(dashboard.selected_record == index),
                                        );
                                        if selected.clicked() {
                                            dashboard.select(index);
                                        }
                                        if ui
                                            .add_sized([64.0, 24.0], DummyToggle { on: active })
                                            .clicked()
                                        {
                                            dashboard.toggle(index);
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
                                &dashboard.fixtures.records[dashboard.selected_record].title,
                            );
                            ui.label(dashboard.detail.as_ref());
                        });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("dashboard-fields")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for index in 0..dashboard.fields.len() {
                                let mut value = dashboard.fields[index].clone();
                                let changed = ui
                                    .push_id(("dashboard-field", index), |ui| {
                                        ui.label(format!("Field {:02}", index + 1));
                                        ui.text_edit_singleline(&mut value).changed()
                                    })
                                    .inner;
                                if changed {
                                    dashboard.edit(index, value);
                                }
                            }
                        });
                },
            );
        });
        self.reset_records_scroll = false;
    }
}

impl eframe::App for BenchmarkApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        match self.harness.poll() {
            Ok(state) => {
                if state.reset_generation != self.state.reset_generation {
                    self.switches = Switches::default();
                    self.search.clear();
                    if let Some(dashboard) = &mut self.dashboard {
                        dashboard.reset();
                        self.reset_records_scroll = true;
                    }
                }
                self.state = state;
            }
            Err(error) => {
                self.fail(context, error);
                return;
            }
        }
        if self.state.should_quit {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.scenario == Scenario::Switches {
            self.switches.advance_changed(
                self.state.update_index,
                self.harness.switch_mode(),
                |_, _| {},
            );
        }
        if self.scenario == Scenario::Memory {
            if let Some(dashboard) = &mut self.dashboard {
                if self.state.running {
                    dashboard.advance(self.state);
                }
                if let Err(error) = self.harness.report_dashboard(dashboard) {
                    self.fail(context, error);
                }
            }
        }
        let applied = if self.scenario == Scenario::Switches {
            self.switches.applied_updates()
        } else if self.scenario == Scenario::Memory {
            self.dashboard
                .as_ref()
                .map_or(0, Dashboard::applied_updates)
        } else {
            self.state.update_index
        };
        if let Err(error) = self.harness.report_applied(applied) {
            self.fail(context, error);
        }
        if self.state.running {
            context.request_repaint();
        } else if self.harness.controlled() {
            context.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.scenario == Scenario::Switches {
            self.switch_grid(ui);
        } else {
            egui::Frame::central_panel(ui.style()).show(ui, |ui| match self.scenario {
                Scenario::Scroll => {
                    self.list(ui, false);
                }
                Scenario::VirtualScroll => {
                    self.list(ui, true);
                }
                Scenario::Switches => unreachable!(),
                Scenario::Memory => self.dashboard(ui),
                Scenario::Startup => {
                    ui.heading("Benchmark startup");
                    ui.label("Shared deterministic startup content");
                    ui.text_edit_singleline(&mut self.search);
                    if ui.button("Continue").clicked() {
                        self.search.push('!');
                    }
                    for (index, record) in self.records.iter().enumerate() {
                        Self::row(ui, record, index);
                    }
                }
            });
        }
        if !self.built_once {
            self.built_once = true;
            if let Err(error) = self.harness.mark_ready("ui-built-not-presented") {
                self.fail(ui.ctx(), error);
            }
        }
    }
}

pub fn run(scenario: Scenario) -> Result<(), Error> {
    let harness = Harness::connect(scenario)?;
    let failure = Rc::new(RefCell::new(None));
    let app_failure = failure.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("GUI benchmark: egui / {}", scenario.id()))
            .with_inner_size([WIDTH, HEIGHT]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Rust GUI benchmark",
        options,
        Box::new(move |creation| {
            Ok(Box::new(BenchmarkApp::new(
                scenario,
                harness,
                &creation.egui_ctx,
                app_failure,
            )?))
        }),
    )?;
    if let Some(message) = failure.borrow_mut().take() {
        return Err(message.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ui::BenchmarkApp;
    use benchmark_common::{Harness, Scenario, SwitchMode};
    use eframe::egui::{self, Rect, Vec2};
    use std::{cell::RefCell, rc::Rc};

    fn app(scenario: Scenario, context: &egui::Context) -> BenchmarkApp {
        BenchmarkApp::new(
            scenario,
            Harness::connect(scenario).unwrap(),
            context,
            Rc::new(RefCell::new(None)),
        )
        .unwrap()
    }

    #[test]
    fn lists_execute_all_rows_only_in_the_nonvirtualized_variant() {
        let context = egui::Context::default();
        let nonvirtualized = app(Scenario::Scroll, &context);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let mut rendered = 0;
        let mut output = context.run_ui(input.clone(), |ui| {
            rendered = nonvirtualized.list(ui, false);
        });
        assert_eq!(rendered, 2_000);
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
        let virtualized = app(Scenario::VirtualScroll, &context);
        let mut output = context.run_ui(input, |ui| {
            rendered = virtualized.list(ui, true);
        });
        output.textures_delta.clear();
        assert_eq!(virtualized.records.len(), 100_000);
        assert!(rendered > 0 && rendered < 100);
    }

    #[test]
    fn dashboard_fixtures_and_switch_state_are_populated() {
        let context = egui::Context::default();
        let dashboard = app(Scenario::Memory, &context);
        let model = dashboard.dashboard.as_ref().unwrap();
        assert_eq!(model.fixtures.records.len(), 200);
        assert_eq!(model.fixtures.records[0].body.chars().count(), 160);
        assert_eq!(model.detail.chars().count(), 10_000);
        assert_eq!(model.fields.len(), 20);
        assert_eq!(dashboard.thumbnails.len(), 10);
        let mut switches = app(Scenario::Switches, &context);
        switches.switches.advance(1, SwitchMode::All);
        let mut output = context.run_ui(egui::RawInput::default(), |ui| switches.switch_grid(ui));
        assert!((0..900).all(|index| switches.switches.get(index)));
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
