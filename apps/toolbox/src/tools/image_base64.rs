use super::image_widgets::{file_input, file_name, preview_texture, show_preview};
use super::{Category, Tool, ValueRow, error_label, human_size, save_file, value_rows};
use egui::{RichText, TextureHandle};
use toolbox_core::images::{self, data_uri};

/// Above this, a data URI is usually worse than a separate file.
const LARGE_IMAGE: usize = 32 * 1024;
/// Characters of a data URI shown before cutting it short.
const SHOWN_CHARS: usize = 96;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    ToBase64,
    ToImage,
}

struct Encoded {
    name: String,
    size: usize,
    uri: String,
    preview: Option<TextureHandle>,
}

struct Decoded {
    mime: String,
    bytes: Vec<u8>,
    preview: Option<(TextureHandle, u32, u32)>,
}

pub struct ImageBase64Tool {
    mode: Mode,
    encoded: Option<Result<Encoded, String>>,
    input: String,
    decoded_from: String,
    decoded: Option<Result<Decoded, String>>,
    status: Option<String>,
}

impl Default for ImageBase64Tool {
    fn default() -> Self {
        Self {
            mode: Mode::ToBase64,
            encoded: None,
            input: String::new(),
            decoded_from: String::new(),
            decoded: None,
            status: None,
        }
    }
}

fn shorten(text: &str) -> String {
    if text.len() <= SHOWN_CHARS {
        text.to_owned()
    } else {
        format!("{}… ({} characters)", &text[..SHOWN_CHARS], text.len())
    }
}

impl ImageBase64Tool {
    fn encode_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = file_input(ui, "Choose image…", false).into_iter().next() {
            self.encoded = Some(
                std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .map(|bytes| {
                        let preview = images::decode(&bytes).ok().map(|loaded| {
                            preview_texture(ui.ctx(), "base64_preview", &loaded.image, 256)
                        });
                        Encoded {
                            name: file_name(&path),
                            size: bytes.len(),
                            uri: data_uri::encode(&bytes),
                            preview,
                        }
                    }),
            );
        }
        ui.add_space(8.0);
        match &self.encoded {
            None => {
                ui.weak("Choose an image to get it as a data URI for HTML, CSS or Markdown.");
            }
            Some(Err(error)) => error_label(ui, error),
            Some(Ok(encoded)) => {
                ui.horizontal_top(|ui| {
                    if let Some(preview) = &encoded.preview {
                        show_preview(ui, preview, 96.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&encoded.name).strong());
                        ui.weak(format!("{} file, {} as Base64", human_size(encoded.size as u64), human_size(encoded.uri.len() as u64)));
                        if encoded.size > LARGE_IMAGE {
                            ui.weak("Data URIs are a third larger than the file and can't be cached separately; they suit small icons best.");
                        }
                    });
                });
                ui.add_space(8.0);
                let mut rows = vec![ValueRow::with_copy(
                    "Data URI",
                    shorten(&encoded.uri),
                    encoded.uri.clone(),
                )];
                rows.extend(data_uri::snippets(&encoded.uri).into_iter().map(
                    |(label, snippet)| ValueRow::with_copy(label, shorten(&snippet), snippet),
                ));
                value_rows(ui, "base64_rows", &rows);
            }
        }
    }

    fn decode_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("base64_input").max_height(160.0).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.input)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
                    .hint_text("Paste a data URI (data:image/png;base64,…), CSS url(…), <img src=…> or plain Base64"),
            );
        });
        if self.input != self.decoded_from {
            self.decoded_from = self.input.clone();
            self.status = None;
            self.decoded = (!self.input.trim().is_empty()).then(|| {
                data_uri::decode(&self.input)
                    .map_err(|e| e.to_string())
                    .map(|(mime, bytes)| {
                        let preview = images::decode(&bytes).ok().map(|loaded| {
                            let (w, h) = (loaded.image.width(), loaded.image.height());
                            (
                                preview_texture(ui.ctx(), "base64_decoded", &loaded.image, 320),
                                w,
                                h,
                            )
                        });
                        Decoded {
                            mime,
                            bytes,
                            preview,
                        }
                    })
            });
        }
        ui.add_space(8.0);
        match &self.decoded {
            None => {}
            Some(Err(error)) => error_label(ui, error),
            Some(Ok(decoded)) => {
                ui.horizontal(|ui| {
                    let size = decoded
                        .preview
                        .as_ref()
                        .map_or(String::new(), |(_, w, h)| format!(" · {w}×{h}"));
                    ui.label(
                        RichText::new(format!(
                            "{}{size} · {}",
                            decoded.mime,
                            human_size(decoded.bytes.len() as u64)
                        ))
                        .strong(),
                    );
                    if ui.button("Save image…").clicked() {
                        let extension = images::extension_for_mime(&decoded.mime);
                        self.status = save_file(
                            &format!("image.{extension}"),
                            "Image",
                            extension,
                            &decoded.bytes,
                        );
                    }
                });
                if let Some(status) = &self.status {
                    ui.weak(status.as_str());
                }
                if let Some((texture, _, _)) = &decoded.preview {
                    ui.add_space(4.0);
                    show_preview(ui, texture, 320.0);
                }
            }
        }
    }
}

impl Tool for ImageBase64Tool {
    fn name(&self) -> &'static str {
        "Base64 Images"
    }

    fn description(&self) -> &'static str {
        "Turn images into data URIs for HTML, CSS and Markdown, or data URIs back into image files."
    }

    fn category(&self) -> Category {
        Category::Images
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, Mode::ToBase64, "Image to Base64");
            ui.selectable_value(&mut self.mode, Mode::ToImage, "Base64 to image");
        });
        ui.add_space(8.0);
        match self.mode {
            Mode::ToBase64 => self.encode_ui(ui),
            Mode::ToImage => self.decode_ui(ui),
        }
    }
}
