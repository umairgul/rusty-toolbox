//! Numbers are up to 128 bits, which UniFFI can't carry, so they cross as digit strings.

use toolbox_core::number_base::{self, Error, Number, group};

const MIN_BASE: u32 = 2;
const MAX_BASE: u32 = 36;

/// The range of bases `to_base` supports, for the custom base.
#[derive(uniffi::Record)]
pub struct NumberBaseLimits {
    pub min_base: u32,
    pub max_base: u32,
}

#[uniffi::export]
pub fn number_base_limits() -> NumberBaseLimits {
    NumberBaseLimits {
        min_base: MIN_BASE,
        max_base: MAX_BASE,
    }
}

/// One way of writing the number. `shown` may be grouped for reading ("0xdead beef");
/// `copied` is the plain form ("0xdeadbeef").
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct NumberBaseRow {
    pub label: String,
    pub shown: String,
    pub copied: String,
}

impl NumberBaseRow {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: label.into(),
            shown: value.clone(),
            copied: value,
        }
    }

    fn with_copy(label: impl Into<String>, shown: String, copied: String) -> Self {
        Self {
            label: label.into(),
            shown,
            copied,
        }
    }
}

#[derive(Debug, PartialEq, Eq, uniffi::Enum)]
pub enum NumberBaseOutcome {
    /// Nothing to convert yet.
    Empty,
    Invalid {
        message: String,
    },
    Ok {
        rows: Vec<NumberBaseRow>,
    },
}

/// Parses `input` in `input_base` (`None` reads `0x`, `0o` and `0b` prefixes, otherwise
/// decimal) and writes it in decimal, hex, octal, binary and `custom_base` (clamped to
/// 2–36), plus its two's complement when negative and its size.
#[uniffi::export]
pub fn number_base_convert(
    input: &str,
    input_base: Option<u32>,
    custom_base: u32,
    uppercase: bool,
) -> NumberBaseOutcome {
    let input_base = input_base.map(|base| base.clamp(MIN_BASE, MAX_BASE));
    match number_base::parse(input, input_base) {
        Ok(number) => NumberBaseOutcome::Ok {
            rows: rows(number, custom_base.clamp(MIN_BASE, MAX_BASE), uppercase),
        },
        Err(Error::Empty) => NumberBaseOutcome::Empty,
        Err(error) => NumberBaseOutcome::Invalid {
            message: error.to_string(),
        },
    }
}

fn rows(number: Number, custom_base: u32, uppercase: bool) -> Vec<NumberBaseRow> {
    let case = |digits: String| {
        if uppercase {
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
    let custom = case(number.to_base(custom_base));

    let mut rows = vec![
        NumberBaseRow::with_copy("Decimal", group(&decimal, 3, ","), decimal.clone()),
        NumberBaseRow::with_copy(
            "Hex",
            prefixed("0x", &group(&hex, 4, " ")),
            prefixed("0x", &hex),
        ),
        NumberBaseRow::new("Octal", prefixed("0o", &octal)),
        NumberBaseRow::with_copy(
            "Binary",
            prefixed("0b", &group(&binary, 4, " ")),
            prefixed("0b", &binary),
        ),
        NumberBaseRow::new(format!("Base {custom_base}"), custom),
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
        rows.push(NumberBaseRow::with_copy(
            format!("Two's complement ({bits}-bit)"),
            format!("0x{}", group(&padded, 4, " ")),
            format!("0x{padded}"),
        ));
    }
    let bits = number.bit_length().max(1);
    let bytes = bits.div_ceil(8);
    rows.push(NumberBaseRow::new(
        "Size",
        format!(
            "{bits} bits · {bytes} byte{}",
            if bytes == 1 { "" } else { "s" }
        ),
    ));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(input: &str, base: Option<u32>, custom: u32, uppercase: bool) -> Vec<NumberBaseRow> {
        match number_base_convert(input, base, custom, uppercase) {
            NumberBaseOutcome::Ok { rows } => rows,
            other => panic!("{other:?}"),
        }
    }

    fn row<'a>(rows: &'a [NumberBaseRow], label: &str) -> &'a NumberBaseRow {
        rows.iter().find(|row| row.label == label).unwrap()
    }

    #[test]
    fn groups_shown_and_copies_plain() {
        let rows = convert("3735928559", None, 36, false);
        assert_eq!(row(&rows, "Decimal").shown, "3,735,928,559");
        assert_eq!(row(&rows, "Decimal").copied, "3735928559");
        assert_eq!(row(&rows, "Hex").shown, "0xdead beef");
        assert_eq!(row(&rows, "Hex").copied, "0xdeadbeef");
        assert_eq!(row(&rows, "Size").shown, "32 bits · 4 bytes");
        assert!(rows.iter().all(|row| !row.label.starts_with("Two's")));
    }

    #[test]
    fn negative_numbers_and_uppercase() {
        let rows = convert("-255", None, 16, true);
        assert_eq!(row(&rows, "Hex").copied, "-0xFF");
        assert_eq!(row(&rows, "Binary").shown, "-0b1111 1111");
        assert_eq!(row(&rows, "Base 16").shown, "-FF");
        let twos = row(&rows, "Two's complement (16-bit)");
        assert_eq!(
            (twos.shown.as_str(), twos.copied.as_str()),
            ("0xFF01", "0xFF01")
        );
        assert_eq!(row(&rows, "Size").shown, "8 bits · 1 byte");
    }

    #[test]
    fn clamps_bases_and_reports_errors() {
        let rows = convert("10", None, 99, false);
        assert_eq!(row(&rows, "Base 36").shown, "a");
        assert_eq!(
            number_base_convert("  ", None, 36, false),
            NumberBaseOutcome::Empty
        );
        assert!(matches!(
            number_base_convert("12", Some(2), 36, false),
            NumberBaseOutcome::Invalid { message } if message.contains("base 2")
        ));
    }

    #[test]
    fn handles_128_bit_numbers() {
        let max = u128::MAX.to_string();
        let rows = convert(&max, None, 36, false);
        assert_eq!(row(&rows, "Decimal").copied, max);
        assert_eq!(row(&rows, "Size").shown, "128 bits · 16 bytes");
    }
}
