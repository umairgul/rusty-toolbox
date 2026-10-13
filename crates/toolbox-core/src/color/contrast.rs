//! WCAG 2 contrast ratios and accessible color suggestions.

use super::{Color, Oklch};

/// A WCAG 2 success criterion and the contrast it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// AA for body text (1.4.3): 4.5:1.
    AaNormal,
    /// AA for large text, 24px or 18.66px bold and up (1.4.3): 3:1.
    AaLarge,
    /// AAA for body text (1.4.6): 7:1.
    AaaNormal,
    /// AAA for large text (1.4.6): 4.5:1.
    AaaLarge,
    /// Icons, borders and other UI components (1.4.11): 3:1.
    UiComponents,
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::AaNormal,
        Level::AaLarge,
        Level::AaaNormal,
        Level::AaaLarge,
        Level::UiComponents,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Level::AaNormal => "AA · normal text",
            Level::AaLarge => "AA · large text",
            Level::AaaNormal => "AAA · normal text",
            Level::AaaLarge => "AAA · large text",
            Level::UiComponents => "AA · UI components and icons",
        }
    }

    pub fn required_ratio(self) -> f64 {
        match self {
            Level::AaNormal | Level::AaaLarge => 4.5,
            Level::AaLarge | Level::UiComponents => 3.0,
            Level::AaaNormal => 7.0,
        }
    }

    pub fn passes(self, ratio: f64) -> bool {
        ratio >= self.required_ratio()
    }
}

/// The WCAG 2 contrast ratio between text and background, from 1 to 21. A translucent
/// foreground is first composited over the background; the background is treated as opaque.
pub fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let background = background.with_alpha(1.0);
    let a = foreground.over(background).relative_luminance();
    let b = background.relative_luminance();
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The color closest to `foreground` (same hue and chroma, adjusted lightness) that reaches
/// `target` contrast against `background`, or `None` if neither darker nor lighter can.
pub fn suggest_for_contrast(foreground: Color, background: Color, target: f64) -> Option<Color> {
    if contrast_ratio(foreground, background) >= target {
        return Some(foreground);
    }
    let start = foreground.with_alpha(1.0).to_oklch();
    let at = |l: f64| Color::from_oklch_in_gamut(Oklch { l, ..start }, 1.0);
    let reaches = |l: f64| contrast_ratio(at(l), background) >= target;

    // Search toward black and toward white for the lightness nearest the start that passes.
    [0.0, 1.0]
        .into_iter()
        .filter(|&end| reaches(end))
        .map(|end| {
            let (mut failing, mut passing) = (start.l, end);
            for _ in 0..30 {
                let mid = (failing + passing) / 2.0;
                if reaches(mid) {
                    passing = mid
                } else {
                    failing = mid
                }
            }
            ((passing - start.l).abs(), at(passing))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, color)| color)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(v: u8) -> Color {
        Color::from_rgb8(v, v, v)
    }

    #[test]
    fn matches_known_ratios() {
        assert!((contrast_ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio(Color::WHITE, Color::WHITE) - 1.0).abs() < 1e-9);
        // #767676 is the lightest grey that passes AA on white; #777777 just fails.
        let passing = contrast_ratio(grey(0x76), Color::WHITE);
        let failing = contrast_ratio(grey(0x77), Color::WHITE);
        assert!(
            (passing - 4.54).abs() < 0.01 && Level::AaNormal.passes(passing),
            "{passing}"
        );
        assert!(
            (failing - 4.48).abs() < 0.01 && !Level::AaNormal.passes(failing),
            "{failing}"
        );
        // The order of the two colors doesn't matter.
        assert_eq!(contrast_ratio(Color::WHITE, grey(0x76)), passing);
    }

    #[test]
    fn composites_translucent_text() {
        let half_black = Color::BLACK.with_alpha(0.5);
        let ratio = contrast_ratio(half_black, Color::WHITE);
        assert_eq!(
            ratio,
            contrast_ratio(half_black.over(Color::WHITE), Color::WHITE)
        );
        // 50% black over white is grey 127.5, between greys 127 and 128.
        assert!(
            ratio > contrast_ratio(grey(128), Color::WHITE)
                && ratio < contrast_ratio(grey(127), Color::WHITE)
        );
    }

    #[test]
    fn suggests_nearest_passing_color() {
        let light_blue = Color::from_rgb8(0x93, 0xc5, 0xfd);
        let suggestion = suggest_for_contrast(light_blue, Color::WHITE, 4.5).unwrap();
        let ratio = contrast_ratio(suggestion, Color::WHITE);
        assert!(
            (4.5..4.7).contains(&ratio),
            "{ratio} for {}",
            suggestion.hex()
        );
        // It keeps the hue: still blue, just darker.
        let (hue, before) = (suggestion.to_oklch().h, light_blue.to_oklch());
        assert!((hue - before.h).abs() < 3.0, "{hue} vs {}", before.h);
        assert!(suggestion.to_oklch().l < before.l);

        // On a dark background, it lightens instead.
        let lighter = suggest_for_contrast(grey(0x55), grey(0x20), 7.0).unwrap();
        assert!(lighter.to_oklch().l > grey(0x55).to_oklch().l);
        assert!(contrast_ratio(lighter, grey(0x20)) >= 7.0);

        // A passing color is returned unchanged; an impossible target gives None.
        assert_eq!(
            suggest_for_contrast(Color::BLACK, Color::WHITE, 4.5),
            Some(Color::BLACK)
        );
        assert_eq!(suggest_for_contrast(grey(0x80), grey(0x80), 22.0), None);
    }
}
