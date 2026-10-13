//! Passphrase generator ("correct-horse-battery-staple" style), using the EFF large word list.
//!
//! Word list: EFF's Long Wordlist by the Electronic Frontier Foundation, licensed under
//! CC BY 3.0 US (https://www.eff.org/dice). The four hyphenated words are removed, so a
//! hyphen separator stays unambiguous.

use crate::strength::Strength;
use rand::seq::IndexedRandom;
use rand::{CryptoRng, RngExt};
use std::sync::OnceLock;

pub const MIN_WORDS: usize = 3;
pub const MAX_WORDS: usize = 12;

const WORD_LIST: &str = include_str!("../data/eff_large_wordlist.txt");

fn words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| WORD_LIST.lines().filter(|line| !line.is_empty()).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Separator {
    Hyphen,
    Space,
    Period,
    Underscore,
    None,
}

impl Separator {
    pub const ALL: [Separator; 5] = [
        Separator::Hyphen,
        Separator::Space,
        Separator::Period,
        Separator::Underscore,
        Separator::None,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Separator::Hyphen => "-",
            Separator::Space => " ",
            Separator::Period => ".",
            Separator::Underscore => "_",
            Separator::None => "",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Separator::Hyphen => "Hyphen ( - )",
            Separator::Space => "Space",
            Separator::Period => "Period ( . )",
            Separator::Underscore => "Underscore ( _ )",
            Separator::None => "None",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub words: usize,
    pub separator: Separator,
    pub capitalize: bool,
    /// Appends one random digit to one random word, for sites that require a number.
    pub include_number: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            words: 6,
            separator: Separator::Hyphen,
            capitalize: false,
            include_number: false,
        }
    }
}

impl Options {
    /// Bits of entropy: `words * log2(list size)`, plus the digit when enabled. The digit's
    /// position is not counted, so this slightly underestimates.
    pub fn entropy_bits(&self) -> f64 {
        let word_bits = self.words as f64 * (words().len() as f64).log2();
        if self.include_number {
            word_bits + 10f64.log2()
        } else {
            word_bits
        }
    }

    pub fn strength(&self) -> Strength {
        Strength::from_bits(self.entropy_bits())
    }
}

/// Generates a passphrase using the thread-local cryptographically secure RNG.
pub fn generate(options: &Options) -> String {
    generate_with(options, &mut rand::rng())
}

pub fn generate_with(options: &Options, rng: &mut impl CryptoRng) -> String {
    let count = options.words.clamp(MIN_WORDS, MAX_WORDS);
    let mut chosen: Vec<String> = (0..count)
        .map(|_| {
            let word = *words().choose(rng).expect("word list is not empty");
            if options.capitalize {
                capitalize(word)
            } else {
                word.to_owned()
            }
        })
        .collect();
    if options.include_number {
        let index = rng.random_range(0..chosen.len());
        let digit = rng.random_range(0..10);
        chosen[index].push_str(&digit.to_string());
    }
    chosen.join(options.separator.as_str())
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn word_list_is_clean() {
        assert_eq!(words().len(), 7772);
        assert!(
            words()
                .iter()
                .all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase()))
        );
    }

    #[test]
    fn has_requested_word_count() {
        for count in [MIN_WORDS, 6, MAX_WORDS] {
            let phrase = generate_with(
                &Options {
                    words: count,
                    ..Default::default()
                },
                &mut StdRng::seed_from_u64(1),
            );
            assert_eq!(phrase.split('-').count(), count, "{phrase}");
            assert!(phrase.split('-').all(|w| words().contains(&w)), "{phrase}");
        }
    }

    #[test]
    fn applies_formatting_options() {
        let options = Options {
            words: 4,
            separator: Separator::Space,
            capitalize: true,
            include_number: true,
        };
        for _ in 0..50 {
            let phrase = generate(&options);
            let parts: Vec<&str> = phrase.split(' ').collect();
            assert_eq!(parts.len(), 4, "{phrase}");
            assert!(
                parts
                    .iter()
                    .all(|w| w.starts_with(|c: char| c.is_ascii_uppercase())),
                "{phrase}"
            );
            assert_eq!(
                phrase.chars().filter(char::is_ascii_digit).count(),
                1,
                "{phrase}"
            );
        }
        assert_eq!(
            generate(&Options {
                separator: Separator::None,
                ..Default::default()
            })
            .matches('-')
            .count(),
            0
        );
    }

    #[test]
    fn entropy() {
        // log2(7772) ≈ 12.92 bits per word.
        let six = Options::default().entropy_bits();
        assert!((77.0..78.0).contains(&six), "{six}");
        assert_eq!(Options::default().strength(), Strength::Strong);
        assert_eq!(
            Options {
                words: 8,
                ..Default::default()
            }
            .strength(),
            Strength::VeryStrong
        );
    }
}
