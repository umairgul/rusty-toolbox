use crate::color::RgbaColor;
use toolbox_core::color::{self, Color, Harmony};

/// One step of the 50–950 scale.
#[derive(uniffi::Record)]
pub struct PaletteShade {
    /// 50, 100, … 900, 950.
    pub step: u16,
    pub color: RgbaColor,
}

#[derive(uniffi::Record)]
pub struct PaletteScale {
    /// Light to dark.
    pub shades: Vec<PaletteShade>,
    /// The step whose lightness is closest to the base color's.
    pub nearest_step: u16,
}

/// A color harmony, e.g. label "Triadic" and three colors starting with the base.
#[derive(uniffi::Record)]
pub struct PaletteHarmony {
    pub label: String,
    pub colors: Vec<RgbaColor>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum PaletteExport {
    CssVariables,
    /// A Tailwind CSS v4 `@theme` block in OKLCH.
    Tailwind,
    Json,
}

/// The shades are built from the opaque base color.
fn opaque(color: RgbaColor) -> Color {
    Color::from(color).with_alpha(1.0)
}

/// The 11-step shade scale for `base`, keeping its hue. Alpha is ignored.
#[uniffi::export]
pub fn palettes_scale(base: RgbaColor) -> PaletteScale {
    let base = opaque(base);
    PaletteScale {
        shades: color::shades(base)
            .into_iter()
            .map(|(step, color)| PaletteShade {
                step,
                color: color.into(),
            })
            .collect(),
        nearest_step: color::nearest_shade(base),
    }
}

/// Every harmony of `base`, in display order. Alpha is ignored.
#[uniffi::export]
pub fn palettes_harmonies(base: RgbaColor) -> Vec<PaletteHarmony> {
    let base = opaque(base);
    Harmony::ALL
        .iter()
        .map(|&kind| PaletteHarmony {
            label: kind.label().into(),
            colors: color::harmony(base, kind)
                .into_iter()
                .map(Into::into)
                .collect(),
        })
        .collect()
}

/// The shade scale of `base` as CSS variables, a Tailwind theme or JSON, named after `name`
/// (made CSS-safe, e.g. "Brand Blue" becomes "brand-blue").
#[uniffi::export]
pub fn palettes_export(base: RgbaColor, name: &str, export: PaletteExport) -> String {
    let shades = color::shades(opaque(base));
    match export {
        PaletteExport::CssVariables => color::css_variables(name, &shades),
        PaletteExport::Tailwind => color::tailwind_theme(name, &shades),
        PaletteExport::Json => color::json_palette(name, &shades),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLUE: RgbaColor = RgbaColor {
        r: 59.0 / 255.0,
        g: 130.0 / 255.0,
        b: 246.0 / 255.0,
        a: 1.0,
    };

    #[test]
    fn scale_ignores_alpha() {
        let scale = palettes_scale(BLUE);
        assert_eq!(scale.shades.len(), 11);
        assert_eq!(scale.nearest_step, 500);
        let translucent = palettes_scale(RgbaColor { a: 0.3, ..BLUE });
        assert!(translucent.shades.iter().all(|shade| shade.color.a == 1.0));
        assert_eq!(translucent.nearest_step, 500);
    }

    #[test]
    fn harmonies_and_exports() {
        let harmonies = palettes_harmonies(BLUE);
        assert_eq!(harmonies.len(), Harmony::ALL.len());
        assert_eq!(harmonies[0].label, "Complementary");
        assert_eq!(harmonies[0].colors.len(), 2);
        assert!(
            palettes_export(BLUE, "Brand", PaletteExport::CssVariables).contains("--brand-500")
        );
        assert!(palettes_export(BLUE, "brand", PaletteExport::Tailwind).starts_with("@theme {"));
        assert!(palettes_export(BLUE, "brand", PaletteExport::Json).contains("\"brand\": {"));
    }
}
