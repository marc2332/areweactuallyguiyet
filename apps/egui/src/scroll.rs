use eframe::egui;

struct ScrollApp;

impl eframe::App for ScrollApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::NONE.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for index in 0..2_000 {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(960.0, 48.0), egui::Sense::hover());
                        let background = if index % 2 == 0 { 245 } else { 230 };
                        ui.painter()
                            .rect_filled(rect, 0.0, egui::Color32::from_gray(background));
                        ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect.shrink(12.0))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        )
                        .add(
                            egui::Label::new(
                                egui::RichText::new(format!("Item {}", index + 1))
                                    .size(14.0)
                                    .color(egui::Color32::BLACK),
                            )
                            .halign(egui::Align::Min)
                            .truncate(),
                        );
                    }
                });
        });
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
            Ok(Box::new(ScrollApp))
        }),
    )
}
