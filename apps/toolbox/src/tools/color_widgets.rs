//! Widgets shared by the color tools: a color input (picker, text and eyedropper) and swatches.

use super::error_label;
use crate::eyedropper::{self, Eyedropper};
use egui::color_picker::{Alpha, color_edit_button_srgba, show_color_at};
use egui::{Color32, RichText};
use std::time::Duration;
use toolbox_core::color::{self, Color};

pub fn to_color32(color: Color) -> Color32 {
    let [r, g, b, a] = color.to_rgba8();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn from_color32(color: Color32) -> Color {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    Color::from_rgba8(r, g, b, a)
}

/// A color with a swatch button that opens a picker, a text field taking any CSS color, and
/// an eyedropper button where supported.
pub struct ColorInput {
    pub color: Color,
    text: String,
    error: Option<String>,
    eyedropper: Eyedropper,
    alpha: bool,
}

impl ColorInput {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            text: color.hex(),
            error: None,
            eyedropper: Eyedropper::default(),
            alpha: true,
        }
    }

    /// For backgrounds, which must be opaque.
    pub fn opaque(color: Color) -> Self {
        Self {
            alpha: false,
            ..Self::new(color)
        }
    }

    pub fn set(&mut self, color: Color) {
        self.color = if self.alpha {
            color
        } else {
            color.with_alpha(1.0)
        };
        self.text = self.color.hex();
        self.error = None;
    }

    /// Returns true if the color changed.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            let mut swatch = to_color32(self.color);
            let alpha = if self.alpha {
                Alpha::OnlyBlend
            } else {
                Alpha::Opaque
            };
            if color_edit_button_srgba(ui, &mut swatch, alpha).changed() {
                self.set(from_color32(swatch));
                changed = true;
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(220.0)
                    .hint_text("#3b82f6, rgb(…), hsl(…), tomato"),
            );
            if field.changed() {
                match color::parse(&self.text) {
                    Ok(parsed) => {
                        self.color = if self.alpha {
                            parsed
                        } else {
                            parsed.with_alpha(1.0)
                        };
                        self.error = None;
                        changed = true;
                    }
                    Err(error) => self.error = Some(error.0),
                }
            }
            if eyedropper::SUPPORTED {
                let label = if self.eyedropper.is_active() {
                    "Click anywhere on screen…"
                } else {
                    "Pick from screen"
                };
                if ui
                    .add_enabled(!self.eyedropper.is_active(), egui::Button::new(label))
                    .clicked()
                {
                    self.eyedropper.start(ui.ctx());
                }
            }
        });
        if let Some(picked) = self.eyedropper.poll() {
            self.set(picked);
            changed = true;
        }
        if let Some(error) = &self.error {
            error_label(ui, error);
        }
        changed
    }

    /// Whether an eyedropper session is running.
    pub fn eyedropper_active(&self) -> bool {
        self.eyedropper.is_active()
    }
}

/// A plain swatch, with a checkerboard behind translucent colors.
pub fn swatch(ui: &mut egui::Ui, color: Color, size: egui::Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    show_color_at(ui.painter(), to_color32(color), rect);
    let stroke = egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
    ui.painter()
        .rect_stroke(rect, 3.0, stroke, egui::StrokeKind::Inside);
    response
}

/// A swatch with a caption below that copies its hex code when clicked.
pub fn copy_swatch(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    color: Color,
    size: egui::Vec2,
    caption: &str,
) {
    let id = ui.make_persistent_id(id_salt);
    let now = ui.input(|input| input.time);
    let copied_at: Option<f64> = ui.ctx().data(|data| data.get_temp(id));
    let recently = copied_at.is_some_and(|at| now - at < 1.5);
    ui.vertical(|ui| {
        ui.set_width(size.x);
        let hex = color.hex();
        let response = swatch(ui, color, size).on_hover_text(format!("Click to copy {hex}"));
        if response.clicked() {
            ui.ctx().copy_text(hex.clone());
            ui.ctx().data_mut(|data| data.insert_temp(id, now));
            ui.ctx().request_repaint_after(Duration::from_millis(1500));
        }
        if !caption.is_empty() {
            ui.label(RichText::new(caption).small().strong());
        }
        ui.label(
            RichText::new(if recently { "Copied!" } else { &hex })
                .small()
                .monospace(),
        );
    });
}

/// A row of equally wide copy swatches that fills the available width.
pub fn swatch_row(ui: &mut egui::Ui, id_salt: &str, colors: &[(String, Color)], height: f32) {
    if colors.is_empty() {
        return;
    }
    let spacing = ui.spacing().item_spacing.x;
    let width = ((ui.available_width() - spacing * (colors.len() - 1) as f32)
        / colors.len() as f32)
        .clamp(36.0, 110.0);
    ui.horizontal_top(|ui| {
        for (index, (caption, color)) in colors.iter().enumerate() {
            copy_swatch(
                ui,
                (id_salt, index),
                *color,
                egui::vec2(width, height),
                caption,
            );
        }
    });
}
