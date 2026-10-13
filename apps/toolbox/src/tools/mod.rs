//! The tools shown in the sidebar, plus widgets they share. To add a tool: create a module
//! implementing [`Tool`], then add it to [`all`].

mod cron;
mod encode;
mod fake_data;
mod formats;
mod hashes;
mod ids;
mod jwt;
mod lorem;
mod names;
mod number_base;
mod password;
mod qr;
mod text_case;
mod timestamps;

use egui::{Color32, RichText};
use std::path::PathBuf;
use std::time::Duration;
use toolbox_core::strength::Strength;

/// How long the "Copied" confirmation stays visible.
const COPIED_FEEDBACK: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Generators,
    Converters,
}

impl Category {
    pub const ALL: [Category; 2] = [Category::Generators, Category::Converters];

    pub fn label(self) -> &'static str {
        match self {
            Category::Generators => "GENERATORS",
            Category::Converters => "ENCODERS & CONVERTERS",
        }
    }
}

pub trait Tool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn category(&self) -> Category;
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Every tool, in sidebar order within each category.
pub fn all() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(password::PasswordTool::default()),
        Box::new(names::NamesTool::default()),
        Box::new(ids::IdsTool::default()),
        Box::new(qr::QrTool::default()),
        Box::new(lorem::LoremTool::default()),
        Box::new(fake_data::FakeDataTool::default()),
        Box::new(hashes::HashesTool::default()),
        Box::new(encode::EncodeTool::default()),
        Box::new(timestamps::TimestampsTool::default()),
        Box::new(jwt::JwtTool::default()),
        Box::new(formats::FormatsTool::default()),
        Box::new(number_base::NumberBaseTool::default()),
        Box::new(text_case::TextCaseTool::default()),
        Box::new(cron::CronTool::default()),
    ]
}

/// A button that copies `text` to the clipboard and briefly reads "Copied!". The feedback
/// clears early if `text` changes, since the clipboard no longer matches what's shown.
pub fn copy_button(ui: &mut egui::Ui, id_salt: impl egui::AsIdSalt, label: &str, text: &str) {
    let id = ui.make_persistent_id(id_salt);
    let now = ui.input(|input| input.time);
    let text_id = egui::Id::new(text);
    let copied: Option<(f64, egui::Id)> = ui.ctx().data(|data| data.get_temp(id));
    let recently_copied = copied.is_some_and(|(at, copied_text)| {
        copied_text == text_id && now - at < COPIED_FEEDBACK.as_secs_f64()
    });

    let button = egui::Button::new(if recently_copied { "Copied!" } else { label });
    if ui.add_enabled(!text.is_empty(), button).clicked() {
        ui.ctx().copy_text(text.to_owned());
        ui.ctx()
            .data_mut(|data| data.insert_temp(id, (now, text_id)));
        ui.ctx().request_repaint_after(COPIED_FEEDBACK);
    } else if recently_copied {
        ui.ctx().request_repaint_after(COPIED_FEEDBACK);
    }
}

/// Selectable, read-only monospace text in a scroll area.
pub fn output_box(ui: &mut egui::Ui, id_salt: impl egui::AsIdSalt, text: &str, max_height: f32) {
    let mut text = text;
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            );
        });
}

/// One row of [`value_rows`]: a label, the text shown, and the text copied.
pub struct ValueRow {
    pub label: String,
    pub shown: String,
    pub copied: String,
}

impl ValueRow {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: label.into(),
            shown: value.clone(),
            copied: value,
        }
    }

    /// Shows `shown` but copies `copied`, e.g. digits grouped for reading but copied plain.
    pub fn with_copy(
        label: impl Into<String>,
        shown: impl Into<String>,
        copied: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            shown: shown.into(),
            copied: copied.into(),
        }
    }
}

/// A table of label, Copy button and value. Values are monospace and wrap.
pub fn value_rows(ui: &mut egui::Ui, id_salt: &str, rows: &[ValueRow]) {
    egui::Grid::new(id_salt)
        .num_columns(3)
        .striped(true)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for (index, row) in rows.iter().enumerate() {
                ui.label(&row.label);
                copy_button(ui, (id_salt, index), "Copy", &row.copied);
                ui.add(egui::Label::new(RichText::new(&row.shown).monospace()).wrap());
                ui.end_row();
            }
        });
}

/// A combo box listing every IANA time zone, with a filter box at the top.
/// Returns true when a zone was picked.
pub fn zone_picker(
    ui: &mut egui::Ui,
    id_salt: &str,
    button_text: &str,
    picked: &mut String,
) -> bool {
    let filter_id = ui.make_persistent_id((id_salt, "filter"));
    let mut filter: String = ui
        .ctx()
        .data(|data| data.get_temp(filter_id))
        .unwrap_or_default();
    let mut chosen = None;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(button_text)
        .width(220.0)
        .height(320.0)
        .show_ui(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Search, e.g. Paris or Asia/")
                    .desired_width(200.0),
            )
            .request_focus();
            let needle = filter.to_lowercase().replace(' ', "_");
            for name in toolbox_core::timestamp::zone_names()
                .iter()
                .filter(|name| name.to_lowercase().contains(&needle))
                .take(300)
            {
                if ui.selectable_label(false, name).clicked() {
                    chosen = Some(name.clone());
                }
            }
        });
    ui.ctx()
        .data_mut(|data| data.insert_temp(filter_id, filter));
    match chosen {
        Some(name) => {
            *picked = name;
            true
        }
        None => false,
    }
}

pub fn error_label(ui: &mut egui::Ui, message: impl std::fmt::Display) {
    ui.label(RichText::new(message.to_string()).color(ui.visuals().error_fg_color));
}

pub fn strength_bar(ui: &mut egui::Ui, strength: Strength, bits: f64) {
    let color = match strength {
        Strength::Weak => Color32::from_rgb(0xc6, 0x3a, 0x2e),
        Strength::Fair => Color32::from_rgb(0xb8, 0x76, 0x12),
        Strength::Strong => Color32::from_rgb(0x3d, 0x8f, 0x41),
        Strength::VeryStrong => Color32::from_rgb(0x2e, 0x7d, 0x50),
    };
    ui.add(
        egui::ProgressBar::new((bits / 128.0).min(1.0) as f32)
            .fill(color)
            .text(
                RichText::new(format!("{} · {bits:.0} bits of entropy", strength.label()))
                    .color(Color32::WHITE),
            ),
    );
}

/// Asks where to save, then writes `contents`. Returns a status message, or `None` if cancelled.
/// A green or red status line, e.g. for "Signature verified".
pub fn status_label(ui: &mut egui::Ui, ok: bool, message: impl Into<String>) {
    let color = if ok {
        Color32::from_rgb(0x2e, 0x9d, 0x4f)
    } else {
        ui.visuals().error_fg_color
    };
    ui.label(RichText::new(message.into()).color(color).strong());
}

pub fn save_file(
    file_name: &str,
    filter_name: &str,
    extension: &str,
    contents: &[u8],
) -> Option<String> {
    let path: PathBuf = rfd::FileDialog::new()
        .set_file_name(file_name)
        .add_filter(filter_name, &[extension])
        .save_file()?;
    Some(match std::fs::write(&path, contents) {
        Ok(()) => format!("Saved to {}", path.display()),
        Err(error) => format!("Couldn't save {}: {error}", path.display()),
    })
}
