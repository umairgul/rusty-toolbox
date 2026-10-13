use super::{Category, Tool, copy_button, error_label, output_box};
use toolbox_core::ids::{self, Kind, MAX_COUNT, MAX_NANOID_LENGTH, NANOID_ALPHABET};

const ALPHABET_PRESETS: &[(&str, &str)] = &[
    ("URL-safe (default)", NANOID_ALPHABET),
    (
        "Alphanumeric",
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    ),
    ("Lowercase + digits", "0123456789abcdefghijklmnopqrstuvwxyz"),
    ("Hex", "0123456789abcdef"),
    ("Digits", "0123456789"),
];

pub struct IdsTool {
    options: ids::Options,
    output: Result<String, ids::Error>,
}

impl Default for IdsTool {
    fn default() -> Self {
        let mut tool = Self {
            options: ids::Options {
                count: 5,
                ..Default::default()
            },
            output: Ok(String::new()),
        };
        tool.regenerate();
        tool
    }
}

impl IdsTool {
    fn regenerate(&mut self) {
        self.output = ids::generate(&self.options).map(|list| list.join("\n"));
    }

    fn nanoid_options_ui(&mut self, ui: &mut egui::Ui) {
        let options = &mut self.options;
        ui.horizontal(|ui| {
            ui.label("Length");
            ui.add(egui::DragValue::new(&mut options.nanoid_length).range(1..=MAX_NANOID_LENGTH));
        });
        ui.horizontal(|ui| {
            ui.label("Alphabet");
            let preset = ALPHABET_PRESETS
                .iter()
                .find(|(_, alphabet)| *alphabet == options.nanoid_alphabet);
            egui::ComboBox::from_id_salt("nanoid_alphabet")
                .selected_text(preset.map_or("Custom", |(name, _)| name))
                .show_ui(ui, |ui| {
                    for (name, alphabet) in ALPHABET_PRESETS {
                        if ui
                            .selectable_label(options.nanoid_alphabet == *alphabet, *name)
                            .clicked()
                        {
                            options.nanoid_alphabet = (*alphabet).to_owned();
                        }
                    }
                });
        });
        ui.add(
            egui::TextEdit::singleline(&mut options.nanoid_alphabet)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .hint_text("Characters to use"),
        );
    }
}

impl Tool for IdsTool {
    fn name(&self) -> &'static str {
        "IDs"
    }

    fn description(&self) -> &'static str {
        "UUIDs, ULIDs and Nano IDs, singly or in bulk."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let before = self.options.clone();

        ui.horizontal(|ui| {
            for kind in Kind::ALL {
                ui.selectable_value(&mut self.options.kind, kind, kind.label());
            }
        });
        ui.weak(self.options.kind.description());
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Count");
            ui.add(egui::DragValue::new(&mut self.options.count).range(1..=MAX_COUNT));
        });
        match self.options.kind {
            Kind::UuidV4 | Kind::UuidV7 => {
                ui.checkbox(&mut self.options.uuid_hyphens, "Hyphens");
                ui.checkbox(&mut self.options.uuid_uppercase, "Uppercase");
            }
            Kind::Ulid => {
                ui.checkbox(&mut self.options.ulid_lowercase, "Lowercase");
            }
            Kind::NanoId => self.nanoid_options_ui(ui),
        }
        ui.add_space(8.0);

        let output = self.output.as_ref().map(String::as_str).unwrap_or_default();
        let mut regenerate = self.options != before;
        ui.horizontal(|ui| {
            regenerate |= ui.button("Regenerate").clicked();
            copy_button(
                ui,
                "copy_ids",
                if self.options.count == 1 {
                    "Copy"
                } else {
                    "Copy all"
                },
                output,
            );
        });
        if regenerate {
            self.regenerate();
        }
        ui.add_space(8.0);

        match &self.output {
            Ok(text) => output_box(ui, "ids_output", text, 320.0),
            Err(error) => error_label(ui, error),
        }
    }
}
