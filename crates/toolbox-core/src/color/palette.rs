//! Palettes: a 50–950 shade scale from one color, color harmonies, and exports for CSS,
//! Tailwind and JSON.

use super::{Color, Oklch, number};

/// Shade names, light to dark, as Tailwind uses them.
pub const SHADE_STEPS: [u16; 11] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];

/// OKLCH lightness for each step, modelled on Tailwind CSS v4's palettes.
const SHADE_LIGHTNESS: [f64; 11] = [
    0.971, 0.932, 0.882, 0.809, 0.707, 0.623, 0.546, 0.488, 0.424, 0.379, 0.282,
];

/// Chroma for each step as a share of the base color's: pale tints and deep shades are less
/// saturated.
const SHADE_CHROMA: [f64; 11] = [0.12, 0.25, 0.45, 0.7, 0.9, 1.0, 1.0, 0.92, 0.82, 0.7, 0.52];

/// The 11-step scale for `base`, keeping its hue and scaling its chroma.
pub fn shades(base: Color) -> Vec<(u16, Color)> {
    let lch = base.with_alpha(1.0).to_oklch();
    SHADE_STEPS
        .iter()
        .zip(SHADE_LIGHTNESS.iter().zip(SHADE_CHROMA))
        .map(|(&step, (&l, chroma_share))| {
            (
                step,
                Color::from_oklch_in_gamut(
                    Oklch {
                        l,
                        c: lch.c * chroma_share,
                        h: lch.h,
                    },
                    1.0,
                ),
            )
        })
        .collect()
}

/// The step whose lightness is closest to `base`'s, e.g. 500 for a mid-tone.
pub fn nearest_shade(base: Color) -> u16 {
    let l = base.to_oklch().l;
    let index = SHADE_LIGHTNESS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - l).abs().total_cmp(&(b.1 - l).abs()))
        .map_or(5, |(i, _)| i);
    SHADE_STEPS[index]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harmony {
    Complementary,
    Analogous,
    Triadic,
    SplitComplementary,
    Tetradic,
}

impl Harmony {
    pub const ALL: [Harmony; 5] = [
        Harmony::Complementary,
        Harmony::Analogous,
        Harmony::Triadic,
        Harmony::SplitComplementary,
        Harmony::Tetradic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Harmony::Complementary => "Complementary",
            Harmony::Analogous => "Analogous",
            Harmony::Triadic => "Triadic",
            Harmony::SplitComplementary => "Split complementary",
            Harmony::Tetradic => "Tetradic (square)",
        }
    }

    /// Hue rotations from the base color, in degrees.
    fn rotations(self) -> &'static [f64] {
        match self {
            Harmony::Complementary => &[0.0, 180.0],
            Harmony::Analogous => &[-30.0, 0.0, 30.0],
            Harmony::Triadic => &[0.0, 120.0, 240.0],
            Harmony::SplitComplementary => &[0.0, 150.0, 210.0],
            Harmony::Tetradic => &[0.0, 90.0, 180.0, 270.0],
        }
    }
}

/// Colors at the harmony's hue rotations, in OKLCH so lightness and vividness stay even.
pub fn harmony(base: Color, kind: Harmony) -> Vec<Color> {
    let lch = base.to_oklch();
    kind.rotations()
        .iter()
        .map(|rotation| {
            if *rotation == 0.0 {
                base
            } else {
                Color::from_oklch_in_gamut(
                    Oklch {
                        h: (lch.h + rotation).rem_euclid(360.0),
                        ..lch
                    },
                    base.a,
                )
            }
        })
        .collect()
}

/// A CSS-safe name: lowercase letters, digits and hyphens, e.g. "Brand Blue" → "brand-blue".
fn variable_name(name: &str) -> String {
    let slug: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if slug.is_empty() {
        "color".into()
    } else {
        slug.join("-")
    }
}

/// `:root { --name-50: #…; … }`
pub fn css_variables(name: &str, shades: &[(u16, Color)]) -> String {
    let name = variable_name(name);
    let lines: Vec<String> = shades
        .iter()
        .map(|(step, color)| format!("  --{name}-{step}: {};", color.hex()))
        .collect();
    format!(":root {{\n{}\n}}\n", lines.join("\n"))
}

/// A Tailwind CSS v4 `@theme` block in OKLCH, like Tailwind's own palettes.
pub fn tailwind_theme(name: &str, shades: &[(u16, Color)]) -> String {
    let name = variable_name(name);
    let lines: Vec<String> = shades
        .iter()
        .map(|(step, color)| {
            let lch = color.to_oklch();
            format!(
                "  --color-{name}-{step}: oklch({}% {} {});",
                number(lch.l * 100.0, 1),
                number(lch.c, 3),
                number(lch.h, 1)
            )
        })
        .collect();
    format!("@theme {{\n{}\n}}\n", lines.join("\n"))
}

/// `{ "name": { "50": "#…", … } }`
pub fn json_palette(name: &str, shades: &[(u16, Color)]) -> String {
    let entries: Vec<String> = shades
        .iter()
        .map(|(step, color)| format!("    \"{step}\": \"{}\"", color.hex()))
        .collect();
    format!(
        "{{\n  \"{}\": {{\n{}\n  }}\n}}\n",
        variable_name(name),
        entries.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLUE_500: Color = Color {
        r: 59.0 / 255.0,
        g: 130.0 / 255.0,
        b: 246.0 / 255.0,
        a: 1.0,
    };

    #[test]
    fn shades_go_from_light_to_dark_with_the_same_hue() {
        let scale = shades(BLUE_500);
        assert_eq!(scale.len(), 11);
        assert!(
            scale
                .windows(2)
                .all(|pair| pair[0].1.relative_luminance() > pair[1].1.relative_luminance())
        );
        let base_hue = BLUE_500.to_oklch().h;
        for (step, color) in &scale {
            assert!(color.in_gamut());
            assert!(
                (color.to_oklch().h - base_hue).abs() < 2.0,
                "{step}: {}",
                color.hex()
            );
        }
        // Tailwind's blue-500 sits at step 500.
        assert_eq!(nearest_shade(BLUE_500), 500);
        assert_eq!(nearest_shade(Color::WHITE), 50);
    }

    #[test]
    fn harmonies_rotate_hue() {
        let triad = harmony(BLUE_500, Harmony::Triadic);
        assert_eq!(triad.len(), 3);
        assert_eq!(triad[0], BLUE_500);
        let hues: Vec<f64> = triad.iter().map(|c| c.to_oklch().h).collect();
        assert!(
            ((hues[1] - hues[0]).rem_euclid(360.0) - 120.0).abs() < 2.0,
            "{hues:?}"
        );
        assert_eq!(harmony(BLUE_500, Harmony::Tetradic).len(), 4);
    }

    #[test]
    fn exports() {
        let scale = shades(BLUE_500);
        let css = css_variables("Brand Blue!", &scale);
        assert!(css.starts_with(":root {\n  --brand-blue-50: #"), "{css}");
        assert_eq!(css.lines().count(), 13);
        let theme = tailwind_theme("brand", &scale);
        assert!(
            theme.contains("  --color-brand-500: oklch(62.3% "),
            "{theme}"
        );
        let json: serde_json::Value = serde_json::from_str(&json_palette("brand", &scale)).unwrap();
        assert_eq!(json["brand"]["950"].as_str().unwrap().len(), 7);
        assert_eq!(variable_name("  "), "color");
    }
}
