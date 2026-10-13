use super::{Category, Tool, copy_button};
use egui::RichText;
use toolbox_core::names::{self, Case, Kind};

const MAX_COUNT: usize = 50;

pub struct NamesTool {
    kind: Kind,
    case: Case,
    count: usize,
    names: Vec<String>,
}

impl Default for NamesTool {
    fn default() -> Self {
        let mut tool = Self {
            kind: Kind::Project,
            case: Case::Kebab,
            count: 10,
            names: Vec::new(),
        };
        tool.regenerate();
        tool
    }
}

impl NamesTool {
    fn regenerate(&mut self) {
        self.names = names::generate_list(self.kind, self.case, self.count);
    }
}

impl Tool for NamesTool {
    fn name(&self) -> &'static str {
        "Names"
    }

    fn description(&self) -> &'static str {
        "Random names for projects, usernames, test people and companies."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let before = (self.kind, self.case, self.count);

        ui.horizontal(|ui| {
            for kind in Kind::ALL {
                ui.selectable_value(&mut self.kind, kind, kind.label());
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.count, 1..=MAX_COUNT).text("Count"));
            if self.kind == Kind::Project {
                ui.add_space(16.0);
                egui::ComboBox::from_id_salt("name_case")
                    .selected_text(self.case.label())
                    .show_ui(ui, |ui| {
                        for case in names::PROJECT_CASES {
                            ui.selectable_value(&mut self.case, case, case.label());
                        }
                    });
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Regenerate").clicked() {
                self.regenerate();
            }
            copy_button(ui, "copy_all_names", "Copy all", &self.names.join("\n"));
        });

        if (self.kind, self.case, self.count) != before {
            self.regenerate();
        }
        ui.add_space(8.0);
        ui.separator();

        egui::Grid::new("name_list")
            .num_columns(2)
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (index, name) in self.names.iter().enumerate() {
                    ui.label(RichText::new(name).monospace().size(15.0));
                    copy_button(ui, ("copy_name", index), "Copy", name);
                    ui.end_row();
                }
            });
    }
}
