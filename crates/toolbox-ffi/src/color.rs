//! Colors shared by the color tools. Each tool's own functions live in its module and take
//! and return [`RgbaColor`].

use crate::ToolError;
use toolbox_core::color::{self, Color, Format};

/// An sRGB color with straight alpha. Components are 0–1.
#[derive(Clone, Copy, uniffi::Record)]
pub struct RgbaColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl From<Color> for RgbaColor {
    fn from(Color { r, g, b, a }: Color) -> Self {
        RgbaColor { r, g, b, a }
    }
}

impl From<RgbaColor> for Color {
    fn from(RgbaColor { r, g, b, a }: RgbaColor) -> Self {
        Color { r, g, b, a }
    }
}

/// One way of writing a color, e.g. label "HSL", value "hsl(217.2, 91.2%, 59.8%)".
#[derive(uniffi::Record)]
pub struct ColorFormatValue {
    pub label: String,
    pub value: String,
}

#[derive(uniffi::Record)]
pub struct ColorName {
    pub name: String,
    /// True when the color is exactly this CSS named color, false when it's only the nearest.
    pub exact: bool,
}

/// Parses any CSS color: hex, `rgb()`, `hsl()`, `hsv()`, `oklch()`, `oklab()`, `cmyk()`,
/// named colors and `transparent`.
#[uniffi::export]
pub fn color_parse(input: &str) -> Result<RgbaColor, ToolError> {
    color::parse(input).map(Into::into).map_err(ToolError::new)
}

/// `#rrggbb`, or `#rrggbbaa` when not opaque.
#[uniffi::export]
pub fn color_hex(color: RgbaColor) -> String {
    Color::from(color).hex()
}

/// The color in every supported format, in display order.
#[uniffi::export]
pub fn color_formats(color: RgbaColor) -> Vec<ColorFormatValue> {
    let color = Color::from(color);
    Format::ALL
        .iter()
        .map(|&format| ColorFormatValue {
            label: format.label().into(),
            value: color.format(format),
        })
        .collect()
}

/// The nearest CSS named color, ignoring alpha.
#[uniffi::export]
pub fn color_nearest_name(color: RgbaColor) -> ColorName {
    let (name, distance) = color::nearest_name(Color::from(color).with_alpha(1.0));
    ColorName {
        name: name.into(),
        exact: distance == 0.0,
    }
}

/// Rounds each component to the nearest 8-bit value. Screen and color-panel colors come back
/// as fractions like 0.99998; rounding makes hex codes and named colors match exactly.
#[uniffi::export]
pub fn color_rounded_to_8bit(color: RgbaColor) -> RgbaColor {
    let [r, g, b, a] = Color::from(color).to_rgba8();
    Color::from_rgba8(r, g, b, a).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats() {
        let color = color_parse("tomato").unwrap();
        assert_eq!(color_hex(color), "#ff6347");
        let name = color_nearest_name(color);
        assert_eq!((name.name.as_str(), name.exact), ("tomato", true));
        let formats = color_formats(color);
        assert_eq!(formats[0].label, "HEX");
        assert!(formats.iter().any(|f| f.value == "rgb(255, 99, 71)"));
        assert!(color_parse("not a color").is_err());
    }

    #[test]
    fn rounds_to_8bit() {
        let near_white = RgbaColor {
            r: 0.99998,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        let rounded = color_rounded_to_8bit(near_white);
        assert_eq!(rounded.r, 1.0);
        assert!(color_nearest_name(rounded).exact);
    }
}
