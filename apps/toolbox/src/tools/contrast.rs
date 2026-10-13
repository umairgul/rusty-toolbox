use super::color_widgets::{ColorInput, swatch, to_color32};
use super::{Category, Tool, status_label};
use egui::{Color32, RichText};
use toolbox_core::color::{self, Color, Level};

pub struct ContrastTool {
    foreground: ColorInput,
    background: ColorInput,
}

impl Default for ContrastTool {
    fn default() -> Self {
        Self {
            foreground: ColorInput::new(Color::from_rgb8(0x93, 0xc5, 0xfd)),
            background: ColorInput::opaque(Color::WHITE),
        }
    }
}

impl ContrastTool {
    fn preview(&self, ui: &mut egui::Ui) {
        let foreground = to_color32(self.foreground.color);
        egui::Frame::NONE
            .fill(to_color32(self.background.color))
            .corner_radius(6.0)
            .inner_margin(16.0)
            .stroke(egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("Large text, 24 px")
                        .size(24.0)
                        .color(foreground),
                );
                ui.label(
                    RichText::new(
                        "Normal body text at 16 px. The quick brown fox jumps over the lazy dog.",
                    )
                    .size(16.0)
                    .color(foreground),
                );
                ui.label(
                    RichText::new("Small print at 12 px")
                        .size(12.0)
                        .color(foreground),
                );
                ui.add_space(6.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(120.0, 30.0), egui::Sense::hover());
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    egui::Stroke::new(2.0, foreground),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Button",
                    egui::FontId::proportional(14.0),
                    foreground,
                );
            });
    }
}

impl Tool for ContrastTool {
    fn name(&self) -> &'static str {
        "Contrast Checker"
    }

    fn description(&self) -> &'static str {
        "Check text and background colors against WCAG 2 accessibility levels, and get passing alternatives."
    }

    fn category(&self) -> Category {
        Category::Design
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("contrast_inputs")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Text");
                self.foreground.ui(ui);
                ui.end_row();
                ui.label("Background");
                self.background.ui(ui);
                ui.end_row();
            });
        if ui.button("Swap colors").clicked() {
            let (text, background) = (self.foreground.color, self.background.color);
            self.foreground.set(background);
            self.background.set(text.over(background));
        }
        ui.add_space(8.0);
        self.preview(ui);
        ui.add_space(12.0);

        let (text, background) = (self.foreground.color, self.background.color);
        let ratio = color::contrast_ratio(text, background);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:.2} : 1", ratio))
                    .size(28.0)
                    .strong(),
            );
            ui.add_space(8.0);
            let verdict = if Level::AaaNormal.passes(ratio) {
                "Excellent: passes every level"
            } else if Level::AaNormal.passes(ratio) {
                "Good: passes AA for all text"
            } else if Level::AaLarge.passes(ratio) {
                "Only for large text and UI components"
            } else {
                "Fails: too little contrast for text"
            };
            ui.label(RichText::new(verdict).size(15.0));
        });
        ui.add_space(4.0);

        egui::Grid::new("contrast_levels")
            .num_columns(3)
            .striped(true)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for level in Level::ALL {
                    ui.label(level.label());
                    ui.weak(format!("{}:1", level.required_ratio()));
                    status_label(
                        ui,
                        level.passes(ratio),
                        if level.passes(ratio) { "Pass" } else { "Fail" },
                    );
                    ui.end_row();
                }
            });

        let mut replacement = None;
        for (level, target) in [(Level::AaNormal, 4.5), (Level::AaaNormal, 7.0)] {
            if level.passes(ratio) {
                continue;
            }
            ui.add_space(8.0);
            match color::suggest_for_contrast(text, background, target) {
                Some(suggestion) => {
                    ui.horizontal(|ui| {
                        swatch(ui, suggestion, egui::vec2(28.0, 20.0));
                        ui.label(format!(
                            "{} passes {} ({:.2}:1), keeping the same hue.",
                            suggestion.hex(),
                            level.label(),
                            color::contrast_ratio(suggestion, background)
                        ));
                        if ui.button("Use it").clicked() {
                            replacement = Some(suggestion);
                        }
                    });
                }
                None => {
                    ui.label(
                        RichText::new(format!(
                            "No shade of this hue reaches {target}:1 on this background."
                        ))
                        .color(Color32::GRAY),
                    );
                }
            }
        }
        if let Some(suggestion) = replacement {
            self.foreground.set(suggestion);
        }
    }
}
