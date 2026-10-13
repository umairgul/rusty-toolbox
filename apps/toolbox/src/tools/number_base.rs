use super::{Category, Tool, ValueRow, error_label, value_rows};
use toolbox_core::number_base::{self, Number, group};

/// Input bases on offer. `None` reads 0x/0o/0b prefixes and treats the rest as decimal.
const INPUT_BASES: [(Option<u32>, &str); 5] = [
    (None, "Auto (0x, 0o, 0b or decimal)"),
    (Some(10), "Decimal"),
    (Some(16), "Hex"),
    (Some(8), "Octal"),
    (Some(2), "Binary"),
];

pub struct NumberBaseTool {
    input: String,
    input_base: Option<u32>,
    custom_base: u32,
    uppercase: bool,
}

impl Default for NumberBaseTool {
    fn default() -> Self {
        Self {
            input: "255".into(),
            input_base: None,
            custom_base: 36,
            uppercase: false,
        }
    }
}

impl NumberBaseTool {
    fn rows(&self, number: Number) -> Vec<ValueRow> {
        let case = |digits: String| {
            if self.uppercase {
                digits.to_uppercase()
            } else {
                digits
            }
        };
        // Prefixes go after the sign: -0xff.
        let prefixed = |prefix: &str, digits: &str| match digits.strip_prefix('-') {
            Some(rest) => format!("-{prefix}{rest}"),
            None => format!("{prefix}{digits}"),
        };
        let decimal = number.to_base(10);
        let hex = case(number.to_base(16));
        let octal = number.to_base(8);
        let binary = number.to_base(2);
        let custom = case(number.to_base(self.custom_base));

        let mut rows = vec![
            ValueRow::with_copy("Decimal", group(&decimal, 3, ","), decimal.clone()),
            ValueRow::with_copy(
                "Hex",
                prefixed("0x", &group(&hex, 4, " ")),
                prefixed("0x", &hex),
            ),
            ValueRow::with_copy("Octal", prefixed("0o", &octal), prefixed("0o", &octal)),
            ValueRow::with_copy(
                "Binary",
                prefixed("0b", &group(&binary, 4, " ")),
                prefixed("0b", &binary),
            ),
            ValueRow::new(format!("Base {}", self.custom_base), custom),
        ];
        if let Some((bits, pattern)) = number.twos_complement() {
            let pattern_hex = case(
                Number {
                    negative: false,
                    magnitude: pattern,
                }
                .to_base(16),
            );
            let width = (bits / 4) as usize;
            let padded = format!("{pattern_hex:0>width$}");
            rows.push(ValueRow::with_copy(
                format!("Two's complement ({bits}-bit)"),
                format!("0x{}", group(&padded, 4, " ")),
                format!("0x{padded}"),
            ));
        }
        let bits = number.bit_length().max(1);
        rows.push(ValueRow::new("Size", {
            let bytes = bits.div_ceil(8);
            format!(
                "{bits} bits · {bytes} byte{}",
                if bytes == 1 { "" } else { "s" }
            )
        }));
        rows
    }
}

impl Tool for NumberBaseTool {
    fn name(&self) -> &'static str {
        "Number Bases"
    }

    fn description(&self) -> &'static str {
        "Convert integers between decimal, hex, octal, binary and any base up to 36."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(320.0)
                    .hint_text("e.g. 255, 0xff, -42"),
            );
            let label = INPUT_BASES
                .iter()
                .find(|(base, _)| *base == self.input_base)
                .map_or("Auto", |(_, label)| label);
            egui::ComboBox::from_id_salt("number_input_base")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for (base, label) in INPUT_BASES {
                        ui.selectable_value(&mut self.input_base, base, label);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Custom base");
            ui.add(egui::DragValue::new(&mut self.custom_base).range(2..=36));
            ui.add_space(12.0);
            ui.checkbox(&mut self.uppercase, "Uppercase letters");
        });
        ui.add_space(8.0);

        match number_base::parse(&self.input, self.input_base) {
            Ok(number) => value_rows(ui, "number_rows", &self.rows(number)),
            Err(number_base::Error::Empty) => {}
            Err(error) => error_label(ui, error),
        }
    }
}
