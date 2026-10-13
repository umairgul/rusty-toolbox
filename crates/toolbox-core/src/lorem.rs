//! Lorem ipsum placeholder text: words, sentences or paragraphs.

use crate::data::LOREM_WORDS;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

pub const MAX_COUNT: usize = 1000;
pub const CLASSIC_OPENING: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Paragraphs,
    Sentences,
    Words,
}

impl Unit {
    pub const ALL: [Unit; 3] = [Unit::Paragraphs, Unit::Sentences, Unit::Words];

    pub fn label(self) -> &'static str {
        match self {
            Unit::Paragraphs => "Paragraphs",
            Unit::Sentences => "Sentences",
            Unit::Words => "Words",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub unit: Unit,
    pub count: usize,
    /// Begin with "Lorem ipsum dolor sit amet, consectetur adipiscing elit".
    pub classic_start: bool,
    /// Wrap each paragraph in `<p>` tags.
    pub html: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            unit: Unit::Paragraphs,
            count: 3,
            classic_start: true,
            html: false,
        }
    }
}

pub fn generate(options: &Options) -> String {
    generate_with(options, &mut rand::rng())
}

pub fn generate_with(options: &Options, rng: &mut impl Rng) -> String {
    let count = options.count.clamp(1, MAX_COUNT);
    let paragraphs: Vec<String> = match options.unit {
        Unit::Words => {
            let mut words: Vec<String> = (0..count).map(|_| word(rng).to_owned()).collect();
            if options.classic_start {
                let opening = CLASSIC_OPENING.split(' ').take(count);
                for (slot, classic) in words.iter_mut().zip(opening) {
                    *slot = classic.to_owned();
                }
            }
            vec![words.join(" ")]
        }
        Unit::Sentences => vec![sentences(count, options.classic_start, rng)],
        Unit::Paragraphs => (0..count)
            .map(|i| {
                let sentence_count = rng.random_range(4..=7);
                sentences(sentence_count, options.classic_start && i == 0, rng)
            })
            .collect(),
    };

    if options.html {
        paragraphs
            .iter()
            .map(|p| format!("<p>{p}</p>"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        paragraphs.join("\n\n")
    }
}

fn sentences(count: usize, classic_start: bool, rng: &mut impl Rng) -> String {
    (0..count)
        .map(|i| {
            if i == 0 && classic_start {
                format!("{CLASSIC_OPENING}.")
            } else {
                sentence(rng)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A sentence of 6–16 words, capitalized, with an occasional comma, ending in a period.
fn sentence(rng: &mut impl Rng) -> String {
    let length = rng.random_range(6..=16);
    let mut text = String::new();
    for i in 0..length {
        let word = word(rng);
        if i == 0 {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                text.extend(first.to_uppercase());
                text.push_str(chars.as_str());
            }
        } else {
            text.push(' ');
            text.push_str(word);
        }
        if i > 1 && i < length - 2 && rng.random_ratio(1, 8) {
            text.push(',');
        }
    }
    text.push('.');
    text
}

fn word(rng: &mut impl Rng) -> &'static str {
    LOREM_WORDS.choose(rng).expect("word list is not empty")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn generate(options: Options) -> String {
        generate_with(&options, &mut StdRng::seed_from_u64(9))
    }

    #[test]
    fn word_count() {
        let text = generate(Options {
            unit: Unit::Words,
            count: 25,
            classic_start: false,
            html: false,
        });
        assert_eq!(text.split(' ').count(), 25);
        let short = generate(Options {
            unit: Unit::Words,
            count: 2,
            classic_start: true,
            html: false,
        });
        assert_eq!(short, "Lorem ipsum");
    }

    #[test]
    fn sentence_count() {
        let text = generate(Options {
            unit: Unit::Sentences,
            count: 5,
            classic_start: true,
            html: false,
        });
        assert!(text.starts_with(CLASSIC_OPENING));
        assert_eq!(text.matches('.').count(), 5, "{text}");
    }

    #[test]
    fn paragraphs() {
        let text = generate(Options {
            unit: Unit::Paragraphs,
            count: 4,
            classic_start: false,
            html: false,
        });
        assert_eq!(text.split("\n\n").count(), 4);
        assert!(!text.starts_with("Lorem ipsum dolor"));

        let html = generate(Options {
            unit: Unit::Paragraphs,
            count: 3,
            classic_start: true,
            html: true,
        });
        assert_eq!(html.lines().count(), 3);
        assert!(
            html.lines()
                .all(|line| line.starts_with("<p>") && line.ends_with("</p>"))
        );
    }
}
