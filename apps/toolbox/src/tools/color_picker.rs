use super::color_widgets::{ColorInput, from_color32, swatch, to_color32};
use super::{Category, Tool, ValueRow, value_rows};
use egui::RichText;
use egui::color_picker::{Alpha, color_picker_color32};
use toolbox_core::color::{self, Color, Format};

const MAX_RECENT: usize = 16;

pub struct ColorPickerTool {
    input: ColorInput,
    recent: Vec<Color>,
    /// Whether the eyedropper was running last frame, to save its pick to the history.
    sampling: bool,
}

impl Default for ColorPickerTool {
    fn default() -> Self {
        Self {
            input: ColorInput::new(Color::from_rgb8(0x3b, 0x82, 0xf6)),
            recent: Vec::new(),
            sampling: false,
        }
    }
}

impl ColorPickerTool {
    fn remember(&mut self, color: Color) {
        self.recent.retain(|c| c.hex() != color.hex());
        self.recent.insert(0, color);
        self.recent.truncate(MAX_RECENT);
    }
}

impl Tool for ColorPickerTool {
    fn name(&self) -> &'static str {
        "Color Picker"
    }

    fn description(&self) -> &'static str {
        "Pick a color anywhere on screen, or enter any CSS color, and get it in every format."
    }

    fn category(&self) -> Category {
        Category::Design
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.input.ui(ui);
        if self.sampling && !self.input.eyedropper_active() {
            self.remember(self.input.color);
        }
        self.sampling = self.input.eyedropper_active();
        ui.add_space(8.0);

        let color = self.input.color;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().slider_width = 220.0;
                let mut picked = to_color32(color);
                if color_picker_color32(ui, &mut picked, Alpha::OnlyBlend) {
                    self.input.set(from_color32(picked));
                }
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    swatch(ui, color, egui::vec2(96.0, 56.0));
                    ui.vertical(|ui| {
                        let (name, distance) = color::nearest_name(color.with_alpha(1.0));
                        if distance == 0.0 {
                            ui.label(RichText::new(name).strong().size(16.0));
                            ui.weak("CSS named color");
                        } else {
                            ui.label(
                                RichText::new(format!("Close to {name}"))
                                    .strong()
                                    .size(16.0),
                            );
                            ui.weak("nearest CSS named color");
                        }
                        if ui.button("Save to recent").clicked() {
                            self.remember(color);
                        }
                    });
                });
                ui.add_space(8.0);
                let rows: Vec<ValueRow> = Format::ALL
                    .iter()
                    .map(|f| ValueRow::new(f.label(), color.format(*f)))
                    .collect();
                value_rows(ui, "color_formats", &rows);
            });
        });

        if !self.recent.is_empty() {
            ui.add_space(12.0);
            ui.label(RichText::new("Recent").strong());
            let mut chosen = None;
            ui.horizontal_wrapped(|ui| {
                for recent in &self.recent {
                    if swatch(ui, *recent, egui::vec2(32.0, 32.0))
                        .on_hover_text(recent.hex())
                        .clicked()
                    {
                        chosen = Some(*recent);
                    }
                }
            });
            if let Some(recent) = chosen {
                self.input.set(recent);
            }
        }
    }
}
