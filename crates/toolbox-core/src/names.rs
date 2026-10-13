//! Name generator: project names, usernames, people and companies.

pub use crate::case::Case;
use crate::data::{ADJECTIVES, BRAND_ENDINGS, COMPANY_SUFFIXES, FIRST_NAMES, LAST_NAMES, NOUNS};
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Project,
    Username,
    Person,
    Company,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Project, Kind::Username, Kind::Person, Kind::Company];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Project => "Project",
            Kind::Username => "Username",
            Kind::Person => "Person",
            Kind::Company => "Company",
        }
    }
}

/// The case styles offered for project names.
pub const PROJECT_CASES: [Case; 6] = [
    Case::Kebab,
    Case::Snake,
    Case::Pascal,
    Case::Camel,
    Case::Title,
    Case::Flat,
];

/// `count` names using the thread-local RNG.
pub fn generate_list(kind: Kind, case: Case, count: usize) -> Vec<String> {
    let mut rng = rand::rng();
    (0..count).map(|_| generate(kind, case, &mut rng)).collect()
}

pub fn generate(kind: Kind, case: Case, rng: &mut impl Rng) -> String {
    match kind {
        Kind::Project => project(case, rng),
        Kind::Username => {
            let (first, last) = person(rng);
            username(first, last, rng)
        }
        Kind::Person => {
            let (first, last) = person(rng);
            format!("{first} {last}")
        }
        Kind::Company => company(rng),
    }
}

pub fn project(case: Case, rng: &mut impl Rng) -> String {
    case.join_words(&[pick(ADJECTIVES, rng), pick(NOUNS, rng)])
}

/// A random (first name, last name) pair.
pub fn person(rng: &mut impl Rng) -> (&'static str, &'static str) {
    (pick(FIRST_NAMES, rng), pick(LAST_NAMES, rng))
}

/// A username in one of several common styles. Only ASCII letters, digits, `.` and `_`.
pub fn username(first: &str, last: &str, rng: &mut impl Rng) -> String {
    let (first, last) = (slug(first), slug(last));
    match rng.random_range(0..4) {
        0 => format!("{first}.{last}"),
        1 => format!("{first}{}{}", &last[..1], rng.random_range(10..100)),
        2 => format!(
            "{}{}{}",
            capitalize(pick(ADJECTIVES, rng)),
            capitalize(pick(NOUNS, rng)),
            rng.random_range(10..100)
        ),
        _ => format!("{first}_{}", pick(NOUNS, rng)),
    }
}

pub fn company(rng: &mut impl Rng) -> String {
    match rng.random_range(0..4) {
        0 => {
            let first = pick(LAST_NAMES, rng);
            let second = loop {
                let name = pick(LAST_NAMES, rng);
                if name != first {
                    break name;
                }
            };
            format!("{first} & {second}")
        }
        1 => format!("{} {}", pick(LAST_NAMES, rng), pick(COMPANY_SUFFIXES, rng)),
        2 => format!(
            "{} {}",
            Case::Title.join_words(&[pick(ADJECTIVES, rng), pick(NOUNS, rng)]),
            pick(COMPANY_SUFFIXES, rng)
        ),
        _ => capitalize(&format!("{}{}", pick(NOUNS, rng), pick(BRAND_ENDINGS, rng))),
    }
}

/// Lowercase ASCII letters and digits only, e.g. "O'Brien" → "obrien".
pub fn slug(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn pick<'a>(list: &[&'a str], rng: &mut impl Rng) -> &'a str {
    list.choose(rng).expect("word lists are not empty")
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
    fn slug_keeps_ascii_alphanumerics() {
        assert_eq!(slug("O'Brien"), "obrien");
        assert_eq!(slug("Anne-Marie 2"), "annemarie2");
    }

    #[test]
    fn usernames_are_safe() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..500 {
            let (first, last) = person(&mut rng);
            let name = username(first, last, &mut rng);
            assert!(!name.is_empty());
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_'),
                "{name}"
            );
        }
    }

    #[test]
    fn every_kind_generates() {
        let mut rng = StdRng::seed_from_u64(3);
        for kind in Kind::ALL {
            for _ in 0..200 {
                let name = generate(kind, Case::Kebab, &mut rng);
                assert!(!name.trim().is_empty(), "{kind:?}");
            }
        }
        let project = generate(Kind::Project, Case::Kebab, &mut rng);
        assert_eq!(project.split('-').count(), 2, "{project}");
        let person = generate(Kind::Person, Case::Kebab, &mut rng);
        assert_eq!(person.split(' ').count(), 2, "{person}");
    }

    #[test]
    fn company_partners_differ() {
        let mut rng = StdRng::seed_from_u64(11);
        for _ in 0..500 {
            if let Some((a, b)) = company(&mut rng).split_once(" & ") {
                assert_ne!(a, b);
            }
        }
    }
}
