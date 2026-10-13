use super::{Category, Tool, copy_button, error_label, output_box, save_file};
use toolbox_core::formats::{self, Format};

pub struct FormatsTool {
    /// `None` detects the input format.
    from: Option<Format>,
    to: Format,
    options: formats::Options,
    input: String,
    /// The inputs the cached output was made from.
    converted: Option<(String, Option<Format>, Format, formats::Options)>,
    output: Result<String, formats::Error>,
    detected: Format,
    status: Option<String>,
}

impl Default for FormatsTool {
    fn default() -> Self {
        Self {
            from: None,
            to: Format::Yaml,
            options: formats::Options::default(),
            input: "{\n  \"name\": \"Toolbox\",\n  \"version\": \"0.1.0\",\n  \"tools\": [\"hashes\", \"timestamps\"],\n  \"window\": { \"width\": 900, \"height\": 640 }\n}".into(),
            converted: None,
            output: Ok(String::new()),
            detected: Format::Json,
            status: None,
        }
    }
}

impl FormatsTool {
    fn convert_if_changed(&mut self) {
        let key = (self.input.clone(), self.from, self.to, self.options.clone());
        if self.converted.as_ref() == Some(&key) {
            return;
        }
        self.detected = formats::detect(&self.input);
        let from = self.from.unwrap_or(self.detected);
        self.output = formats::convert(&self.input, from, self.to, &self.options);
        self.converted = Some(key);
        self.status = None;
    }
}

impl Tool for FormatsTool {
    fn name(&self) -> &'static str {
        "Data Formats"
    }

    fn description(&self) -> &'static str {
        "Convert between JSON, YAML, TOML and CSV."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("From");
            let from_label = match self.from {
                Some(format) => format.label().to_owned(),
                None => format!("Auto ({})", self.detected.label()),
            };
            egui::ComboBox::from_id_salt("formats_from")
                .selected_text(from_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.from, None, "Auto-detect");
                    for format in Format::ALL {
                        ui.selectable_value(&mut self.from, Some(format), format.label());
                    }
                });
            if ui
                .button("Swap")
                .on_hover_text("Swap: convert the output back")
                .clicked()
                && let Ok(output) = &self.output
            {
                let from = self.from.unwrap_or(self.detected);
                self.input = output.clone();
                self.from = Some(self.to);
                self.to = from;
            }
            ui.label("To");
            egui::ComboBox::from_id_salt("formats_to")
                .selected_text(self.to.label())
                .show_ui(ui, |ui| {
                    for format in Format::ALL {
                        ui.selectable_value(&mut self.to, format, format.label());
                    }
                });
            ui.add_space(12.0);
            if self.to == Format::Json {
                ui.checkbox(&mut self.options.pretty, "Pretty");
            }
            if self.from.unwrap_or(self.detected) == Format::Csv {
                ui.checkbox(
                    &mut self.options.infer_csv_types,
                    "Detect numbers and booleans",
                );
            }
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("formats_input")
            .max_height(240.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(10),
                );
            });
        self.convert_if_changed();
        ui.add_space(8.0);

        let text = self.output.as_ref().map(String::as_str).unwrap_or_default();
        ui.horizontal(|ui| {
            copy_button(ui, "copy_converted", "Copy result", text);
            if ui
                .add_enabled(!text.is_empty(), egui::Button::new("Save…"))
                .clicked()
            {
                let extension = self.to.extension();
                self.status = save_file(
                    &format!("converted.{extension}"),
                    self.to.label(),
                    extension,
                    text.as_bytes(),
                );
            }
            if let Some(status) = &self.status {
                ui.weak(status.as_str());
            }
        });
        match &self.output {
            Ok(text) => output_box(ui, "formats_output", text, 320.0),
            Err(error) => error_label(ui, error),
        }
    }
}
