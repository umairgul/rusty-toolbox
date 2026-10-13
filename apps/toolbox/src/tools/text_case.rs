use super::{Category, Tool, ValueRow, value_rows};
use toolbox_core::case::Case;

pub struct TextCaseTool {
    input: String,
}

impl Default for TextCaseTool {
    fn default() -> Self {
        Self {
            input: "parseHTTPResponse".into(),
        }
    }
}

impl Tool for TextCaseTool {
    fn name(&self) -> &'static str {
        "Text Case"
    }

    fn description(&self) -> &'static str {
        "Convert text and identifiers between camelCase, snake_case, kebab-case and more. Each line is converted separately."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::multiline(&mut self.input)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .hint_text("Text or identifiers, one per line"),
        );
        ui.add_space(8.0);
        if self.input.trim().is_empty() {
            return;
        }
        let rows: Vec<ValueRow> = Case::ALL
            .iter()
            .map(|case| ValueRow::new(case.label(), case.convert(&self.input)))
            .collect();
        value_rows(ui, "case_rows", &rows);
    }
}
