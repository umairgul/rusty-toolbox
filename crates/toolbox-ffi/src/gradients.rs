use crate::color::RgbaColor;
use toolbox_core::color::{self, Color, Space};

/// Sampled stops in the CSS that works in every browser.
const FALLBACK_SAMPLES: usize = 9;

/// The color space to blend in.
#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum GradientSpace {
    Oklab,
    Oklch,
    Srgb,
    LinearSrgb,
}

impl From<GradientSpace> for Space {
    fn from(space: GradientSpace) -> Self {
        match space {
            GradientSpace::Oklab => Space::Oklab,
            GradientSpace::Oklch => Space::Oklch,
            GradientSpace::Srgb => Space::Srgb,
            GradientSpace::LinearSrgb => Space::LinearSrgb,
        }
    }
}

/// e.g. "OKLab (smooth, recommended)".
#[uniffi::export]
pub fn gradients_space_label(space: GradientSpace) -> String {
    Space::from(space).label().into()
}

fn colors(stops: Vec<RgbaColor>) -> Vec<Color> {
    stops.into_iter().map(Into::into).collect()
}

/// `count` evenly spaced colors from the first stop to the last, blended in `space`. Also
/// used to draw the preview, so it matches the chosen space.
#[uniffi::export]
pub fn gradients_steps(stops: Vec<RgbaColor>, count: u32, space: GradientSpace) -> Vec<RgbaColor> {
    color::steps(&colors(stops), count as usize, space.into())
        .into_iter()
        .map(Into::into)
        .collect()
}

#[derive(uniffi::Record)]
pub struct GradientCss {
    /// `linear-gradient(in oklab 90deg, …)`, blended by the browser (Chrome 111+, Safari
    /// 16.2+, Firefox 127+).
    pub modern: String,
    /// The gradient pre-blended into sRGB stops with positions, for every browser.
    pub fallback: String,
}

/// CSS `linear-gradient()`s for the stops. `angle` is in degrees, as in CSS: 0 points up,
/// 90 to the right.
#[uniffi::export]
pub fn gradients_css(stops: Vec<RgbaColor>, angle: f64, space: GradientSpace) -> GradientCss {
    let (stops, space) = (colors(stops), space.into());
    GradientCss {
        modern: color::css_gradient(&stops, angle, space),
        fallback: color::css_gradient_fallback(&stops, angle, space, FALLBACK_SAMPLES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: RgbaColor = RgbaColor {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    const BLUE: RgbaColor = RgbaColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    #[test]
    fn steps_and_css() {
        let steps = gradients_steps(vec![RED, BLUE], 5, GradientSpace::Oklab);
        assert_eq!(steps.len(), 5);
        assert_eq!(Color::from(steps[0]).hex(), "#ff0000");
        assert_eq!(Color::from(steps[4]).hex(), "#0000ff");
        let css = gradients_css(vec![RED, BLUE], 90.0, GradientSpace::Oklch);
        assert_eq!(
            css.modern,
            "linear-gradient(in oklch 90deg, #ff0000, #0000ff)"
        );
        assert_eq!(css.fallback.matches('%').count(), FALLBACK_SAMPLES);
        assert_eq!(
            gradients_space_label(GradientSpace::Srgb),
            "sRGB (classic CSS)"
        );
    }
}
