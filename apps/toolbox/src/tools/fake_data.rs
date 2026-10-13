use super::{Category, Tool, copy_button, error_label, output_box, save_file};
use toolbox_core::fake::{self, Field, Format, MAX_ROWS};

/// Lines shown in the preview. Copy and Save always use the full output.
const PREVIEW_LINES: usize = 200;
const FIELD_COLUMNS: usize = 4;

pub struct FakeDataTool {
    options: fake::Options,
    output: Result<String, fake::Error>,
    status: Option<String>,
}

impl Default for FakeDataTool {
    fn default() -> Self {
        let options = fake::Options::default();
        Self {
            output: fake::generate(&options),
            options,
            status: None,
        }
    }
}

impl FakeDataTool {
    fn regenerate(&mut self) {
        self.output = fake::generate(&self.options);
        self.status = None;
    }

    fn fields_ui(&mut self, ui: &mut egui::Ui) {
        let mut enabled = Field::ALL.map(|field| self.options.fields.contains(&field));
        egui::Grid::new("fake_fields")
            .num_columns(FIELD_COLUMNS)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (index, (field, on)) in Field::ALL.iter().zip(enabled.iter_mut()).enumerate() {
                    ui.checkbox(on, field.column());
                    if (index + 1) % FIELD_COLUMNS == 0 {
                        ui.end_row();
                    }
                }
            });
        self.options.fields = Field::ALL
            .into_iter()
            .zip(enabled)
            .filter(|(_, on)| *on)
            .map(|(field, _)| field)
            .collect();
    }
}

impl Tool for FakeDataTool {
    fn name(&self) -> &'static str {
        "Fake Data"
    }

    fn description(&self) -> &'static str {
        "Realistic test records for seeding databases, as CSV, JSON or SQL. Emails and phone numbers are fictional."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let before = self.options.clone();

        ui.horizontal(|ui| {
            for format in Format::ALL {
                ui.selectable_value(&mut self.options.format, format, format.label());
            }
            ui.add_space(16.0);
            ui.label("Rows");
            ui.add(egui::DragValue::new(&mut self.options.rows).range(1..=MAX_ROWS));
            if self.options.format == Format::Sql {
                ui.add_space(16.0);
                ui.label("Table");
                ui.add(egui::TextEdit::singleline(&mut self.options.table).desired_width(120.0));
            }
        });
        ui.add_space(8.0);
        self.fields_ui(ui);
        ui.add_space(8.0);

        let full = self.output.as_ref().map(String::as_str).unwrap_or_default();
        let mut regenerate = self.options != before;
        ui.horizontal(|ui| {
            regenerate |= ui.button("Regenerate").clicked();
            copy_button(ui, "copy_fake_data", "Copy all", full);
            if ui
                .add_enabled(!full.is_empty(), egui::Button::new("Save…"))
                .clicked()
            {
                let format = self.options.format;
                let file_name = format!("{}.{}", self.options.table, format.extension());
                self.status = save_file(
                    &file_name,
                    format.label(),
                    format.extension(),
                    full.as_bytes(),
                );
            }
        });
        if let Some(status) = &self.status {
            ui.weak(status.as_str());
        }
        if regenerate {
            self.regenerate();
        }
        ui.add_space(8.0);

        match &self.output {
            Ok(text) => {
                let total_lines = text.lines().count();
                if total_lines > PREVIEW_LINES {
                    let preview: String = text
                        .lines()
                        .take(PREVIEW_LINES)
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.weak(format!("Preview: first {PREVIEW_LINES} of {total_lines} lines. Copy and Save include everything."));
                    output_box(ui, "fake_output", &preview, 360.0);
                } else {
                    output_box(ui, "fake_output", text, 360.0);
                }
            }
            Err(error) => error_label(ui, error),
        }
    }
}
