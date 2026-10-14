//! Gradients: interpolating between color stops in different color spaces, and CSS output.

use super::{Color, Oklab, Oklch};

/// The color space to blend in. OKLab avoids the muddy grey middle that sRGB blending gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Oklab,
    Oklch,
    Srgb,
    LinearSrgb,
}

impl Space {
    pub const ALL: [Space; 4] = [Space::Oklab, Space::Oklch, Space::Srgb, Space::LinearSrgb];

    pub fn label(self) -> &'static str {
        match self {
            Space::Oklab => "OKLab (smooth, recommended)",
            Space::Oklch => "OKLCH (vivid, through the hue wheel)",
            Space::Srgb => "sRGB (classic CSS)",
            Space::LinearSrgb => "Linear sRGB (light-like)",
        }
    }

    /// The `in …` keyword for CSS `linear-gradient()`.
    fn css(self) -> &'static str {
        match self {
            Space::Oklab => "oklab",
            Space::Oklch => "oklch",
            Space::Srgb => "srgb",
            Space::LinearSrgb => "srgb-linear",
        }
    }
}

/// The color `t` (0–1) of the way from `a` to `b`, blended in `space`.
pub fn mix(a: Color, b: Color, t: f64, space: Space) -> Color {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    let alpha = lerp(a.a, b.a);
    match space {
        Space::Srgb => Color {
            r: lerp(a.r, b.r),
            g: lerp(a.g, b.g),
            b: lerp(a.b, b.b),
            a: alpha,
        },
        Space::LinearSrgb => {
            let (la, lb) = (linear(a), linear(b));
            let channel = |i: usize| super::from_linear(lerp(la[i], lb[i]));
            Color {
                r: channel(0),
                g: channel(1),
                b: channel(2),
                a: alpha,
            }
            .clamped()
        }
        Space::Oklab => {
            let (x, y) = (a.to_oklab(), b.to_oklab());
            Color::from_oklab(
                Oklab {
                    l: lerp(x.l, y.l),
                    a: lerp(x.a, y.a),
                    b: lerp(x.b, y.b),
                },
                alpha,
            )
            .clamped()
        }
        Space::Oklch => {
            let (x, y) = (a.to_oklch(), b.to_oklch());
            // Take the shorter way around the hue circle; greys take the other color's hue.
            let (hx, hy) = match (x.c < 1e-4, y.c < 1e-4) {
                (true, false) => (y.h, y.h),
                (false, true) => (x.h, x.h),
                _ => (x.h, y.h),
            };
            let delta = (hy - hx + 540.0).rem_euclid(360.0) - 180.0;
            let lch = Oklch {
                l: lerp(x.l, y.l),
                c: lerp(x.c, y.c),
                h: (hx + delta * t).rem_euclid(360.0),
            };
            Color::from_oklch_in_gamut(lch, alpha)
        }
    }
}

fn linear(color: Color) -> [f64; 3] {
    [
        super::to_linear(color.r),
        super::to_linear(color.g),
        super::to_linear(color.b),
    ]
}

/// The color at `t` (0–1) along evenly spaced `stops`.
pub fn sample(stops: &[Color], t: f64, space: Space) -> Color {
    match stops {
        [] => Color::BLACK,
        [only] => *only,
        _ => {
            let position = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
            let index = (position.floor() as usize).min(stops.len() - 2);
            mix(
                stops[index],
                stops[index + 1],
                position - index as f64,
                space,
            )
        }
    }
}

/// The most colors [`steps`] returns, so a huge count can't exhaust memory.
pub const MAX_STEPS: usize = 1000;

/// `count` evenly spaced colors, from the first stop to the last. At most [`MAX_STEPS`].
pub fn steps(stops: &[Color], count: usize, space: Space) -> Vec<Color> {
    let count = count.min(MAX_STEPS);
    match count {
        0 => Vec::new(),
        1 => vec![sample(stops, 0.0, space)],
        _ => (0..count)
            .map(|i| sample(stops, i as f64 / (count - 1) as f64, space))
            .collect(),
    }
}

