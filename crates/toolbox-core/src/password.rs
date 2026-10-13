//! Password generator: random passwords from chosen character sets, using a cryptographically
//! secure RNG, plus an entropy-based strength estimate.

use crate::strength::Strength;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{CryptoRng, RngExt};
use std::fmt;

pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 128;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.<>?/~";
/// Characters that are easy to confuse when read or typed by hand.
const AMBIGUOUS: &str = "0O1lI|";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NoCharacterSets,
    InvalidLength,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoCharacterSets => write!(f, "Select at least one character set"),
            Error::InvalidLength => {
                write!(f, "Length must be between {MIN_LENGTH} and {MAX_LENGTH}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Options {
    /// The enabled character sets, with ambiguous characters removed if requested.
    fn sets(&self) -> Vec<Vec<char>> {
        [
            (self.lowercase, LOWERCASE),
            (self.uppercase, UPPERCASE),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ]
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, chars)| {
            chars
                .chars()
                .filter(|c| !(self.exclude_ambiguous && AMBIGUOUS.contains(*c)))
                .collect()
        })
        .collect()
    }

    /// Bits of entropy for a password made with these options: `length * log2(pool size)`.
    pub fn entropy_bits(&self) -> f64 {
        let pool: usize = self.sets().iter().map(Vec::len).sum();
        if pool == 0 {
            0.0
        } else {
            self.length as f64 * (pool as f64).log2()
        }
    }

    pub fn strength(&self) -> Strength {
        Strength::from_bits(self.entropy_bits())
    }
}

/// Generates a password using the thread-local cryptographically secure RNG.
pub fn generate(options: &Options) -> Result<String, Error> {
    generate_with(options, &mut rand::rng())
}

/// Generates a password containing at least one character from every enabled set.
pub fn generate_with(options: &Options, rng: &mut impl CryptoRng) -> Result<String, Error> {
    let sets = options.sets();
    if sets.is_empty() {
        return Err(Error::NoCharacterSets);
    }
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&options.length) {
        return Err(Error::InvalidLength);
    }

    let pool: Vec<char> = sets.concat();
    // One guaranteed character per set, the rest from the whole pool, then shuffled so the
    // guaranteed characters aren't always at the start.
    let mut chars: Vec<char> = sets
        .iter()
        .filter_map(|set| set.choose(rng).copied())
        .collect();
    while chars.len() < options.length {
        chars.push(pool[rng.random_range(0..pool.len())]);
    }
    chars.shuffle(rng);
    Ok(chars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn has_requested_length() {
        for length in [MIN_LENGTH, 20, MAX_LENGTH] {
            let password = generate_with(
                &Options {
                    length,
                    ..Default::default()
                },
                &mut rng(),
            )
            .unwrap();
            assert_eq!(password.chars().count(), length);
        }
    }

    #[test]
    fn contains_every_enabled_set() {
        let options = Options {
            length: MIN_LENGTH,
            ..Default::default()
        };
        for _ in 0..200 {
            let password = generate(&options).unwrap();
            assert!(
                password.chars().any(|c| c.is_ascii_lowercase()),
                "{password}"
            );
            assert!(
                password.chars().any(|c| c.is_ascii_uppercase()),
                "{password}"
            );
            assert!(password.chars().any(|c| c.is_ascii_digit()), "{password}");
            assert!(password.chars().any(|c| SYMBOLS.contains(c)), "{password}");
        }
    }

    #[test]
    fn uses_only_enabled_sets() {
        let options = Options {
            digits: true,
            lowercase: false,
            uppercase: false,
            symbols: false,
            ..Default::default()
        };
        let password = generate_with(&options, &mut rng()).unwrap();
        assert!(password.chars().all(|c| c.is_ascii_digit()), "{password}");
    }

    #[test]
    fn excludes_ambiguous_characters() {
        let options = Options {
            length: MAX_LENGTH,
            exclude_ambiguous: true,
            ..Default::default()
        };
        for _ in 0..50 {
            let password = generate(&options).unwrap();
            assert!(
                !password.chars().any(|c| AMBIGUOUS.contains(c)),
                "{password}"
            );
        }
    }

    #[test]
    fn rejects_invalid_options() {
        let none = Options {
            lowercase: false,
            uppercase: false,
            digits: false,
            symbols: false,
            ..Default::default()
        };
        assert_eq!(generate(&none), Err(Error::NoCharacterSets));
        assert_eq!(
            generate(&Options {
                length: MIN_LENGTH - 1,
                ..Default::default()
            }),
            Err(Error::InvalidLength)
        );
        assert_eq!(
            generate(&Options {
                length: MAX_LENGTH + 1,
                ..Default::default()
            }),
            Err(Error::InvalidLength)
        );
    }

    #[test]
    fn strength_grows_with_length_and_sets() {
        let digits_only = Options {
            length: 6,
            lowercase: false,
            uppercase: false,
            symbols: false,
            ..Default::default()
        };
        assert_eq!(digits_only.strength(), Strength::Weak);
        assert_eq!(Options::default().strength(), Strength::VeryStrong);
        assert!(
            Options {
                length: 10,
                ..Default::default()
            }
            .strength()
                < Options::default().strength()
        );
    }
}
