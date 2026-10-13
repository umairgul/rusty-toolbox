use super::{Category, Tool, copy_button, error_label, output_box};
use toolbox_core::encoding::{self, Codec, Error};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Encode,
    Decode,
}

pub struct EncodeTool {
    codec: Codec,
    direction: Direction,
    options: encoding::Options,
    input: String,
}

impl Default for EncodeTool {
    fn default() -> Self {
        Self {
            codec: Codec::Base64,
            direction: Direction::Encode,
            options: encoding::Options::default(),
            input: String::new(),
        }
    }
}

impl EncodeTool {
    fn output(&self) -> Result<String, Error> {
        match self.direction {
            Direction::Encode => Ok(encoding::encode(self.codec, &self.input, &self.options)),
            Direction::Decode => encoding::decode(self.codec, &self.input, &self.options),
        }
    }

    fn options_ui(&mut self, ui: &mut egui::Ui) {
        let options = &mut self.options;
        match self.codec {
            Codec::Base64 => {
                ui.checkbox(&mut options.base64_url_safe, "URL-safe alphabet (- and _)");
                ui.add_enabled(
                    self.direction == Direction::Encode,
                    egui::Checkbox::new(&mut options.base64_padding, "Padding (=)"),
                );
                if self.direction == Direction::Decode {
                    ui.weak("Decoding accepts either alphabet, with or without padding.");
                }
            }
            Codec::Url => {
                ui.checkbox(
                    &mut options.url_plus_for_space,
                    "Spaces as + (HTML form encoding)",
                );
            }
            Codec::Html => {
                if self.direction == Direction::Encode {
                    ui.checkbox(
                        &mut options.html_encode_non_ascii,
                        "Encode all non-ASCII characters",
                    );
                }
            }
            Codec::Hex => {}
        }
    }
}

impl Tool for EncodeTool {
    fn name(&self) -> &'static str {
        "Encode / Decode"
    }

    fn description(&self) -> &'static str {
        "Base64, URL percent-encoding, HTML entities and hex."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for codec in Codec::ALL {
                ui.selectable_value(&mut self.codec, codec, codec.label());
            }
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.direction, Direction::Encode, "Encode");
            ui.radio_value(&mut self.direction, Direction::Decode, "Decode");
        });
        self.options_ui(ui);
        ui.add_space(8.0);

        let hint = match self.direction {
            Direction::Encode => "Text to encode",
            Direction::Decode => "Text to decode",
        };
        ui.add(
            egui::TextEdit::multiline(&mut self.input)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(5)
                .hint_text(hint),
        );
        ui.add_space(8.0);

        let output = self.output();
        let text = output.as_ref().map(String::as_str).unwrap_or_default();
        ui.horizontal(|ui| {
            copy_button(ui, "copy_encoded", "Copy result", text);
            if ui
                .add_enabled(output.is_ok(), egui::Button::new("Swap"))
                .on_hover_text("Use the result as input and reverse the direction")
                .clicked()
            {
                self.input = text.to_owned();
                self.direction = match self.direction {
                    Direction::Encode => Direction::Decode,
                    Direction::Decode => Direction::Encode,
                };
            }
            if self.direction == Direction::Encode {
                ui.weak(format!(
                    "{} characters in, {} out",
                    self.input.chars().count(),
                    text.chars().count()
                ));
            }
        });
        ui.add_space(4.0);

        match &output {
            Ok(text) => output_box(ui, "encode_output", text, 300.0),
            Err(Error::NotText { hex, .. }) => {
                error_label(ui, output.as_ref().unwrap_err());
                ui.weak("As hex:");
                output_box(ui, "encode_output_hex", hex, 200.0);
            }
            Err(error) => error_label(ui, error),
        }
    }
}
