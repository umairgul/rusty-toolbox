use super::image_widgets::{Task, file_input, file_name, reveal, short_path};
use super::{Category, Tool, error_label, human_size};
use egui::RichText;
use std::path::{Path, PathBuf};
use toolbox_core::images::convert::{self, OutputFormat, Resize};
use toolbox_core::images::{self, Probe};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeMode {
    Original,
    Fit,
    Scale,
    Exact,
}

impl ResizeMode {
    const ALL: [ResizeMode; 4] = [
        ResizeMode::Original,
        ResizeMode::Fit,
        ResizeMode::Scale,
        ResizeMode::Exact,
    ];

    fn label(self) -> &'static str {
        match self {
            ResizeMode::Original => "Keep original size",
            ResizeMode::Fit => "Fit within",
            ResizeMode::Scale => "Scale by percentage",
            ResizeMode::Exact => "Exact size (may stretch)",
        }
    }
}

struct Entry {
    path: PathBuf,
    probe: Result<Probe, String>,
}

/// One converted file, or why it failed.
struct Outcome {
    input: PathBuf,
    result: Result<(PathBuf, u64, u64), String>,
}

pub struct ImageConverterTool {
    files: Vec<Entry>,
    format: OutputFormat,
    quality: u8,
    background: [u8; 3],
    resize_mode: ResizeMode,
    width: u32,
    height: u32,
    percent: u32,
    allow_upscale: bool,
    /// `None` saves next to each original.
    output_folder: Option<PathBuf>,
    task: Option<Task<Vec<Outcome>>>,
    outcomes: Vec<Outcome>,
}

impl Default for ImageConverterTool {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            format: OutputFormat::Jpeg,
            quality: 85,
            background: [255, 255, 255],
            resize_mode: ResizeMode::Original,
            width: 1920,
            height: 1080,
            percent: 50,
            allow_upscale: false,
            output_folder: None,
            task: None,
            outcomes: Vec::new(),
        }
    }
}

impl ImageConverterTool {
    fn add(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if self.files.iter().any(|entry| entry.path == path) {
                continue;
            }
            let probe = images::probe(&path).map_err(|e| e.to_string());
            self.files.push(Entry { path, probe });
        }
    }

    fn options(&self) -> convert::Options {
        let resize = match self.resize_mode {
            ResizeMode::Original => Resize::Original,
            ResizeMode::Fit => Resize::Fit {
                width: self.width,
                height: self.height,
            },
            ResizeMode::Scale => Resize::Scale(self.percent),
            ResizeMode::Exact => Resize::Exact {
                width: self.width,
                height: self.height,
            },
        };
        convert::Options {
            format: self.format,
            quality: self.quality,
            resize,
            allow_upscale: self.allow_upscale,
            background: self.background,
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        let inputs: Vec<PathBuf> = self.files.iter().map(|entry| entry.path.clone()).collect();
        let options = self.options();
        let folder = self.output_folder.clone();
        self.outcomes.clear();
        self.task = Some(Task::spawn(ctx, inputs.len(), move |handle| {
            let mut outcomes = Vec::new();
            for input in inputs {
                if handle.cancelled() {
                    break;
                }
                let folder = folder
                    .clone()
                    .or_else(|| input.parent().map(Path::to_owned))
                    .unwrap_or_default();
                let result = convert_one(&input, &folder, &options);
                outcomes.push(Outcome { input, result });
                handle.advance();
            }
            outcomes
        }));
    }

    fn file_list(&mut self, ui: &mut egui::Ui) {
        let mut remove = None;
        egui::Grid::new("converter_files")
            .num_columns(4)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (index, entry) in self.files.iter().enumerate() {
                    ui.label(file_name(&entry.path));
                    match &entry.probe {
                        Ok(probe) => {
                            ui.weak(match (probe.dimensions, probe.mime) {
                                (Some((w, h)), _) => format!("{w}×{h}"),
                                (None, Some("image/heic")) => "HEIC".into(),
                                (None, Some("image/svg+xml")) => "SVG (vector)".into(),
                                (None, _) => "unknown size".into(),
                            });
                            ui.weak(human_size(probe.file_size));
                        }
                        Err(error) => {
                            error_label(ui, error);
                            ui.label("");
                        }
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text("Remove from the list")
                        .clicked()
                    {
                        remove = Some(index);
                    }
                    ui.end_row();
                }
            });
        if let Some(index) = remove {
            self.files.remove(index);
        }
    }

    fn options_ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("converter_options")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Format");
                egui::ComboBox::from_id_salt("convert_format")
                    .selected_text(self.format.label())
                    .show_ui(ui, |ui| {
                        for format in OutputFormat::ALL {
                            ui.selectable_value(&mut self.format, format, format.label());
                        }
                    });
                ui.end_row();
                if self.format.has_quality() {
                    ui.label("Quality");
                    ui.add(egui::Slider::new(&mut self.quality, 1..=100));
                    ui.end_row();
                }
                if !self.format.has_alpha() {
                    ui.label("Background");
                    ui.horizontal(|ui| {
                        ui.color_edit_button_srgb(&mut self.background);
                        ui.weak("fills transparent areas");
                    });
                    ui.end_row();
                }
                ui.label("Size");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("convert_resize")
                        .selected_text(self.resize_mode.label())
                        .show_ui(ui, |ui| {
                            for mode in ResizeMode::ALL {
                                ui.selectable_value(&mut self.resize_mode, mode, mode.label());
                            }
                        });
                    match self.resize_mode {
                        ResizeMode::Original => {}
                        ResizeMode::Fit | ResizeMode::Exact => {
                            ui.add(
                                egui::DragValue::new(&mut self.width)
                                    .range(0..=20000)
                                    .suffix(" px wide"),
                            );
                            ui.label("×");
                            ui.add(
                                egui::DragValue::new(&mut self.height)
                                    .range(0..=20000)
                                    .suffix(" px high"),
                            );
                        }
                        ResizeMode::Scale => {
                            ui.add(
                                egui::DragValue::new(&mut self.percent)
                                    .range(1..=1000)
                                    .suffix("%"),
                            );
                        }
                    }
                });
                ui.end_row();
                if matches!(self.resize_mode, ResizeMode::Fit | ResizeMode::Scale) {
                    ui.label("");
                    ui.checkbox(&mut self.allow_upscale, "Allow making images larger");
                    ui.end_row();
                }
                ui.label("Save to");
                ui.horizontal(|ui| {
                    match &self.output_folder {
                        Some(folder) => {
                            ui.label(RichText::new(short_path(folder)).monospace())
                                .on_hover_text(folder.display().to_string());
                        }
                        None => {
                            ui.label("Next to the originals");
                        }
                    }
                    if ui.button("Choose folder…").clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        self.output_folder = Some(folder);
                    }
                    if self.output_folder.is_some() && ui.small_button("Reset").clicked() {
                        self.output_folder = None;
                    }
                });
                ui.end_row();
            });
        if let Some(note) = self.format.note() {
            ui.weak(note);
        }
        ui.weak("Converting removes EXIF and other metadata, and keeps the color profile. Originals are never overwritten.");
    }

    fn outcomes_ui(&self, ui: &mut egui::Ui) {
        let succeeded = self.outcomes.iter().filter(|o| o.result.is_ok()).count();
        ui.label(
            RichText::new(format!("Converted {succeeded} of {}", self.outcomes.len())).strong(),
        );
        egui::Grid::new("converter_results")
            .num_columns(4)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for outcome in &self.outcomes {
                    ui.label(file_name(&outcome.input));
                    match &outcome.result {
                        Ok((output, before, after)) => {
                            ui.label(file_name(output));
                            let change = (*after as f64 / (*before).max(1) as f64 - 1.0) * 100.0;
                            ui.weak(format!(
                                "{} to {} ({}{:.0}%)",
                                human_size(*before),
                                human_size(*after),
                                if change > 0.0 { "+" } else { "" },
                                change
                            ));
                            if ui.small_button("Show in Finder").clicked() {
                                reveal(output);
                            }
                        }
                        Err(error) => {
                            error_label(ui, error);
                            ui.label("");
                            ui.label("");
                        }
                    }
                    ui.end_row();
                }
            });
    }
}

