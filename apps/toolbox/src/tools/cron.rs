use super::{Category, Tool, error_label, zone_picker};
use egui::RichText;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use toolbox_core::cron::{self, EXAMPLES};
use toolbox_core::timestamp;

const NEXT_RUNS: usize = 10;

pub struct CronTool {
    pattern: String,
    zone_name: String,
}

impl Default for CronTool {
    fn default() -> Self {
        Self {
            pattern: "0 9 * * 1-5".into(),
            zone_name: timestamp::system_zone_name().unwrap_or_else(|| "UTC".into()),
        }
    }
}

impl Tool for CronTool {
    fn name(&self) -> &'static str {
        "Cron"
    }

    fn description(&self) -> &'static str {
        "Explain a cron expression in plain English and list its next run times."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.pattern)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(260.0)
                    .hint_text("*/15 * * * *"),
            );
            ui.menu_button("Examples", |ui| {
                for (pattern, label) in EXAMPLES {
                    let text = egui::RichText::new(format!("{pattern:<14} {label}")).monospace();
                    if ui.button(text).clicked() {
                        self.pattern = (*pattern).to_owned();
                        ui.close();
                    }
                }
            });
        });
        ui.weak("Fields: minute  hour  day-of-month  month  day-of-week, with an optional leading seconds field. Nicknames like @daily work too.");
        ui.add_space(8.0);

        let zone = TimeZone::get(&self.zone_name).unwrap_or(TimeZone::UTC);
        let now = Timestamp::now().to_zoned(zone.clone());
        let explanation = match cron::explain(&self.pattern, &now, NEXT_RUNS) {
            Ok(explanation) => explanation,
            Err(error) => {
                error_label(ui, error);
                return;
            }
        };
        ui.label(RichText::new(&explanation.description).size(17.0).strong());
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Next {NEXT_RUNS} runs in")).strong());
            let current = self.zone_name.clone();
            zone_picker(ui, "cron_zone", &current, &mut self.zone_name);
        });
        egui::Grid::new("cron_runs")
            .num_columns(2)
            .striped(true)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for run in &explanation.next_runs {
                    ui.label(
                        RichText::new(run.strftime("%a %Y-%m-%d %H:%M:%S %Z").to_string())
                            .monospace(),
                    );
                    ui.weak(timestamp::relative(run.timestamp(), now.timestamp()));
                    ui.end_row();
                }
            });
        if explanation.next_runs.is_empty() {
            ui.weak("This expression never runs again.");
        }
    }
}
