use super::image_widgets::{
    Task, file_input, file_name, preview_texture, reveal, short_path, show_preview,
};
use super::{Category, Tool, copy_button, error_label, output_box, status_label};
use egui::{RichText, TextureHandle};
use image::DynamicImage;
use std::path::PathBuf;
use toolbox_core::images::icons::{self, FAVICON_HTML, Fit, IconSet};
use toolbox_core::images::{self};

/// Below this, larger icons come out blurry.
const RECOMMENDED_SIZE: u32 = 1024;

/// The folder and the files written there, or why generating failed.
type Generated = Result<(PathBuf, Vec<String>), String>;

struct Source {
    path: PathBuf,
    image: DynamicImage,
    preview: Option<(Fit, TextureHandle)>,
}

pub struct IconGeneratorTool {
    source: Option<Result<Source, String>>,
    fit: Fit,
    sets: Vec<IconSet>,
    name: String,
    task: Option<Task<Generated>>,
    result: Option<Generated>,
}

impl Default for IconGeneratorTool {
    fn default() -> Self {
        Self {
            source: None,
            fit: Fit::Pad,
            sets: IconSet::ALL.to_vec(),
            name: "My App".into(),
            task: None,
            result: None,
        }
    }
}

impl IconGeneratorTool {
    fn start(&mut self, ctx: &egui::Context, folder: PathBuf) {
        let Some(Ok(source)) = &self.source else {
            return;
        };
        let (image, fit, sets, name) = (
            source.image.clone(),
            self.fit,
            self.sets.clone(),
            self.name.clone(),
        );
        self.result = None;
        self.task = Some(Task::spawn(ctx, 1, move |handle| {
            let files = icons::generate(&image, fit, &sets, &name).map_err(|e| e.to_string())?;
            let mut written = Vec::new();
            for file in files {
                std::fs::write(folder.join(&file.name), &file.bytes)
                    .map_err(|e| format!("Couldn't save {}: {e}", file.name))?;
                written.push(file.name);
            }
            handle.advance();
            Ok((folder, written))
        }));
    }
}

impl Tool for IconGeneratorTool {
    fn name(&self) -> &'static str {
        "Icon Generator"
    }

    fn description(&self) -> &'static str {
        "Make website favicons, a macOS .icns, a Windows .ico and PNGs at every size from one image or SVG."
    }

    fn category(&self) -> Category {
        Category::Images
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = file_input(ui, "Choose image…", false).into_iter().next() {
            self.result = None;
            self.source = Some(
                images::load(&path)
                    .map_err(|e| e.to_string())
                    .map(|loaded| Source {
                        path,
                        image: loaded.image,
                        preview: None,
                    }),
            );
        }
        ui.add_space(8.0);
        let Some(source) = &mut self.source else {
            ui.weak(
                "Choose a square image of at least 1024×1024, or an SVG, for the sharpest icons.",
            );
            return;
        };
        let source = match source {
            Ok(source) => source,
            Err(error) => {
                error_label(ui, error);
                return;
            }
        };

        // Rebuild the preview when the fit changes, to show the icon as it will look.
        if source
            .preview
            .as_ref()
            .is_none_or(|(fit, _)| *fit != self.fit)
        {
            let square = DynamicImage::ImageRgba8(icons::square(&source.image, self.fit));
            source.preview = Some((
                self.fit,
                preview_texture(ui.ctx(), "icon_preview", &square, 256),
            ));
        }
        ui.horizontal_top(|ui| {
            if let Some((_, texture)) = &source.preview {
                egui::Frame::NONE.fill(ui.visuals().extreme_bg_color).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
                    show_preview(ui, texture, 128.0);
                });
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(file_name(&source.path)).strong());
                let (w, h) = (source.image.width(), source.image.height());
                ui.weak(format!("{w}×{h}"));
                if w.min(h) < RECOMMENDED_SIZE {
                    ui.weak(format!("Smaller than {RECOMMENDED_SIZE}×{RECOMMENDED_SIZE}: the largest icons will be upscaled and look soft."));
                }
                if w != h {
                    ui.horizontal(|ui| {
                        ui.label("Not square:");
                        for fit in Fit::ALL {
                            ui.radio_value(&mut self.fit, fit, fit.label());
                        }
                    });
                }
            });
        });
        ui.add_space(12.0);

        ui.label(RichText::new("Make").strong());
        for set in IconSet::ALL {
            let mut on = self.sets.contains(&set);
            ui.horizontal(|ui| {
                ui.checkbox(&mut on, set.label());
                ui.weak(set.description());
            });
            if on != self.sets.contains(&set) {
                if on {
                    self.sets.push(set)
                } else {
                    self.sets.retain(|s| *s != set)
                }
            }
        }
        if self.sets.contains(&IconSet::Favicons) {
            ui.horizontal(|ui| {
                ui.label("App name");
                ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(200.0));
                ui.weak("for site.webmanifest");
            });
        }
        ui.add_space(12.0);

        if let Some(task) = &self.task {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Generating…");
            });
            if let Some(result) = task.poll(ui.ctx()) {
                self.result = Some(result);
                self.task = None;
            }
        } else if ui
            .add_enabled(
                !self.sets.is_empty(),
                egui::Button::new(RichText::new("Generate…").strong()),
            )
            .clicked()
            && let Some(folder) = rfd::FileDialog::new()
                .set_title("Choose a folder for the icons")
                .pick_folder()
        {
            self.start(ui.ctx(), folder);
        }

        match &self.result {
            Some(Ok((folder, written))) => {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    status_label(
                        ui,
                        true,
                        format!("Saved {} files to {}", written.len(), short_path(folder)),
                    );
                    if let Some(first) = written.first()
                        && ui.small_button("Show in Finder").clicked()
                    {
                        reveal(&folder.join(first));
                    }
                });
                ui.weak(written.join(", "));
                if written.iter().any(|name| name == "favicon.ico") {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Add to your page's <head>").strong());
                        copy_button(ui, "copy_favicon_html", "Copy", FAVICON_HTML);
                    });
                    output_box(ui, "favicon_html", FAVICON_HTML, 120.0);
                }
            }
            Some(Err(error)) => error_label(ui, error),
            None => {}
        }
    }
}
