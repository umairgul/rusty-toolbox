use super::image_widgets::{file_input, file_name, preview_texture, reveal, show_preview};
use super::{Category, Tool, error_label, human_size, status_label};
use egui::{Color32, RichText, TextureHandle};
use std::path::PathBuf;
use toolbox_core::images::{self, metadata};

struct Opened {
    path: PathBuf,
    bytes: Vec<u8>,
    report: metadata::Report,
    preview: Option<TextureHandle>,
}

/// What happened after saving a clean copy.
struct Saved {
    path: PathBuf,
    removed: Vec<String>,
    verified_clean: bool,
    kept_orientation: bool,
}

#[derive(Default)]
pub struct ImageMetadataTool {
    opened: Option<Result<Opened, String>>,
    saved: Option<Result<Saved, String>>,
}

impl ImageMetadataTool {
    fn open(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.saved = None;
        self.opened = Some(
            std::fs::read(&path)
                .map_err(|e| e.to_string())
                .map(|bytes| {
                    let report = metadata::read(&bytes);
                    let preview = images::load(&path)
                        .ok()
                        .map(|loaded| preview_texture(ctx, "metadata_preview", &loaded.image, 320));
                    Opened {
                        path,
                        bytes,
                        report,
                        preview,
                    }
                }),
        );
    }

    fn save_clean_copy(opened: &Opened) -> Option<Result<Saved, String>> {
        let stem = opened
            .path
            .file_stem()
            .map_or("image".into(), |s| s.to_string_lossy().into_owned());
        let extension = opened
            .path
            .extension()
            .map_or("jpg".into(), |e| e.to_string_lossy().into_owned());
        let mut dialog = rfd::FileDialog::new().set_file_name(format!("{stem}-clean.{extension}"));
        if let Some(folder) = opened.path.parent() {
            dialog = dialog.set_directory(folder);
        }
        let path = dialog.save_file()?;
        if path == opened.path {
            return Some(Err(
                "Choose a different name: the original is kept untouched.".into(),
            ));
        }
        Some(
            metadata::strip(&opened.bytes)
                .map_err(|e| e.to_string())
                .and_then(|stripped| {
                    std::fs::write(&path, &stripped.bytes)
                        .map_err(|e| format!("Couldn't save: {e}"))?;
                    // Read the saved file back to confirm nothing is left.
                    let check = metadata::read(&std::fs::read(&path).map_err(|e| e.to_string())?);
                    Ok(Saved {
                        path,
                        removed: stripped.removed,
                        verified_clean: check.is_clean(),
                        kept_orientation: check.orientation != 1,
                    })
                }),
        )
    }
}

fn location_warning(ui: &mut egui::Ui, gps: metadata::Gps) {
    let warning = if ui.visuals().dark_mode {
        Color32::from_rgb(80, 52, 14)
    } else {
        Color32::from_rgb(255, 240, 210)
    };
    egui::Frame::NONE
        .fill(warning)
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("This image reveals where it was taken")
                    .strong()
                    .size(15.0),
            );
            let altitude = gps
                .altitude
                .map_or(String::new(), |a| format!(", {a:.0} m above sea level"));
            ui.label(format!(
                "Latitude {:.5}, longitude {:.5}{altitude}",
                gps.latitude, gps.longitude
            ));
            if ui
                .link("Show on OpenStreetMap")
                .on_hover_text("Opens your browser; the location is sent to openstreetmap.org")
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(gps.maps_url()));
            }
        });
}

