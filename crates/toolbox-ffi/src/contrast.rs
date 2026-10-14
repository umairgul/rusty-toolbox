use crate::color::RgbaColor;
use toolbox_core::color::{self, Color, Level};

/// The levels that get a suggested passing color when they fail, with the ratio each needs.
const SUGGESTED: [(Level, f64); 2] = [(Level::AaNormal, 4.5), (Level::AaaNormal, 7.0)];

/// One WCAG 2 success criterion and whether the colors meet it.
#[derive(uniffi::Record)]
pub struct ContrastLevelResult {
    /// e.g. "AA · normal text".
    pub label: String,
    /// The ratio it needs, e.g. 4.5.
    pub required_ratio: f64,
    pub passes: bool,
}

/// A color with the same hue as the text that passes a level it fails.
#[derive(uniffi::Record)]
pub struct ContrastSuggestion {
    /// The label of the level it passes, e.g. "AA · normal text".
    pub level: String,
    /// The ratio it aims for, e.g. 4.5.
    pub target: f64,
    /// `None` when no shade of the text's hue reaches the target on this background.
    pub color: Option<RgbaColor>,
    /// The suggested color's contrast against the background, or 0 without one.
    pub ratio: f64,
}

#[derive(uniffi::Record)]
pub struct ContrastReport {
    /// From 1 to 21.
    pub ratio: f64,
    /// A one-line summary, e.g. "Good: passes AA for all text".
    pub verdict: String,
    pub levels: Vec<ContrastLevelResult>,
    /// For AA and AAA normal text, when they fail.
    pub suggestions: Vec<ContrastSuggestion>,
}

/// Checks text against a background. Translucent text is composited over the background,
/// which is treated as opaque.
#[uniffi::export]
pub fn contrast_check(foreground: RgbaColor, background: RgbaColor) -> ContrastReport {
    let (text, background) = (Color::from(foreground), Color::from(background));
    let ratio = color::contrast_ratio(text, background);
    let levels = Level::ALL
        .iter()
        .map(|&level| ContrastLevelResult {
            label: level.label().into(),
            required_ratio: level.required_ratio(),
            passes: level.passes(ratio),
        })
        .collect();
    let suggestions = SUGGESTED
        .iter()
        .filter(|(level, _)| !level.passes(ratio))
        .map(|&(level, target)| {
            let suggestion = color::suggest_for_contrast(text, background, target);
            ContrastSuggestion {
                level: level.label().into(),
                target,
                color: suggestion.map(Into::into),
                ratio: suggestion.map_or(0.0, |c| color::contrast_ratio(c, background)),
            }
        })
        .collect();
    ContrastReport {
        ratio,
        verdict: verdict(ratio).into(),
        levels,
        suggestions,
    }
}

fn verdict(ratio: f64) -> &'static str {
    if Level::AaaNormal.passes(ratio) {
        "Excellent: passes every level"
    } else if Level::AaNormal.passes(ratio) {
        "Good: passes AA for all text"
    } else if Level::AaLarge.passes(ratio) {
        "Only for large text and UI components"
    } else {
        "Fails: too little contrast for text"
    }
}

#[derive(uniffi::Record)]
pub struct ContrastColors {
    pub foreground: RgbaColor,
    pub background: RgbaColor,
}

/// Swaps text and background. The background must be opaque, so translucent text becomes
/// the opaque color it shows as over the old background.
#[uniffi::export]
pub fn contrast_swap(foreground: RgbaColor, background: RgbaColor) -> ContrastColors {
    let (text, background) = (
        Color::from(foreground),
        Color::from(background).with_alpha(1.0),
    );
    ContrastColors {
        foreground: background.into(),
        background: text.over(background).with_alpha(1.0).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> RgbaColor {
        Color::from_rgb8(r, g, b).into()
    }

    #[test]
    fn reports_levels_and_suggestions() {
        // The default: light blue text on white fails everything but nothing is impossible.
        let report = contrast_check(rgb(0x93, 0xc5, 0xfd), rgb(255, 255, 255));
        assert!(report.ratio < 3.0);
        assert_eq!(report.verdict, "Fails: too little contrast for text");
        assert_eq!(report.levels.len(), 5);
        assert!(report.levels.iter().all(|level| !level.passes));
        assert_eq!(report.suggestions.len(), 2);
        for suggestion in &report.suggestions {
            assert!(suggestion.color.is_some());
            assert!(suggestion.ratio >= suggestion.target);
        }

        let black_on_white = contrast_check(rgb(0, 0, 0), rgb(255, 255, 255));
        assert!((black_on_white.ratio - 21.0).abs() < 1e-9);
        assert_eq!(black_on_white.verdict, "Excellent: passes every level");
        assert!(black_on_white.suggestions.is_empty());

        // Only AAA fails: one suggestion.
        let grey = contrast_check(rgb(0x76, 0x76, 0x76), rgb(255, 255, 255));
        assert_eq!(grey.verdict, "Good: passes AA for all text");
        assert_eq!(grey.suggestions.len(), 1);
        assert_eq!(grey.suggestions[0].target, 7.0);

        // Mid grey on mid grey can't reach 7:1 with any shade… but black or white can reach
        // 4.5:1, so check the impossible case directly.
        let same = contrast_check(rgb(0x80, 0x80, 0x80), rgb(0x80, 0x80, 0x80));
        assert_eq!(same.ratio, 1.0);
        assert!(same.suggestions.iter().any(|s| s.color.is_none()));
    }

    #[test]
    fn swap_keeps_background_opaque() {
        let half_black = RgbaColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        };
        let swapped = contrast_swap(half_black, rgb(255, 255, 255));
        assert_eq!(Color::from(swapped.foreground).hex(), "#ffffff");
        assert_eq!(swapped.background.a, 1.0);
        assert_eq!(Color::from(swapped.background).hex(), "#808080");
    }
}
