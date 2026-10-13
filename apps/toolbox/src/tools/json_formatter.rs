use super::{Category, Tool, copy_button, error_label, human_size, output_box, status_label};
use toolbox_core::json_format::{self, Indent, SyntaxError};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Format,
    Minify,
}

#[derive(Clone, PartialEq, Eq)]
struct Settings {
    mode: Mode,
    indent: Indent,
    sort_keys: bool,
}

pub struct JsonFormatterTool {
    input: String,
    settings: Settings,
    /// The input and settings the cached output was made from.
    formatted: Option<(String, Settings)>,
    output: Result<String, SyntaxError>,
}

impl Default for JsonFormatterTool {
    fn default() -> Self {
        Self {
            input: r#"{"name":"Toolbox","version":"0.1.0","price":19.90,"tags":["rust","egui"],"window":{"width":900,"height":640},"beta":false,"owner":null}"#.into(),
            settings: Settings { mode: Mode::Format, indent: Indent::TwoSpaces, sort_keys: false },
            formatted: None,
            output: Ok(String::new()),
        }
    }
}

impl JsonFormatterTool {
    fn format_if_changed(&mut self) {
        let key = (self.input.clone(), self.settings.clone());
        if self.formatted.as_ref() == Some(&key) {
            return;
        }
        let Settings {
            mode,
            indent,
            sort_keys,
        } = self.settings;
        self.output = match mode {
            Mode::Minify => json_format::minify(&self.input),
            Mode::Format if sort_keys => json_format::format_sorted(&self.input, indent),
            Mode::Format => json_format::format(&self.input, indent),
        };
        self.formatted = Some(key);
    }
}

impl Tool for JsonFormatterTool {
    fn name(&self) -> &'static str {
        "JSON Formatter"
    }

    fn description(&self) -> &'static str {
        "Format, minify and validate JSON. Numbers, escapes and key order are kept exactly as written."
    }

    fn category(&self) -> Category {
        Category::Formatters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.settings.mode, Mode::Format, "Format");
            ui.selectable_value(&mut self.settings.mode, Mode::Minify, "Minify");
            if self.settings.mode == Mode::Format {
                ui.add_space(12.0);
                egui::ComboBox::from_id_salt("json_indent")
                    .selected_text(self.settings.indent.label())
                    .show_ui(ui, |ui| {
                        for indent in Indent::ALL {
                            ui.selectable_value(&mut self.settings.indent, indent, indent.label());
                        }
                    });
                ui.checkbox(&mut self.settings.sort_keys, "Sort keys")
                    .on_hover_text("Sorting parses the JSON, so very large integers and float formatting may change.");
            }
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("json_input")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(8)
                        .hint_text("Paste JSON"),
                );
            });
        self.format_if_changed();
        ui.add_space(8.0);

        match &self.output {
            Ok(output) => {
                status_label(
                    ui,
                    true,
                    format!(
                        "Valid JSON · {} in, {} out",
                        human_size(self.input.len() as u64),
                        human_size(output.len() as u64)
                    ),
                );
                let output = output.clone();
                ui.horizontal(|ui| {
                    copy_button(ui, "copy_json", "Copy result", &output);
                    if ui.button("Use as input").clicked() {
                        self.input = output.clone();
                    }
                });
                output_box(ui, "json_output", &output, 360.0);
            }
            Err(error) => {
                error_label(ui, error);
                if let Some(context) = error.context(&self.input) {
                    output_box(ui, "json_error_context", &context, 80.0);
                }
            }
        }
    }
}
