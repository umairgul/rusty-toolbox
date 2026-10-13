//! Widgets shared by the image tools: choosing or dropping files, previews, revealing files
//! in Finder and running slow work in the background.

use egui::{ColorImage, RichText, TextureHandle, TextureOptions};
use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use toolbox_core::images::INPUT_EXTENSIONS;

/// A "Choose…" button plus drag-and-drop. Returns the files picked or dropped this frame.
pub fn file_input(ui: &mut egui::Ui, button: &str, multiple: bool) -> Vec<PathBuf> {
    let mut chosen = Vec::new();
    ui.horizontal(|ui| {
        if ui.button(button).clicked() {
            let dialog = rfd::FileDialog::new().add_filter("Images", INPUT_EXTENSIONS);
            if multiple {
                chosen.extend(dialog.pick_files().unwrap_or_default());
            } else if let Some(path) = dialog.pick_file() {
                chosen.push(path);
            }
        }
        let hovering = ui.input(|input| !input.raw.hovered_files.is_empty());
        let hint = if hovering {
            RichText::new("Drop to open").strong()
        } else {
            RichText::new("or drop images onto the window").weak()
        };
        ui.label(hint);
    });
    let dropped: Vec<PathBuf> = ui.input(|input| {
        input
            .raw
            .dropped_files
            .iter()
            .map(|file| file.path().to_owned())
            .collect()
    });
    chosen.extend(dropped.into_iter().filter(|path| has_image_extension(path)));
    if !multiple {
        chosen.truncate(1);
    }
    chosen
}

fn has_image_extension(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        INPUT_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str())
    })
}

/// A path short enough for one line: long ones become `…/parent/name`. Show the full path
/// on hover.
pub fn short_path(path: &Path) -> String {
    const MAX_CHARS: usize = 48;
    let full = path.display().to_string();
    if full.chars().count() <= MAX_CHARS {
        return full;
    }
    let mut tail: Vec<String> = path
        .iter()
        .rev()
        .take(2)
        .map(|part| part.to_string_lossy().into_owned())
        .collect();
    tail.reverse();
    format!("…/{}", tail.join("/"))
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// A texture of `image` scaled to fit `max` pixels, for previews.
pub fn preview_texture(
    ctx: &egui::Context,
    name: &str,
    image: &DynamicImage,
    max: u32,
) -> TextureHandle {
    let small = if image.width() > max || image.height() > max {
        image.thumbnail(max, max)
    } else {
        image.clone()
    };
    let rgba = small.to_rgba8();
    let pixels = ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    );
    ctx.load_texture(name, pixels, TextureOptions::LINEAR)
}

/// Shows a texture within `max` points, keeping its aspect ratio.
pub fn show_preview(ui: &mut egui::Ui, texture: &TextureHandle, max: f32) {
    let [w, h] = texture.size().map(|v| v as f32);
    let scale = (max / w.max(h)).min(1.0);
    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(w * scale, h * scale)));
}

/// Opens a Finder window with the file selected (Explorer or the file manager elsewhere).
pub fn reveal(path: &Path) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
    } else {
        std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .spawn()
    };
    if let Err(error) = result {
        eprintln!("couldn't reveal {}: {error}", path.display());
    }
}

/// Work running on a background thread, with progress and cancellation.
pub struct Task<T> {
    result: mpsc::Receiver<T>,
    progress: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    pub total: usize,
}

/// Lets the work report progress and notice cancellation.
#[derive(Clone)]
pub struct TaskHandle {
    progress: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    ctx: egui::Context,
}

impl TaskHandle {
    pub fn advance(&self) {
        self.progress.fetch_add(1, Ordering::Relaxed);
        self.ctx.request_repaint();
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn(
        ctx: &egui::Context,
        total: usize,
        work: impl FnOnce(TaskHandle) -> T + Send + 'static,
    ) -> Self {
        let (sender, result) = mpsc::channel();
        let progress = Arc::new(AtomicUsize::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let handle = TaskHandle {
            progress: progress.clone(),
            cancel: cancel.clone(),
            ctx: ctx.clone(),
        };
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // The receiver is gone if the tool was reset; nothing to deliver then.
            let _ = sender.send(work(handle));
            ctx.request_repaint();
        });
        Self {
            result,
            progress,
            cancel,
            total,
        }
    }

    /// The result, once the work has finished.
    pub fn poll(&self, ctx: &egui::Context) -> Option<T> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(_) => {
                ctx.request_repaint_after(Duration::from_millis(100));
                None
            }
        }
    }

    pub fn done(&self) -> usize {
        self.progress.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// A progress bar with a Cancel button.
    pub fn progress_ui(&self, ui: &mut egui::Ui, what: &str) {
        ui.horizontal(|ui| {
            let fraction = if self.total == 0 {
                0.0
            } else {
                self.done() as f32 / self.total as f32
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(300.0)
                    .text(format!(
                        "{what} {} of {}",
                        self.done().min(self.total),
                        self.total
                    )),
            );
            if ui.button("Cancel").clicked() {
                self.cancel();
            }
        });
    }
}
