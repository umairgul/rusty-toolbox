use super::{Category, Tool, copy_button, error_label, output_box};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, RichText};
use std::ops::Range;
use toolbox_core::regex_test::{self, Flags, Matches};

/// Matches listed and highlighted. Counting stops here too.
const MAX_MATCHES: usize = 1000;
/// Rows shown in the match table.
const MAX_ROWS: usize = 200;

const CHEAT_SHEET: &[(&str, &str)] = &[
    (".", "any character except newline"),
    (r"\d  \w  \s", "digit, word character, whitespace"),
    (r"\D  \W  \S", "not a digit, word character, whitespace"),
    ("[abc]  [^abc]  [a-z]", "one of, none of, range"),
    ("^  $", "start, end (of line with the m flag)"),
    (r"\b", "word boundary"),
    ("*  +  ?", "0 or more, 1 or more, 0 or 1"),
    ("{3}  {2,5}  {2,}", "exactly, between, at least"),
    ("*?  +?", "lazy: as few as possible"),
    ("(…)  (?:…)", "capture group, non-capturing group"),
    ("(?<name>…)", "named group"),
    (r"\1", "backreference to group 1"),
    ("(?=…)  (?!…)", "lookahead, negative lookahead"),
    ("(?<=…)  (?<!…)", "lookbehind, negative lookbehind"),
    ("a|b", "a or b"),
];

pub struct RegexTool {
    pattern: String,
    flags: Flags,
    text: String,
    replacement: String,
}

impl Default for RegexTool {
    fn default() -> Self {
        Self {
            pattern: r"(?<user>[\w.+-]+)@(?<domain>[\w-]+\.[\w.]+)".into(),
            flags: Flags::default(),
            text: "Contact alice@example.com or bob.smith+test@mail.example.org.\nInvalid: not-an-email@, @nope.com".into(),
            replacement: "${user} at ${domain}".into(),
        }
    }
}

/// Highlight colours that alternate between neighbouring matches.
fn match_colors(dark_mode: bool) -> [Color32; 2] {
    if dark_mode {
        [
            Color32::from_rgb(110, 88, 20),
            Color32::from_rgb(30, 72, 120),
        ]
    } else {
        [
            Color32::from_rgb(255, 226, 120),
            Color32::from_rgb(186, 222, 255),
        ]
    }
}

fn highlighted(ui: &egui::Ui, text: &str, ranges: &[Range<usize>], wrap_width: f32) -> LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let plain = TextFormat::simple(font, ui.visuals().text_color());
    let colors = match_colors(ui.visuals().dark_mode);
    let mut job = LayoutJob::default();
    let mut position = 0;
    for (index, range) in ranges.iter().enumerate() {
        if range.start < position || range.end > text.len() {
            continue;
        }
        job.append(&text[position..range.start], 0.0, plain.clone());
        job.append(
            &text[range.clone()],
            0.0,
            TextFormat {
                background: colors[index % 2],
                ..plain.clone()
            },
        );
        position = range.end;
    }
    job.append(&text[position..], 0.0, plain);
    job.wrap.max_width = wrap_width;
    job
}

impl RegexTool {
    fn flags_ui(&mut self, ui: &mut egui::Ui) {
        let flags = &mut self.flags;
        ui.checkbox(&mut flags.case_insensitive, "i")
            .on_hover_text("Case-insensitive");
        ui.checkbox(&mut flags.multi_line, "m")
            .on_hover_text("Multi-line: ^ and $ match at line breaks");
        ui.checkbox(&mut flags.dot_all, "s")
            .on_hover_text("Dot matches newlines");
        ui.checkbox(&mut flags.extended, "x")
            .on_hover_text("Extended: ignore whitespace and # comments in the pattern");
    }