fn report_ui(ui: &mut egui::Ui, report: &metadata::Report) {
    if report.is_clean() {
        status_label(ui, true, "No metadata found: this image is clean.");
        return;
    }
    let personal = report.sensitive_count();
    ui.label(
        RichText::new(format!(
            "{} EXIF field{}, {personal} of them personal, and {} other block{}",
            report.fields.len(),
            if report.fields.len() == 1 { "" } else { "s" },
            report.other.len(),
            if report.other.len() == 1 { "" } else { "s" }
        ))
        .strong(),
    );
    let personal_color = if ui.visuals().dark_mode {
        Color32::from_rgb(255, 180, 90)
    } else {
        Color32::from_rgb(170, 90, 0)
    };
    for group in ["Location", "Camera", "Image", "Other"] {
        let fields: Vec<&metadata::Field> =
            report.fields.iter().filter(|f| f.group == group).collect();
        if fields.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(format!("{group} ({})", fields.len()))
            .default_open(group != "Other")
            .show(ui, |ui| {
                egui::Grid::new(("metadata_fields", group))
                    .num_columns(3)
                    .striped(true)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        // The wrapping value goes last, where it gets the remaining width.
                        for field in fields {
                            ui.label(&field.name);
                            if field.sensitive {
                                ui.label(RichText::new("personal").small().color(personal_color));
                            } else {
                                ui.label("");
                            }
                            ui.add(
                                egui::Label::new(RichText::new(&field.value).monospace()).wrap(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
    if !report.other.is_empty() {
        ui.label(RichText::new("Also found").strong());
        for item in &report.other {
            ui.label(format!("· {item}"));
        }
    }
}

impl Tool for ImageMetadataTool {
    fn name(&self) -> &'static str {
        "Metadata Remover"
    }

    fn description(&self) -> &'static str {
        "See what a photo reveals (camera, date, GPS location) and save a clean copy. JPEG, PNG and WebP are cleaned without any quality loss."
    }

    fn category(&self) -> Category {
        Category::Images
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = file_input(ui, "Open image…", false).into_iter().next() {
            self.open(ui.ctx(), path);
        }
        ui.add_space(8.0);
        let Some(opened) = &self.opened else {
            ui.weak("Open a photo to see its hidden information.");
            return;
        };
        let opened = match opened {
            Ok(opened) => opened,
            Err(error) => {
                error_label(ui, error);
                return;
            }
        };

        ui.horizontal_top(|ui| {
            if let Some(preview) = &opened.preview {
                show_preview(ui, preview, 160.0);
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(file_name(&opened.path)).strong().size(15.0));
                ui.weak(human_size(opened.bytes.len() as u64));
                ui.add_space(8.0);
                let can_clean = opened.report.container != metadata::Container::Other;
                let clean = ui.add_enabled(can_clean && !opened.report.is_clean(), egui::Button::new(RichText::new("Save clean copy…").strong()));
                if clean.clicked()
                    && let Some(result) = Self::save_clean_copy(opened)
                {
                    self.saved = Some(result);
                }
                if !can_clean {
                    ui.weak("Lossless cleaning works for JPEG, PNG and WebP. To clean this file, convert it with the Image Converter.");
                }
            });
        });
        ui.add_space(8.0);

        match &self.saved {
            Some(Ok(saved)) => {
                ui.horizontal(|ui| {
                    let kept = if saved.kept_orientation {
                        " Kept only the rotation, so it still displays upright."
                    } else {
                        ""
                    };
                    if saved.verified_clean {
                        status_label(
                            ui,
                            true,
                            format!(
                                "Saved {}. Checked: no metadata left.{kept}",
                                file_name(&saved.path)
                            ),
                        );
                    } else {
                        status_label(
                            ui,
                            false,
                            format!(
                                "Saved {}, but some metadata remains.",
                                file_name(&saved.path)
                            ),
                        );
                    }
                    if ui.small_button("Show in Finder").clicked() {
                        reveal(&saved.path);
                    }
                });
                if !saved.removed.is_empty() {
                    ui.weak(format!("Removed: {}", saved.removed.join(", ")));
                }
                ui.add_space(8.0);
            }
            Some(Err(error)) => error_label(ui, error),
            None => {}
        }

        if let Some(gps) = opened.report.gps {
            location_warning(ui, gps);
            ui.add_space(8.0);
        }
        report_ui(ui, &opened.report);
    }
}
