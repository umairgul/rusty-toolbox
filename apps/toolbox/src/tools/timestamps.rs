use super::{Category, Tool, ValueRow, copy_button, error_label, value_rows, zone_picker};
use egui::RichText;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use std::time::Duration;
use toolbox_core::timestamp::{self, Unit};

const DEFAULT_ZONES: [&str; 5] = [
    "UTC",
    "America/New_York",
    "Europe/London",
    "Asia/Kolkata",
    "Asia/Tokyo",
];

struct Zone {
    name: String,
    zone: TimeZone,
}

impl Zone {
    fn get(name: &str) -> Option<Zone> {
        TimeZone::get(name).ok().map(|zone| Zone {
            name: name.to_owned(),
            zone,
        })
    }
}

pub struct TimestampsTool {
    input: String,
    /// `None` guesses the unit of a plain number from its size.
    unit: Option<Unit>,
    /// Index into `zones` of the zone used for dates without an offset.
    input_zone: usize,
    zones: Vec<Zone>,
    zone_to_add: String,
}

impl Default for TimestampsTool {
    fn default() -> Self {
        let mut names: Vec<String> = timestamp::system_zone_name().into_iter().collect();
        for name in DEFAULT_ZONES {
            if !names.iter().any(|n| n == name) {
                names.push(name.to_owned());
            }
        }
        Self {
            input: Timestamp::now().as_second().to_string(),
            unit: None,
            input_zone: 0,
            zones: names.iter().filter_map(|name| Zone::get(name)).collect(),
            zone_to_add: String::new(),
        }
    }
}

impl TimestampsTool {
    fn input_zone(&self) -> TimeZone {
        self.zones
            .get(self.input_zone)
            .map_or(TimeZone::UTC, |zone| zone.zone.clone())
    }

    fn input_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(320.0)
                    .hint_text("1700000000, 2023-11-14T22:13:20Z, …"),
            );
            if ui.button("Now").clicked() {
                self.input = Timestamp::now().as_second().to_string();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Numbers are");
            let unit_label = self.unit.map_or("auto-detected", Unit::label);
            egui::ComboBox::from_id_salt("timestamp_unit")
                .selected_text(unit_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.unit, None, "auto-detected");
                    for unit in Unit::ALL {
                        ui.selectable_value(&mut self.unit, Some(unit), unit.label());
                    }
                });
            ui.add_space(12.0);
            ui.label("Dates without an offset are in");
            let zone_label = self
                .zones
                .get(self.input_zone)
                .map_or("UTC", |zone| zone.name.as_str());
            egui::ComboBox::from_id_salt("timestamp_input_zone")
                .selected_text(zone_label)
                .show_ui(ui, |ui| {
                    for (index, zone) in self.zones.iter().enumerate() {
                        ui.selectable_value(&mut self.input_zone, index, &zone.name);
                    }
                });
        });
    }

    fn zones_ui(&mut self, ui: &mut egui::Ui, timestamp: Timestamp) {
        let mut remove = None;
        egui::Grid::new("timestamp_zones")
            .num_columns(4)
            .striped(true)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for (index, zone) in self.zones.iter().enumerate() {
                    let formatted = timestamp::format_in_zone(timestamp, &zone.zone);
                    let is_system = index == 0
                        && timestamp::system_zone_name().as_deref() == Some(zone.name.as_str());
                    ui.label(if is_system {
                        format!("{} (this Mac)", zone.name)
                    } else {
                        zone.name.clone()
                    });
                    copy_button(ui, ("copy_zone", index), "Copy", &formatted);
                    ui.label(RichText::new(formatted).monospace());
                    if ui.small_button("×").on_hover_text("Remove").clicked() {
                        remove = Some(index);
                    }
                    ui.end_row();
                }
            });
        if let Some(index) = remove {
            self.zones.remove(index);
            if self.input_zone >= index && self.input_zone > 0 {
                self.input_zone -= 1;
            }
        }
        if zone_picker(
            ui,
            "timestamp_add_zone",
            "Add time zone…",
            &mut self.zone_to_add,
        ) && !self.zones.iter().any(|zone| zone.name == self.zone_to_add)
            && let Some(zone) = Zone::get(&self.zone_to_add)
        {
            self.zones.push(zone);
        }
    }
}

impl Tool for TimestampsTool {
    fn name(&self) -> &'static str {
        "Timestamps"
    }

    fn description(&self) -> &'static str {
        "Convert Unix timestamps and dates, and see them in any time zone."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let now = Timestamp::now();
        ui.horizontal(|ui| {
            ui.weak("Current Unix time:");
            let current = now.as_second().to_string();
            ui.label(RichText::new(&current).monospace());
            copy_button(ui, "copy_now", "Copy", &current);
        });
        ui.ctx().request_repaint_after(Duration::from_millis(
            1000 - u64::from(now.subsec_millisecond().unsigned_abs()),
        ));
        ui.add_space(8.0);

        self.input_ui(ui);
        ui.add_space(8.0);

        let (timestamp, source) = match timestamp::parse(&self.input, self.unit, &self.input_zone())
        {
            Ok(parsed) => parsed,
            Err(error) => {
                error_label(ui, error);
                return;
            }
        };
        ui.weak(format!("Read as {source}."));
        ui.add_space(4.0);

        let input_zone = self.input_zone();
        let rows = [
            ValueRow::new(
                "Unix seconds",
                timestamp::unix(timestamp, Unit::Seconds).to_string(),
            ),
            ValueRow::new(
                "Unix milliseconds",
                timestamp::unix(timestamp, Unit::Milliseconds).to_string(),
            ),
            ValueRow::new("ISO 8601 (UTC)", timestamp.to_string()),
            ValueRow::new("RFC 2822", timestamp::rfc2822(timestamp, &input_zone)),
            ValueRow::new("Relative", timestamp::relative(timestamp, now)),
            ValueRow::new(
                "Calendar",
                timestamp::calendar_details(timestamp, &input_zone),
            ),
        ];
        value_rows(ui, "timestamp_rows", &rows);
        ui.add_space(12.0);

        ui.label(RichText::new("Time zones").strong());
        self.zones_ui(ui, timestamp);
    }
}
