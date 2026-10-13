use super::{Category, Tool, ValueRow, error_label, status_label, value_rows};
use egui::RichText;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use toolbox_core::hash::{self, Digests};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Text,
    File,
}

/// A file being hashed on a background thread.
struct FileJob {
    path: PathBuf,
    size: u64,
    progress: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<std::io::Result<Option<Digests>>>,
}

pub struct HashesTool {
    mode: Mode,
    text: String,
    text_digests: Digests,
    hashed_text: String,
    job: Option<FileJob>,
    file_result: Option<(PathBuf, Result<Digests, String>)>,
    uppercase: bool,
    expected: String,
}

impl Default for HashesTool {
    fn default() -> Self {
        Self {
            mode: Mode::Text,
            text: String::new(),
            text_digests: hash::hash_bytes(b""),
            hashed_text: String::new(),
            job: None,
            file_result: None,
            uppercase: false,
            expected: String::new(),
        }
    }
}

impl HashesTool {
    fn start_file(&mut self, path: PathBuf) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let (thread_path, thread_progress, thread_cancel) =
            (path.clone(), progress.clone(), cancel.clone());
        std::thread::spawn(move || {
            // The receiver is gone if a newer file replaced this job; nothing to report then.
            let _ = sender.send(hash::hash_file(
                &thread_path,
                &thread_progress,
                &thread_cancel,
            ));
        });
        self.job = Some(FileJob {
            path,
            size,
            progress,
            cancel,
            result,
        });
        self.file_result = None;
    }

    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        match job.result.try_recv() {
            Ok(Ok(Some(digests))) => self.file_result = Some((job.path.clone(), Ok(digests))),
            Ok(Ok(None)) => {}
            Ok(Err(error)) => self.file_result = Some((job.path.clone(), Err(error.to_string()))),
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(100));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {}
        }
        self.job = None;
    }

    fn file_ui(&mut self, ui: &mut egui::Ui) {
        let dropped = ui.input(|input| {
            input
                .raw
                .dropped_files
                .first()
                .map(|file| file.path().to_owned())
        });
        if let Some(path) = dropped {
            self.start_file(path);
        }
        let hovering = ui.input(|input| !input.raw.hovered_files.is_empty());

        ui.horizontal(|ui| {
            if ui.button("Choose file…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.start_file(path);
            }
            let hint = if hovering {
                RichText::new("Drop to hash it").strong()
            } else {
                RichText::new("or drop a file onto the window").weak()
            };
            ui.label(hint);
        });
        ui.add_space(8.0);

        self.poll_job(ui.ctx());
        if let Some(job) = &self.job {
            let done = job.progress.load(Ordering::Relaxed);
            let fraction = if job.size == 0 {
                0.0
            } else {
                done as f32 / job.size as f32
            };
            ui.label(file_name(&job.path));
            ui.horizontal(|ui| {
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_width(300.0)
                        .text(format!("{} of {}", human_size(done), human_size(job.size))),
                );
                if ui.button("Cancel").clicked() {
                    job.cancel.store(true, Ordering::Relaxed);
                }
            });
        }
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1000.0 && unit < UNITS.len() - 1 {
        size /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} bytes")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

impl Tool for HashesTool {
    fn name(&self) -> &'static str {
        "Hashes"
    }

    fn description(&self) -> &'static str {
        "MD5, SHA-1, SHA-2 and BLAKE3 hashes of text or files, and checksum verification."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Text, "Text");
            ui.selectable_value(&mut self.mode, Mode::File, "File");
        });
        ui.add_space(8.0);

        let digests = match self.mode {
            Mode::Text => {
                ui.add(
                    egui::TextEdit::multiline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text("Text to hash (UTF-8)"),
                );
                if self.text != self.hashed_text {
                    self.text_digests = hash::hash_bytes(self.text.as_bytes());
                    self.hashed_text = self.text.clone();
                }
                Some(&self.text_digests)
            }
            Mode::File => {
                self.file_ui(ui);
                match &self.file_result {
                    Some((path, Ok(digests))) if self.job.is_none() => {
                        ui.label(RichText::new(file_name(path)).strong());
                        Some(digests)
                    }
                    Some((path, Err(error))) if self.job.is_none() => {
                        error_label(ui, format!("Couldn't read {}: {error}", file_name(path)));
                        None
                    }
                    _ => None,
                }
            }
        };
        let Some(digests) = digests else { return };
        ui.add_space(8.0);

        ui.checkbox(&mut self.uppercase, "Uppercase");
        let rows: Vec<ValueRow> = digests
            .iter()
            .map(|(algorithm, digest)| {
                let digest = if self.uppercase {
                    digest.to_uppercase()
                } else {
                    digest.to_owned()
                };
                let label = if algorithm.is_insecure() {
                    format!("{} (legacy)", algorithm.label())
                } else {
                    algorithm.label().to_owned()
                };
                ValueRow::new(label, digest)
            })
            .collect();
        value_rows(ui, "hash_rows", &rows);
        ui.weak("MD5 and SHA-1 are fine for checksums but broken for security uses.");
        ui.add_space(12.0);

        ui.label("Verify a checksum");
        ui.add(
            egui::TextEdit::singleline(&mut self.expected)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .hint_text("Paste the expected hash"),
        );
        if !self.expected.trim().is_empty() {
            match digests.find_match(&self.expected) {
                Some(algorithm) => {
                    status_label(ui, true, format!("Matches the {} hash", algorithm.label()))
                }
                None => status_label(ui, false, "Doesn't match any hash above"),
            }
        }
    }
}
