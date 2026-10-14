//! Lorem ipsum placeholder text.

use toolbox_core::lorem::{self, Unit};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum LoremUnit {
    Paragraphs,
    Sentences,
    Words,
}

impl From<LoremUnit> for Unit {
    fn from(unit: LoremUnit) -> Self {
        match unit {
            LoremUnit::Paragraphs => Unit::Paragraphs,
            LoremUnit::Sentences => Unit::Sentences,
            LoremUnit::Words => Unit::Words,
        }
    }
}

impl From<Unit> for LoremUnit {
    fn from(unit: Unit) -> Self {
        match unit {
            Unit::Paragraphs => LoremUnit::Paragraphs,
            Unit::Sentences => LoremUnit::Sentences,
            Unit::Words => LoremUnit::Words,
        }
    }
}

/// e.g. "Paragraphs".
#[uniffi::export]
pub fn lorem_unit_label(unit: LoremUnit) -> String {
    Unit::from(unit).label().into()
}

/// The most of each unit offered at once (the egui tool's limits, within the core's
/// `MAX_COUNT`).
#[uniffi::export]
pub fn lorem_max_count(unit: LoremUnit) -> u32 {
    let max = match unit {
        LoremUnit::Paragraphs => 50,
        LoremUnit::Sentences => 100,
        LoremUnit::Words => 1000,
    };
    max.min(lorem::MAX_COUNT as u32)
}

#[derive(uniffi::Record)]
pub struct LoremOptions {
    pub unit: LoremUnit,
    pub count: u32,
    /// Begin with "Lorem ipsum dolor sit amet, consectetur adipiscing elit".
    pub classic_start: bool,
    /// Wrap each paragraph in `<p>` tags.
    pub html: bool,
}

impl From<lorem::Options> for LoremOptions {
    fn from(options: lorem::Options) -> Self {
        LoremOptions {
            unit: options.unit.into(),
            count: options.count as u32,
            classic_start: options.classic_start,
            html: options.html,
        }
    }
}

impl From<LoremOptions> for lorem::Options {
    fn from(options: LoremOptions) -> Self {
        lorem::Options {
            unit: options.unit.into(),
            count: options.count as usize,
            classic_start: options.classic_start,
            html: options.html,
        }
    }
}

#[uniffi::export]
pub fn lorem_default_options() -> LoremOptions {
    lorem::Options::default().into()
}

#[uniffi::export]
pub fn lorem_generate(options: LoremOptions) -> String {
    lorem::generate(&options.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_round_trip() {
        let options = lorem::Options {
            unit: Unit::Sentences,
            count: 17,
            classic_start: false,
            html: true,
        };
        assert_eq!(
            lorem::Options::from(LoremOptions::from(options.clone())),
            options
        );
        for unit in Unit::ALL {
            assert_eq!(Unit::from(LoremUnit::from(unit)), unit);
        }
    }

    #[test]
    fn limits_stay_within_the_core() {
        for unit in Unit::ALL {
            assert!(lorem_max_count(unit.into()) as usize <= lorem::MAX_COUNT);
        }
    }
}
