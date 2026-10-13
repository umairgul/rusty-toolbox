//! Number base conversion between binary, octal, decimal, hex and any base from 2 to 36.

use std::fmt;

/// An integer as a sign and a 128-bit magnitude, so it covers every `u128` and every `i128`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Number {
    pub negative: bool,
    pub magnitude: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    InvalidDigit { digit: char, base: u32 },
    TooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "Enter a number"),
            Error::InvalidDigit { digit, base } => {
                write!(f, "'{digit}' isn't a valid digit in base {base}")
            }
            Error::TooLarge => write!(f, "Too large: the limit is 128 bits"),
        }
    }
}

impl std::error::Error for Error {}

/// Parses `input` in `base`. With `None`, a `0x`, `0o` or `0b` prefix picks the base and
/// anything else is decimal. Spaces, `_` and `,` are ignored and a leading `-` is allowed.
pub fn parse(input: &str, base: Option<u32>) -> Result<Number, Error> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != ',')
        .collect();
    let (negative, digits) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let lower = digits.to_ascii_lowercase();
    let (base, digits) = match base {
        Some(base) => (base, strip_prefix_for(base, &lower)),
        None => match lower.get(..2) {
            Some("0x") => (16, &lower[2..]),
            Some("0o") => (8, &lower[2..]),
            Some("0b") => (2, &lower[2..]),
            _ => (10, lower.as_str()),
        },
    };
    if digits.is_empty() {
        return Err(Error::Empty);
    }
    let mut magnitude: u128 = 0;
    for c in digits.chars() {
        let digit = c
            .to_digit(base)
            .ok_or(Error::InvalidDigit { digit: c, base })?;
        magnitude = magnitude
            .checked_mul(u128::from(base))
            .and_then(|m| m.checked_add(u128::from(digit)))
            .ok_or(Error::TooLarge)?;
    }
    Ok(Number {
        negative: negative && magnitude != 0,
        magnitude,
    })
}

/// Lets "0xff" be pasted while hex is selected.
fn strip_prefix_for(base: u32, digits: &str) -> &str {
    let prefix = match base {
        16 => "0x",
        8 => "0o",
        2 => "0b",
        _ => return digits,
    };
    digits.strip_prefix(prefix).unwrap_or(digits)
}

impl Number {
    /// Digits in `base` (2–36, lowercase letters), with a leading `-` if negative.
    pub fn to_base(self, base: u32) -> String {
        assert!((2..=36).contains(&base), "base must be between 2 and 36");
        let mut digits = Vec::new();
        let mut rest = self.magnitude;
        loop {
            digits.push(
                std::char::from_digit((rest % u128::from(base)) as u32, base)
                    .expect("digit is below base"),
            );
            rest /= u128::from(base);
            if rest == 0 {
                break;
            }
        }
        if self.negative {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }

    /// Bits needed to write the magnitude in binary.
    pub fn bit_length(self) -> u32 {
        128 - self.magnitude.leading_zeros()
    }

    /// The two's complement bit pattern of a negative number in the smallest of 8, 16, 32,
    /// 64 or 128 bits that holds it, as (bits, pattern).
    pub fn twos_complement(self) -> Option<(u32, u128)> {
        if !self.negative {
            return None;
        }
        [8u32, 16, 32, 64, 128]
            .into_iter()
            .find(|&bits| self.magnitude <= 1u128 << (bits - 1))
            .map(|bits| {
                let pattern = self.magnitude.wrapping_neg();
                let mask = if bits == 128 {
                    u128::MAX
                } else {
                    (1u128 << bits) - 1
                };
                (bits, pattern & mask)
            })
    }
}

/// Inserts `separator` every `size` characters from the right: `group("11111111", 4, " ")` is
/// `"1111 1111"`. A leading `-` stays in front.
pub fn group(digits: &str, size: usize, separator: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |rest| ("-", rest));
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::from(sign);
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(size) {
            out.push_str(separator);
        }
        out.push(*c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(input: &str) -> Number {
        parse(input, None).unwrap()
    }

    #[test]
    fn parses_prefixes_and_separators() {
        assert_eq!(number("0xFF").magnitude, 255);
        assert_eq!(number("0b1010").magnitude, 10);
        assert_eq!(number("0o777").magnitude, 511);
        assert_eq!(number("1_000,000").magnitude, 1_000_000);
        assert_eq!(
            number("-42"),
            Number {
                negative: true,
                magnitude: 42
            }
        );
        assert_eq!(
            number("-0"),
            Number {
                negative: false,
                magnitude: 0
            }
        );
        assert_eq!(parse("0xff", Some(16)).unwrap().magnitude, 255);
        assert_eq!(parse("zz", Some(36)).unwrap().magnitude, 35 * 36 + 35);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(
            parse("12", Some(2)),
            Err(Error::InvalidDigit {
                digit: '2',
                base: 2
            })
        );
        assert_eq!(parse("0x", None), Err(Error::Empty));
        assert_eq!(parse("", None), Err(Error::Empty));
        let max = u128::MAX.to_string();
        assert_eq!(number(&max).magnitude, u128::MAX);
        assert_eq!(parse(&format!("{max}0"), None), Err(Error::TooLarge));
    }

    #[test]
    fn converts_to_bases() {
        let n = number("255");
        assert_eq!(n.to_base(2), "11111111");
        assert_eq!(n.to_base(8), "377");
        assert_eq!(n.to_base(16), "ff");
        assert_eq!(n.to_base(36), "73");
        assert_eq!(number("0").to_base(2), "0");
        assert_eq!(number("-10").to_base(16), "-a");
        assert_eq!(n.bit_length(), 8);
    }

    #[test]
    fn twos_complement_uses_smallest_width() {
        assert_eq!(number("-1").twos_complement(), Some((8, 0xff)));
        assert_eq!(number("-128").twos_complement(), Some((8, 0x80)));
        assert_eq!(number("-129").twos_complement(), Some((16, 0xff7f)));
        assert_eq!(
            number("-2147483648").twos_complement(),
            Some((32, 0x8000_0000))
        );
        assert_eq!(number("5").twos_complement(), None);
        assert_eq!(
            number(&format!("-{}", 1u128 << 127)).twos_complement(),
            Some((128, 1u128 << 127))
        );
        assert_eq!(
            number(&format!("-{}", (1u128 << 127) + 1)).twos_complement(),
            None
        );
    }

    #[test]
    fn groups_digits() {
        assert_eq!(group("11111111", 4, " "), "1111 1111");
        assert_eq!(group("1010101", 4, " "), "101 0101");
        assert_eq!(group("-1234567", 3, ","), "-1,234,567");
        assert_eq!(group("12", 3, ","), "12");
    }
}
