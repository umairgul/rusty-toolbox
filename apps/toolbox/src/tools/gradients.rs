use super::color_widgets::{ColorInput, swatch_row, to_color32};
use super::{Category, Tool, ValueRow, value_rows};
use egui::RichText;
use toolbox_core::color::{self, Color, Space};

const MIN_STOPS: usize = 2;
const MAX_STOPS: usize = 6;
/// Vertical strips drawn in the preview.
const PREVIEW_STRIPS: usize = 180;
/// Sampled stops in the all-browsers CSS.
const FALLBACK_SAMPLES: usize = 9;

pub struct GradientsTool {
    stops: Vec<ColorInput>,
    space: Space,
    angle: f64,
    steps: usize,
}

impl Default for GradientsTool {
    fn default() -> Self {
        Self {
            stops: vec![
                ColorInput::new(Color::from_rgb8(0xff, 0x5f, 0x6d)),
                ColorInput::new(Color::from_rgb8(0x36, 0x5b, 0xff)),
            ],
            space: Space::Oklab,
            angle: 90.0,
            steps: 7,
        }
    }
}

impl Tool for GradientsTool {
    fn name(&self) -> &'static str {
        "Gradients"
    }

    fn description(&self) -> &'static str {
        "Blend colors in OKLab, OKLCH or sRGB, and copy the CSS or the in-between steps."
    }

    fn category(&self) -> Category {
        Category::Design
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut remove = None;
        let can_remove = self.stops.len() > MIN_STOPS;
        for (index, stop) in self.stops.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("Stop {}", index + 1));
                stop.ui(ui);
                if can_remove
                    && ui
                        .small_button("×")
                        .on_hover_text("Remove this stop")
                        .clicked()
                {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            self.stops.remove(index);
        }
        if self.stops.len() < MAX_STOPS && ui.button("Add stop").clicked() {
            let last = self.stops.last().map_or(Color::WHITE, |stop| stop.color);
            self.stops.push(ColorInput::new(last));
        }
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Blend in");
            egui::ComboBox::from_id_salt("gradient_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    for space in Space::ALL {
                        ui.selectable_value(&mut self.space, space, space.label());
                    }
                });
            ui.add_space(12.0);
            ui.add(
                egui::Slider::new(&mut self.angle, 0.0..=360.0)
                    .step_by(1.0)
                    .suffix("°")
                    .text("Angle"),
            );
        });
        ui.add_space(8.0);

        let stops: Vec<Color> = self.stops.iter().map(|stop| stop.color).collect();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::hover());
        let strip = rect.width() / PREVIEW_STRIPS as f32;
        for i in 0..PREVIEW_STRIPS {
            let t = (i as f64 + 0.5) / PREVIEW_STRIPS as f64;
            let x = rect.left() + i as f32 * strip;
            let strip_rect = egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + strip + 0.5, rect.bottom()),
            );
            ui.painter().rect_filled(
                strip_rect,
                0.0,
                to_color32(color::sample(&stops, t, self.space)),
            );
        }
        ui.weak("Preview runs left to right; the angle applies to the CSS.");
        ui.add_space(8.0);

        let css = color::css_gradient(&stops, self.angle, self.space);
        let fallback =
            color::css_gradient_fallback(&stops, self.angle, self.space, FALLBACK_SAMPLES);
        value_rows(
            ui,
            "gradient_css",
            &[
                ValueRow::new("CSS (modern browsers)", css),
                ValueRow::new("CSS (every browser)", fallback),
            ],
        );
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Steps").strong());
            ui.add(egui::Slider::new(&mut self.steps, 2..=16));
        });
        let colors: Vec<(String, Color)> = color::steps(&stops, self.steps, self.space)
            .into_iter()
            .enumerate()
            .map(|(i, c)| ((i + 1).to_string(), c))
            .collect();
        swatch_row(ui, "gradient_steps", &colors, 40.0);
    }
}
