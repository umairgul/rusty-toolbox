use super::color_widgets::{ColorInput, swatch_row};
use super::{Category, Tool, copy_button, output_box};
use egui::RichText;
use toolbox_core::color::{self, Color, Harmony};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Export {
    CssVariables,
    Tailwind,
    Json,
}

impl Export {
    const ALL: [Export; 3] = [Export::CssVariables, Export::Tailwind, Export::Json];

    fn label(self) -> &'static str {
        match self {
            Export::CssVariables => "CSS variables",
            Export::Tailwind => "Tailwind v4",
            Export::Json => "JSON",
        }
    }
}

pub struct PalettesTool {
    base: ColorInput,
    name: String,
    export: Export,
}

impl Default for PalettesTool {
    fn default() -> Self {
        Self {
            base: ColorInput::new(Color::from_rgb8(0x3b, 0x82, 0xf6)),
            name: "brand".into(),
            export: Export::CssVariables,
        }
    }
}

impl Tool for PalettesTool {
    fn name(&self) -> &'static str {
        "Palettes"
    }

    fn description(&self) -> &'static str {
        "Build a 50–950 shade scale and color harmonies from one color. Click a swatch to copy it."
    }

    fn category(&self) -> Category {
        Category::Design
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.base.ui(ui);
        let base = self.base.color.with_alpha(1.0);
        ui.add_space(12.0);

        let shades = color::shades(base);
        let nearest = color::nearest_shade(base);
        ui.label(RichText::new("Shades").strong());
        ui.weak(format!("Your color is closest to step {nearest}."));
        let row: Vec<(String, Color)> = shades
            .iter()
            .map(|(step, c)| {
                (
                    if *step == nearest {
                        format!("{step} (base)")
                    } else {
                        step.to_string()
                    },
                    *c,
                )
            })
            .collect();
        swatch_row(ui, "shades", &row, 48.0);
        ui.add_space(12.0);

        ui.label(RichText::new("Harmonies").strong());
        egui::Grid::new("harmonies")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                for kind in Harmony::ALL {
                    ui.label(kind.label());
                    let colors: Vec<(String, Color)> = color::harmony(base, kind)
                        .into_iter()
                        .map(|c| (String::new(), c))
                        .collect();
                    ui.scope(|ui| {
                        ui.set_max_width(80.0 * colors.len() as f32);
                        swatch_row(ui, kind.label(), &colors, 32.0);
                    });
                    ui.end_row();
                }
            });
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Export").strong());
            for export in Export::ALL {
                ui.selectable_value(&mut self.export, export, export.label());
            }
            ui.add_space(12.0);
            ui.label("Name");
            ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(120.0));
        });
        let exported = match self.export {
            Export::CssVariables => color::css_variables(&self.name, &shades),
            Export::Tailwind => color::tailwind_theme(&self.name, &shades),
            Export::Json => color::json_palette(&self.name, &shades),
        };
        copy_button(ui, "copy_palette", "Copy", &exported);
        output_box(ui, "palette_export", &exported, 240.0);
    }
}