fn convert_one(
    input: &Path,
    folder: &Path,
    options: &convert::Options,
) -> Result<(PathBuf, u64, u64), String> {
    let loaded = images::load(input).map_err(|e| e.to_string())?;
    let bytes = convert::encode(&loaded, options).map_err(|e| e.to_string())?;
    let output = convert::output_path(input, folder, options.format);
    std::fs::write(&output, &bytes).map_err(|e| format!("Couldn't save: {e}"))?;
    Ok((output, loaded.file_size, bytes.len() as u64))
}

impl Tool for ImageConverterTool {
    fn name(&self) -> &'static str {
        "Image Converter"
    }

    fn description(&self) -> &'static str {
        "Convert images between PNG, JPEG, WebP, AVIF and more, resizing and compressing in bulk. Opens HEIC and SVG too."
    }

    fn category(&self) -> Category {
        Category::Images
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let busy = self.task.is_some();
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                let added = file_input(ui, "Add images…", true);
                self.add(added);
                if !self.files.is_empty() && ui.button("Clear list").clicked() {
                    self.files.clear();
                    self.outcomes.clear();
                }
            });
        });
        if self.files.is_empty() {
            ui.add_space(8.0);
            ui.weak("Add one or more images to convert.");
            return;
        }
        ui.add_space(8.0);
        ui.add_enabled_ui(!busy, |ui| {
            self.file_list(ui);
            ui.add_space(12.0);
            self.options_ui(ui);
        });
        ui.add_space(12.0);

        if let Some(task) = &self.task {
            task.progress_ui(ui, "Converting");
            if let Some(outcomes) = task.poll(ui.ctx()) {
                self.outcomes = outcomes;
                self.task = None;
            }
        } else {
            let count = self.files.len();
            let label = if count == 1 {
                "Convert 1 image".to_owned()
            } else {
                format!("Convert {count} images")
            };
            if ui.button(RichText::new(label).strong()).clicked() {
                self.start(ui.ctx());
            }
        }
        if !self.outcomes.is_empty() {
            ui.add_space(12.0);
            self.outcomes_ui(ui);
        }
    }
}
