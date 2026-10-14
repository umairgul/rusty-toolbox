//! Random project names, usernames, people and companies.

use toolbox_core::names::{self, Case, Kind};

/// The most names generated at once. The core has no limit; this is the egui tool's.
const MAX_COUNT: u32 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NamesKind {
    Project,
    Username,
    Person,
    Company,
}

impl From<NamesKind> for Kind {
    fn from(kind: NamesKind) -> Self {
        match kind {
            NamesKind::Project => Kind::Project,
            NamesKind::Username => Kind::Username,
            NamesKind::Person => Kind::Person,
            NamesKind::Company => Kind::Company,
        }
    }
}

/// The case styles offered for project names (the core's `PROJECT_CASES`, in that order).
#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NamesCase {
    Kebab,
    Snake,
    Pascal,
    Camel,
    Title,
    Flat,
}

impl From<NamesCase> for Case {
    fn from(style: NamesCase) -> Self {
        match style {
            NamesCase::Kebab => Case::Kebab,
            NamesCase::Snake => Case::Snake,
            NamesCase::Pascal => Case::Pascal,
            NamesCase::Camel => Case::Camel,
            NamesCase::Title => Case::Title,
            NamesCase::Flat => Case::Flat,
        }
    }
}

/// e.g. "Username".
#[uniffi::export]
pub fn names_kind_label(kind: NamesKind) -> String {
    Kind::from(kind).label().into()
}

/// e.g. "kebab-case".
#[uniffi::export]
pub fn names_case_label(style: NamesCase) -> String {
    Case::from(style).label().into()
}

#[uniffi::export]
pub fn names_max_count() -> u32 {
    MAX_COUNT
}

/// `count` random names (clamped to 1–[`names_max_count`]). `style` applies to project names
/// only.
#[uniffi::export]
pub fn names_generate(kind: NamesKind, style: NamesCase, count: u32) -> Vec<String> {
    names::generate_list(
        kind.into(),
        style.into(),
        count.clamp(1, MAX_COUNT) as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolbox_core::names::PROJECT_CASES;

    #[test]
    fn cases_match_the_core_project_cases() {
        let cases = [
            NamesCase::Kebab,
            NamesCase::Snake,
            NamesCase::Pascal,
            NamesCase::Camel,
            NamesCase::Title,
            NamesCase::Flat,
        ];
        assert_eq!(cases.map(Case::from), PROJECT_CASES);
    }

    #[test]
    fn clamps_count() {
        assert_eq!(
            names_generate(NamesKind::Person, NamesCase::Kebab, 0).len(),
            1
        );
        assert_eq!(
            names_generate(NamesKind::Company, NamesCase::Kebab, 7).len(),
            7
        );
        assert_eq!(
            names_generate(NamesKind::Project, NamesCase::Snake, 500).len(),
            MAX_COUNT as usize
        );
    }
}