    fn matches_table(&self, ui: &mut egui::Ui, found: &Matches) {
        egui::Grid::new("regex_matches")
            .num_columns(4)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("#").strong());
                ui.label(RichText::new("Position").strong());
                ui.label(RichText::new("Match").strong());
                ui.label(RichText::new("Groups").strong());
                ui.end_row();
                for (index, found) in found.matches.iter().take(MAX_ROWS).enumerate() {
                    ui.label((index + 1).to_string());
                    ui.weak(format!("{}–{}", found.range.start, found.range.end));
                    ui.label(
                        RichText::new(format!("{:?}", &self.text[found.range.clone()])).monospace(),
                    );
                    let groups: Vec<String> = found
                        .groups
                        .iter()
                        .enumerate()
                        .map(|(i, group)| {
                            let name = group.name.clone().unwrap_or_else(|| (i + 1).to_string());
                            match &group.range {
                                Some(range) => format!("{name}: {:?}", &self.text[range.clone()]),
                                None => format!("{name}: —"),
                            }
                        })
                        .collect();
                    ui.add(egui::Label::new(RichText::new(groups.join("   ")).monospace()).wrap());
                    ui.end_row();
                }
            });
        if found.matches.len() > MAX_ROWS {
            ui.weak(format!("Showing the first {MAX_ROWS} matches."));
        }
    }
}

impl Tool for RegexTool {
    fn name(&self) -> &'static str {
        "Regex Tester"
    }

    fn description(&self) -> &'static str {
        "Test regular expressions with live highlighting, capture groups and replace. Supports lookaround and backreferences."
    }

    fn category(&self) -> Category {
        Category::Formatters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("/").monospace().size(16.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.pattern)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width() - 190.0)
                    .hint_text("Regular expression"),
            );
            ui.label(RichText::new("/").monospace().size(16.0));
            self.flags_ui(ui);
        });

        let found = regex_test::find(&self.pattern, self.flags, &self.text, MAX_MATCHES);
        let ranges: Vec<Range<usize>> = found
            .as_ref()
            .map(|f| f.matches.iter().map(|m| m.range.clone()).collect())
            .unwrap_or_default();
        match &found {
            Err(error) => error_label(ui, error),
            Ok(found) if self.pattern.is_empty() => {
                let _ = found;
                ui.weak("Enter a pattern to start matching.");
            }
            Ok(found) => {
                let count = found.matches.len();
                let more = if found.truncated { "+" } else { "" };
                ui.label(
                    RichText::new(format!(
                        "{count}{more} match{}",
                        if count == 1 { "" } else { "es" }
                    ))
                    .strong(),
                );
            }
        }
        ui.add_space(4.0);

        // Highlight using the matches computed from this frame's text. If the text changed
        // during this frame's edit, the old ranges no longer fit, so skip highlighting once.
        let snapshot = self.text.clone();
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let text = buffer.as_str();
            let ranges = if text == snapshot {
                ranges.as_slice()
            } else {
                &[]
            };
            ui.painter()
                .layout_job(highlighted(ui, text, ranges, wrap_width))
        };
        egui::ScrollArea::vertical()
            .id_salt("regex_text")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(6)
                        .hint_text("Text to search")
                        .layouter(&mut layouter),
                );
            });
        ui.add_space(8.0);

        if let Ok(found) = &found
            && !found.matches.is_empty()
        {
            self.matches_table(ui, found);
            ui.add_space(8.0);
        }

        egui::CollapsingHeader::new("Replace")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("With");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.replacement)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(300.0),
                    );
                    ui.weak("$1 or ${name} insert groups, $$ a literal $");
                });
                match regex_test::replace(&self.pattern, self.flags, &self.text, &self.replacement)
                {
                    Ok(result) => {
                        copy_button(ui, "copy_replaced", "Copy result", &result);
                        output_box(ui, "regex_replaced", &result, 200.0);
                    }
                    Err(error) => error_label(ui, error),
                }
            });
        egui::CollapsingHeader::new("Cheat sheet")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("regex_cheat_sheet")
                    .num_columns(2)
                    .striped(true)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for (token, meaning) in CHEAT_SHEET {
                            ui.label(RichText::new(*token).monospace());
                            ui.label(*meaning);
                            ui.end_row();
                        }
                    });
            });
    }
}
