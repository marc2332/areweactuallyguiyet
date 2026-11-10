use eframe::egui;

struct Dashboard {
    notes: String,
    refreshes: usize,
    enabled: bool,
    images: Vec<egui::TextureHandle>,
}

impl eframe::App for Dashboard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let bounds = ui.max_rect();
        ui.painter().rect_filled(bounds, 0.0, egui::Color32::WHITE);
        let origin = bounds.min;
        let rect = |x, y, width, height| {
            egui::Rect::from_min_size(origin + egui::vec2(x, y), egui::vec2(width, height))
        };
        ui.put(
            rect(24.0, 16.0, 400.0, 40.0),
            egui::Label::new(egui::RichText::new("Project dashboard").size(24.0)),
        );
        ui.put(
            rect(24.0, 70.0, 360.0, 40.0),
            egui::TextEdit::singleline(&mut self.notes),
        );
        if ui
            .put(rect(416.0, 70.0, 160.0, 40.0), egui::Button::new("Refresh"))
            .clicked()
        {
            self.refreshes += 1;
        }
        if ui
            .put(
                rect(624.0, 70.0, 160.0, 40.0),
                egui::Button::new(if self.enabled { "Enabled" } else { "Disabled" }),
            )
            .clicked()
        {
            self.enabled = !self.enabled;
        }
        ui.put(
            rect(24.0, 124.0, 360.0, 24.0),
            egui::Label::new(format!("Refresh count: {}", self.refreshes)),
        );

        let list_rect = egui::Rect::from_min_max(origin + egui::vec2(0.0, 180.0), bounds.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(list_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for index in 0..100 {
                            ui.horizontal(|ui| {
                                let image = &self.images[index % self.images.len()];
                                ui.add(egui::Image::new((image.id(), egui::vec2(64.0, 64.0))));
                                ui.vertical(|ui| {
                                    ui.label(format!("Project {}", index + 1));
                                    ui.label(
                                        "A project record with descriptive dashboard information.",
                                    );
                                });
                            });
                            ui.separator();
                        }
                    });
            },
        );
    }
}

pub fn main() -> eframe::Result {
    eframe::run_native(
        "egui Scroll 2000",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("egui Scroll 2000")
                .with_inner_size([1000.0, 700.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|creation| {
            creation.egui_ctx.set_visuals(egui::Visuals::light());
            let image_bytes = [
                include_bytes!("../../../assets/images/dashboard-0.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-1.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-2.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-3.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-4.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-5.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-6.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-7.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-8.png").as_slice(),
                include_bytes!("../../../assets/images/dashboard-9.png").as_slice(),
            ];
            let images = image_bytes
                .into_iter()
                .enumerate()
                .map(|(index, bytes)| {
                    let decoded = image::load_from_memory(bytes)?.to_rgba8();
                    let size = [decoded.width() as usize, decoded.height() as usize];
                    Ok(creation.egui_ctx.load_texture(
                        format!("dashboard-{index}"),
                        egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw()),
                        Default::default(),
                    ))
                })
                .collect::<Result<Vec<_>, image::ImageError>>()?;
            Ok(Box::new(Dashboard {
                notes: "Project notes".into(),
                refreshes: 0,
                enabled: false,
                images,
            }))
        }),
    )
}