/// Modern CSS: `linear-gradient(in oklab 90deg, #a, #b)`. Browsers blend in that space
/// themselves (Chrome 111+, Safari 16.2+, Firefox 127+).
pub fn css_gradient(stops: &[Color], angle: f64, space: Space) -> String {
    let colors: Vec<String> = stops.iter().map(|c| c.hex()).collect();
    let angle = super::number(angle, 1);
    match space {
        Space::Srgb => format!("linear-gradient({angle}deg, {})", colors.join(", ")),
        _ => format!(
            "linear-gradient(in {} {angle}deg, {})",
            space.css(),
            colors.join(", ")
        ),
    }
}

/// CSS that looks the same in every browser: the gradient pre-blended into `samples` sRGB
/// stops with positions.
pub fn css_gradient_fallback(stops: &[Color], angle: f64, space: Space, samples: usize) -> String {
    let samples = samples.max(2);
    let colors: Vec<String> = steps(stops, samples, space)
        .iter()
        .enumerate()
        .map(|(i, color)| {
            format!(
                "{} {}%",
                color.hex(),
                super::number(i as f64 * 100.0 / (samples - 1) as f64, 1)
            )
        })
        .collect();
    format!(
        "linear-gradient({}deg, {})",
        super::number(angle, 1),
        colors.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_step_counts_are_capped() {
        assert_eq!(
            steps(&[RED, BLUE], usize::MAX, Space::Oklab).len(),
            MAX_STEPS
        );
        assert!(
            steps(&[], 3, Space::Srgb)
                .iter()
                .all(|c| *c == Color::BLACK)
        );
    }

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    #[test]
    fn endpoints_are_the_stops() {
        for space in Space::ALL {
            let colors = steps(&[RED, Color::WHITE, BLUE], 5, space);
            assert_eq!(colors.len(), 5);
            assert_eq!(colors[0].hex(), "#ff0000", "{space:?}");
            assert_eq!(colors[2].hex(), "#ffffff", "{space:?}");
            assert_eq!(colors[4].hex(), "#0000ff", "{space:?}");
        }
    }

    #[test]
    fn blends_differ_by_space() {
        let srgb = mix(Color::BLACK, Color::WHITE, 0.5, Space::Srgb);
        assert_eq!(srgb.hex(), "#808080");
        let linear = mix(Color::BLACK, Color::WHITE, 0.5, Space::LinearSrgb);
        assert_eq!(linear.hex(), "#bcbcbc");
        // sRGB's red-blue midpoint is dull purple; OKLab's is brighter.
        let dull = mix(RED, BLUE, 0.5, Space::Srgb);
        let smooth = mix(RED, BLUE, 0.5, Space::Oklab);
        assert!(smooth.to_oklch().l > dull.to_oklch().l);
        // OKLCH goes around the hue wheel, so the midpoint stays vivid.
        let vivid = mix(RED, BLUE, 0.5, Space::Oklch);
        assert!(vivid.to_oklch().c > smooth.to_oklch().c);
    }

    #[test]
    fn css_output() {
        assert_eq!(
            css_gradient(&[RED, BLUE], 90.0, Space::Oklab),
            "linear-gradient(in oklab 90deg, #ff0000, #0000ff)"
        );
        assert_eq!(
            css_gradient(&[RED, BLUE], 45.5, Space::Srgb),
            "linear-gradient(45.5deg, #ff0000, #0000ff)"
        );
        let fallback = css_gradient_fallback(&[RED, BLUE], 90.0, Space::Oklab, 5);
        assert!(
            fallback.starts_with("linear-gradient(90deg, #ff0000 0%, #"),
            "{fallback}"
        );
        assert!(fallback.ends_with("#0000ff 100%)"), "{fallback}");
        assert_eq!(fallback.matches('%').count(), 5);
    }
}
