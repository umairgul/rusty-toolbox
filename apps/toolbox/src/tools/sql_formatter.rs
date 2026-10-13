use super::{Category, Tool, copy_button, output_box};
use toolbox_core::sql::{self, Dialect, Indent, KeywordCase};

pub struct SqlFormatterTool {
    input: String,
    options: sql::Options,
}

impl Default for SqlFormatterTool {
    fn default() -> Self {
        Self {
            input: "select u.id, u.name, count(o.id) as orders from users u left join orders o on o.user_id = u.id where u.active = 1 and o.created_at >= '2024-01-01' group by u.id, u.name having count(o.id) > 5 order by orders desc limit 10;".into(),
            options: sql::Options::default(),
        }
    }
}

impl Tool for SqlFormatterTool {
    fn name(&self) -> &'static str {
        "SQL Formatter"
    }

    fn description(&self) -> &'static str {
        "Format SQL queries for reading, or compact them onto one line."
    }

    fn category(&self) -> Category {
        Category::Formatters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let options = &mut self.options;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("sql_keyword_case")
                .selected_text(options.keyword_case.label())
                .show_ui(ui, |ui| {
                    for case in KeywordCase::ALL {
                        ui.selectable_value(&mut options.keyword_case, case, case.label());
                    }
                });
            ui.add_enabled_ui(!options.compact, |ui| {
                egui::ComboBox::from_id_salt("sql_indent")
                    .selected_text(options.indent.label())
                    .show_ui(ui, |ui| {
                        for indent in Indent::ALL {
                            ui.selectable_value(&mut options.indent, indent, indent.label());
                        }
                    });
            });
            egui::ComboBox::from_id_salt("sql_dialect")
                .selected_text(options.dialect.label())
                .show_ui(ui, |ui| {
                    for dialect in Dialect::ALL {
                        ui.selectable_value(&mut options.dialect, dialect, dialect.label());
                    }
                });
            ui.checkbox(&mut options.compact, "Compact (one line per query)");
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("sql_input")
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .hint_text("Paste SQL"),
                );
            });
        ui.add_space(8.0);

        let formatted = sql::format(&self.input, &self.options);
        copy_button(ui, "copy_sql", "Copy result", &formatted);
        output_box(ui, "sql_output", &formatted, 400.0);
    }
}
