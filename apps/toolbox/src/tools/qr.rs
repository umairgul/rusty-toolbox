use super::{Category, Tool, error_label, save_file};
use egui::{ColorImage, TextureHandle, TextureOptions};
use toolbox_core::qr::{self, ErrorCorrection, QrMatrix, WifiSecurity};

/// On-screen preview size in points.
const PREVIEW_SIZE: f32 = 260.0;
/// Pixels per module for saved and copied images.
const EXPORT_SCALE: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Text,
    Wifi,
}

#[derive(Clone, PartialEq, Eq)]
struct Input {
    mode: Mode,
    text: String,
    ssid: String,
    password: String,
    security: WifiSecurity,
    hidden: bool,
    error_correction: ErrorCorrection,
}

impl Input {
    fn payload(&self) -> String {
        match self.mode {
            Mode::Text => self.text.clone(),
            Mode::Wifi if self.ssid.is_empty() => String::new(),
            Mode::Wifi => qr::wifi_payload(&self.ssid, &self.password, self.security, self.hidden),
        }
    }
}

pub struct QrTool {
    input: Input,
    /// The input the current matrix and texture were made from.
    rendered: Option<Input>,
    matrix: Result<QrMatrix, qr::Error>,
    texture: Option<TextureHandle>,
    status: Option<String>,
}

impl Default for QrTool {
    fn default() -> Self {
        Self {
            input: Input {
                mode: Mode::Text,
                text: "https://example.com".into(),
                ssid: String::new(),
                password: String::new(),
                security: WifiSecurity::Wpa,
                hidden: false,
                error_correction: ErrorCorrection::Medium,
            },
            rendered: None,
            matrix: Err(qr::Error::Empty),
            texture: None,
            status: None,
        }
    }
}

impl QrTool {
    fn render_if_changed(&mut self, ctx: &egui::Context) {
        if self.rendered.as_ref() == Some(&self.input) {
            return;
        }
        self.matrix = QrMatrix::encode(&self.input.payload(), self.input.error_correction);
        // One texel per module, drawn with nearest-neighbour filtering so edges stay crisp.
        self.texture = self.matrix.as_ref().ok().map(|matrix| {
            let size = matrix.pixel_size(1);
            ctx.load_texture(
                "qr_code",
                ColorImage::from_gray([size, size], &matrix.to_pixels(1)),
                TextureOptions::NEAREST,
            )
        });
        self.rendered = Some(self.input.clone());
        self.status = None;
    }

    fn input_ui(&mut self, ui: &mut egui::Ui) {
        let input = &mut self.input;
        match input.mode {
            Mode::Text => {
                ui.add(
                    egui::TextEdit::multiline(&mut input.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Text or URL"),
                );
            }
            Mode::Wifi => {
                egui::Grid::new("wifi_fields")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Network name");
                        ui.add(egui::TextEdit::singleline(&mut input.ssid).hint_text("SSID"));
                        ui.end_row();
                        ui.label("Security");
                        egui::ComboBox::from_id_salt("wifi_security")
                            .selected_text(input.security.label())
                            .show_ui(ui, |ui| {
                                for security in WifiSecurity::ALL {
                                    ui.selectable_value(
                                        &mut input.security,
                                        security,
                                        security.label(),
                                    );
                                }
                            });
                        ui.end_row();
                        if input.security != WifiSecurity::Open {
                            ui.label("Password");
                            ui.add(egui::TextEdit::singleline(&mut input.password).password(true));
                            ui.end_row();
                        }
                        ui.label("");
                        ui.checkbox(&mut input.hidden, "Hidden network");
                        ui.end_row();
                    });
            }
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Error correction");
            egui::ComboBox::from_id_salt("qr_error_correction")
                .selected_text(input.error_correction.label())
                .show_ui(ui, |ui| {
                    for level in ErrorCorrection::ALL {
                        ui.selectable_value(&mut input.error_correction, level, level.label());
                    }
                });
        });
    }
}

fn export_ui(ui: &mut egui::Ui, matrix: &QrMatrix, status: &mut Option<String>) {
    ui.horizontal(|ui| {
        let size = matrix.pixel_size(EXPORT_SCALE);
        if ui.button("Copy image").clicked() {
            ui.ctx().copy_image(ColorImage::from_gray(
                [size, size],
                &matrix.to_pixels(EXPORT_SCALE),
            ));
            *status = Some("Image copied to clipboard".into());
        }
        if ui.button("Save PNG…").clicked() {
            *status = match encode_png(matrix) {
                Some(png) => save_file("qr-code.png", "PNG image", "png", &png),
                None => Some("Couldn't create the PNG".into()),
            };
        }
        if ui.button("Save SVG…").clicked() {
            *status = save_file(
                "qr-code.svg",
                "SVG image",
                "svg",
                matrix.to_svg().as_bytes(),
            );
        }
    });
    if let Some(status) = status {
        ui.weak(status.as_str());
    }
}

fn encode_png(matrix: &QrMatrix) -> Option<Vec<u8>> {
    let size = matrix.pixel_size(EXPORT_SCALE) as u32;
    let image = image::GrayImage::from_raw(size, size, matrix.to_pixels(EXPORT_SCALE))?;
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

impl Tool for QrTool {
    fn name(&self) -> &'static str {
        "QR Codes"
    }

    fn description(&self) -> &'static str {
        "QR codes for text, links and Wi-Fi networks. Export as PNG or SVG."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.input.mode, Mode::Text, "Text / URL");
            ui.selectable_value(&mut self.input.mode, Mode::Wifi, "Wi-Fi");
        });
        ui.add_space(8.0);
        self.input_ui(ui);
        self.render_if_changed(ui.ctx());
        ui.add_space(12.0);

        match (&self.matrix, &self.texture) {
            (Ok(matrix), Some(texture)) => {
                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(egui::vec2(PREVIEW_SIZE, PREVIEW_SIZE)),
                );
                ui.weak(format!("{0}×{0} modules", matrix.width()));
                ui.add_space(8.0);
                export_ui(ui, matrix, &mut self.status);
            }
            (Err(qr::Error::Empty), _) if self.input.mode == Mode::Wifi => {
                ui.weak("Enter the network name (SSID) to make a Wi-Fi QR code.");
            }
            (Err(error), _) => error_label(ui, error),
            (Ok(_), None) => {}
        }
    }
}
