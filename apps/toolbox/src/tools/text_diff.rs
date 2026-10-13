use super::{Category, Tool, copy_button, status_label};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, RichText};
use toolbox_core::diff::{self, Diff, Options, RowKind, Side};

/// Rows drawn. Larger diffs are cut off with a note; the unified diff stays complete.
const MAX_ROWS: usize = 2000;
const LINE_NUMBER_WIDTH: f32 = 34.0;

struct Palette {
    removed_line: Color32,
    removed_word: Color32,
    added_line: Color32,
    added_word: Color32,
    missing: Color32,
}

impl Palette {
    fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Palette {
                removed_line: Color32::from_rgb(72, 30, 32),
                removed_word: Color32::from_rgb(130, 42, 46),
                added_line: Color32::from_rgb(24, 60, 36),
                added_word: Color32::from_rgb(34, 110, 58),
                missing: Color32::from_gray(38),
            }
        } else {
            Palette {
                removed_line: Color32::from_rgb(255, 235, 233),
                removed_word: Color32::from_rgb(255, 190, 186),
                added_line: Color32::from_rgb(230, 255, 236),
                added_word: Color32::from_rgb(166, 240, 184),
                missing: Color32::from_gray(242),
            }
        }
    }
}

pub struct TextDiffTool {
    left: String,
    right: String,
    options: Options,
    compared: Option<(String, String, Options)>,
    diff: Diff,
}

impl Default for TextDiffTool {
    fn default() -> Self {
        Self {
            left: "fn greet(name: &str) {\n    println!(\"Hello, {name}!\");\n}\n\nfn main() {\n    greet(\"world\");\n}".into(),
            right: "fn greet(name: &str, excited: bool) {\n    println!(\"Hello, {name}!\");\n}\n\nfn main() {\n    greet(\"Toolbox\", true);\n    greet(\"world\", false);\n}".into(),
            options: Options::default(),
            compared: None,
            diff: Diff::default(),
        }
    }
}

impl TextDiffTool {
    fn compare_if_changed(&mut self) {
        let key = (self.left.clone(), self.right.clone(), self.options);
        if self.compared.as_ref() != Some(&key) {
            self.diff = diff::diff(&self.left, &self.right, self.options);
            self.compared = Some(key);
        }
    }
}

/// One side of a diff row: a right-aligned line number, then the line with changed words
/// marked. `removed` picks the red or green palette.
fn side_ui(
    ui: &mut egui::Ui,
    side: Option<&Side>,
    kind: RowKind,
    removed: bool,
    width: f32,
    palette: &Palette,
) {
    let (line_color, word_color) = if removed {
        (palette.removed_line, palette.removed_word)
    } else {
        (palette.added_line, palette.added_word)
    };
    let background = match (side, kind) {
        (None, _) => palette.missing,
        (Some(_), RowKind::Equal) => Color32::TRANSPARENT,
        (Some(_), _) => line_color,
    };
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let plain = TextFormat::simple(font, ui.visuals().text_color());
    let mut job = LayoutJob::default();
    if let Some(side) = side {
        for segment in &side.segments {
            let format = if segment.changed {
                TextFormat {
                    background: word_color,
                    ..plain.clone()
                }
            } else {
                plain.clone()
            };
            job.append(&segment.text, 0.0, format);
        }
    }
    ui.allocate_ui_with_layout(
        egui::vec2(LINE_NUMBER_WIDTH, 0.0),
        egui::Layout::right_to_left(egui::Align::TOP),
        |ui| {
            ui.label(
                RichText::new(side.map_or(String::new(), |s| s.line.to_string()))
                    .monospace()
                    .weak(),
            );
        },
    );
    egui::Frame::NONE
        .fill(background)
        .inner_margin(egui::Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.set_width(width - 8.0);
            ui.add(egui::Label::new(job).wrap());
        });
}

impl Tool for TextDiffTool {
    fn name(&self) -> &'static str {
        "Text Diff"
    }

    fn description(&self) -> &'static str {
        "Compare two texts side by side, with changed words highlighted."
    }

    fn category(&self) -> Category {
        Category::Formatters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.ignore_whitespace, "Ignore whitespace");
            ui.checkbox(&mut self.options.ignore_case, "Ignore case");
            if ui.button("Swap sides").clicked() {
                std::mem::swap(&mut self.left, &mut self.right);
            }
            copy_button(
                ui,
                "copy_unified_diff",
                "Copy unified diff",
                &diff::unified(&self.left, &self.right),
            );
        });
        ui.add_space(8.0);

        ui.columns(2, |columns| {
            for (column, (title, text, id)) in columns.iter_mut().zip([
                ("Original", &mut self.left, "diff_left"),
                ("Changed", &mut self.right, "diff_right"),
            ]) {
                column.label(RichText::new(title).strong());
                egui::ScrollArea::vertical()
                    .id_salt(id)
                    .max_height(200.0)
                    .show(column, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(text)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(8),
                        );
                    });
            }
        });
        self.compare_if_changed();
        ui.add_space(8.0);

        if self.diff.is_identical() {
            status_label(ui, true, "No differences");
            return;
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("+{} added", self.diff.added))
                    .color(Color32::from_rgb(0x2e, 0x9d, 0x4f))
                    .strong(),
            );
            ui.label(
                RichText::new(format!("−{} removed", self.diff.removed))
                    .color(ui.visuals().error_fg_color)
                    .strong(),
            );
        });
        ui.add_space(4.0);

        let palette = Palette::new(ui.visuals().dark_mode);
        let spacing = ui.spacing().item_spacing.x;
        let half =
            ((ui.available_width() - 2.0 * LINE_NUMBER_WIDTH - 3.0 * spacing) / 2.0).max(120.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        for row in self.diff.rows.iter().take(MAX_ROWS) {
            ui.horizontal_top(|ui| {
                side_ui(ui, row.left.as_ref(), row.kind, true, half, &palette);
                side_ui(ui, row.right.as_ref(), row.kind, false, half, &palette);
            });
        }
        if self.diff.rows.len() > MAX_ROWS {
            ui.add_space(4.0);
            ui.weak(format!(
                "Showing the first {MAX_ROWS} of {} lines. The unified diff has everything.",
                self.diff.rows.len()
            ));
        }
    }
}
