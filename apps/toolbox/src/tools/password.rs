use super::{Category, Tool, copy_button, error_label, strength_bar};
use egui::RichText;
use toolbox_core::passphrase::{self, MAX_WORDS, MIN_WORDS, Separator};
use toolbox_core::password::{self, MAX_LENGTH, MIN_LENGTH};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Password,
    Passphrase,
}

pub struct PasswordTool {
    mode: Mode,
    password_options: password::Options,
    password: Result<String, password::Error>,
    passphrase_options: passphrase::Options,
    passphrase: String,
}

impl Default for PasswordTool {
    fn default() -> Self {
        let password_options = password::Options::default();
        let passphrase_options = passphrase::Options::default();
        Self {
            mode: Mode::Password,
            password: password::generate(&password_options),
            passphrase: passphrase::generate(&passphrase_options),
            password_options,
            passphrase_options,
        }
    }
}

impl PasswordTool {
    fn regenerate(&mut self) {
        match self.mode {
            Mode::Password => self.password = password::generate(&self.password_options),
            Mode::Passphrase => self.passphrase = passphrase::generate(&self.passphrase_options),
        }
    }

    fn password_options_ui(&mut self, ui: &mut egui::Ui) {
        let options = &mut self.password_options;
        ui.add(egui::Slider::new(&mut options.length, MIN_LENGTH..=MAX_LENGTH).text("Length"));
        ui.checkbox(&mut options.lowercase, "Lowercase (a–z)");
        ui.checkbox(&mut options.uppercase, "Uppercase (A–Z)");
        ui.checkbox(&mut options.digits, "Digits (0–9)");
        ui.checkbox(&mut options.symbols, "Symbols (!@#$…)");
        ui.checkbox(
            &mut options.exclude_ambiguous,
            "Exclude look-alikes (0 O 1 l I |)",
        );
    }

    fn passphrase_options_ui(&mut self, ui: &mut egui::Ui) {
        let options = &mut self.passphrase_options;
        ui.add(egui::Slider::new(&mut options.words, MIN_WORDS..=MAX_WORDS).text("Words"));
        ui.horizontal(|ui| {
            ui.label("Separator");
            egui::ComboBox::from_id_salt("passphrase_separator")
                .selected_text(options.separator.label())
                .show_ui(ui, |ui| {
                    for separator in Separator::ALL {
                        ui.selectable_value(&mut options.separator, separator, separator.label());
                    }
                });
        });
        ui.checkbox(&mut options.capitalize, "Capitalize words");
        ui.checkbox(&mut options.include_number, "Add a number");
    }
}

impl Tool for PasswordTool {
    fn name(&self) -> &'static str {
        "Passwords"
    }

    fn description(&self) -> &'static str {
        "Random passwords and memorable passphrases, generated locally with a cryptographically secure RNG."
    }

    fn category(&self) -> Category {
        Category::Generators
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Password, "Password");
            ui.selectable_value(&mut self.mode, Mode::Passphrase, "Passphrase");
        });
        ui.add_space(12.0);

        let output = match self.mode {
            Mode::Password => self
                .password
                .as_ref()
                .map(String::as_str)
                .map_err(ToString::to_string),
            Mode::Passphrase => Ok(self.passphrase.as_str()),
        };
        let output = match output {
            Ok(text) => {
                ui.label(RichText::new(text).monospace().size(20.0));
                text.to_owned()
            }
            Err(message) => {
                error_label(ui, message);
                String::new()
            }
        };
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            copy_button(ui, "copy_secret", "Copy", &output);
            if ui.button("Regenerate").clicked() {
                self.regenerate();
            }
        });
        ui.add_space(16.0);

        ui.spacing_mut().slider_width = 260.0;
        let (password_before, passphrase_before) = (
            self.password_options.clone(),
            self.passphrase_options.clone(),
        );
        match self.mode {
            Mode::Password => self.password_options_ui(ui),
            Mode::Passphrase => self.passphrase_options_ui(ui),
        }
        if self.password_options != password_before || self.passphrase_options != passphrase_before
        {
            self.regenerate();
        }
        ui.add_space(16.0);

        match self.mode {
            Mode::Password => {
                strength_bar(
                    ui,
                    self.password_options.strength(),
                    self.password_options.entropy_bits(),
                );
            }
            Mode::Passphrase => {
                strength_bar(
                    ui,
                    self.passphrase_options.strength(),
                    self.passphrase_options.entropy_bits(),
                );
                ui.add_space(4.0);
                ui.weak("Words from the EFF long word list (7,772 words).");
            }
        }
    }
}
