//! Colors: parsing any CSS color, converting between color models, WCAG contrast, palettes
//! and gradients.
//!
//! OKLab and OKLCH follow Björn Ottosson's definitions (<https://bottosson.github.io/posts/oklab/>),
//! the same ones CSS Color 4 uses.

mod contrast;
mod gradient;
mod names;
mod palette;

pub use contrast::{Level, contrast_ratio, suggest_for_contrast};
pub use gradient::{Space, css_gradient, css_gradient_fallback, mix, sample, steps};
pub use names::{NAMED_COLORS, named, nearest_name};
pub use palette::{
    Harmony, SHADE_STEPS, css_variables, harmony, json_palette, nearest_shade, shades,
    tailwind_theme,
};

use std::fmt;

/// An sRGB color with straight (not premultiplied) alpha. Components are 0–1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab {
    /// Perceived lightness, 0–1.
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    /// Perceived lightness, 0–1.
    pub l: f64,
    /// Chroma: 0 is grey; the most vivid sRGB colors reach about 0.32.
    pub c: f64,
    /// Hue in degrees, 0–360.
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

fn error(message: impl Into<String>) -> ParseError {
    ParseError(message.into())
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgba8(r, g, b, 255)
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
        let unit = |v: u8| f64::from(v) / 255.0;
        Color {
            r: unit(r),
            g: unit(g),
            b: unit(b),
            a: unit(a),
        }
    }

    /// Red, green, blue and alpha as 0–255, rounded.
    pub fn to_rgba8(self) -> [u8; 4] {
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [byte(self.r), byte(self.g), byte(self.b), byte(self.a)]
    }

    pub fn with_alpha(self, a: f64) -> Color {
        Color {
            a: a.clamp(0.0, 1.0),
            ..self
        }
    }

    pub fn is_opaque(self) -> bool {
        self.a >= 1.0
    }

    /// Whether the color fits in sRGB without clipping (tolerating rounding noise).
    pub fn in_gamut(self) -> bool {
        const EPSILON: f64 = 1e-4;
        [self.r, self.g, self.b]
            .iter()
            .all(|v| (-EPSILON..=1.0 + EPSILON).contains(v))
    }

    pub fn clamped(self) -> Color {
        Color {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// This color drawn over an opaque `background`.
    pub fn over(self, background: Color) -> Color {
        let mix = |front: f64, back: f64| front * self.a + back * (1.0 - self.a);
        Color {
            r: mix(self.r, background.r),
            g: mix(self.g, background.g),
            b: mix(self.b, background.b),
            a: 1.0,
        }
    }

    /// `#rrggbb`, or `#rrggbbaa` when not opaque.
    pub fn hex(self) -> String {
        let [r, g, b, a] = self.to_rgba8();
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }

    /// (hue in degrees, saturation 0–1, lightness 0–1).
    pub fn to_hsl(self) -> (f64, f64, f64) {
        let Color { r, g, b, .. } = self.clamped();
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let lightness = (max + min) / 2.0;
        let delta = max - min;
        if delta == 0.0 {
            return (0.0, 0.0, lightness);
        }
        let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
        (hue(r, g, b, max, delta), saturation, lightness)
    }

    pub fn from_hsl(h: f64, s: f64, l: f64, a: f64) -> Color {
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
        from_hue_chroma(h, chroma, l - chroma / 2.0, a)
    }

    /// (hue in degrees, saturation 0–1, value 0–1), also called HSB.
    pub fn to_hsv(self) -> (f64, f64, f64) {
        let Color { r, g, b, .. } = self.clamped();
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let saturation = if max == 0.0 { 0.0 } else { delta / max };
        let hue = if delta == 0.0 {
            0.0
        } else {
            hue(r, g, b, max, delta)
        };
        (hue, saturation, max)
    }

    pub fn from_hsv(h: f64, s: f64, v: f64, a: f64) -> Color {
        let chroma = v.clamp(0.0, 1.0) * s.clamp(0.0, 1.0);
        from_hue_chroma(h, chroma, v.clamp(0.0, 1.0) - chroma, a)
    }

    /// (cyan, magenta, yellow, black), each 0–1, by the simple device-independent formula.
    pub fn to_cmyk(self) -> (f64, f64, f64, f64) {
        let Color { r, g, b, .. } = self.clamped();
        let k = 1.0 - r.max(g).max(b);
        if k >= 1.0 {
            return (0.0, 0.0, 0.0, 1.0);
        }
        (
            (1.0 - r - k) / (1.0 - k),
            (1.0 - g - k) / (1.0 - k),
            (1.0 - b - k) / (1.0 - k),
            k,
        )
    }

    pub fn from_cmyk(c: f64, m: f64, y: f64, k: f64, a: f64) -> Color {
        let channel = |v: f64| (1.0 - v.clamp(0.0, 1.0)) * (1.0 - k.clamp(0.0, 1.0));
        Color {
            r: channel(c),
            g: channel(m),
            b: channel(y),
            a,
        }
    }

    pub fn to_oklab(self) -> Oklab {
        let (r, g, b) = (to_linear(self.r), to_linear(self.g), to_linear(self.b));
        let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
        let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
        let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
        Oklab {
            l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
            a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
            b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
        }
    }

    /// May be outside sRGB; see [`Color::in_gamut`].
    pub fn from_oklab(lab: Oklab, alpha: f64) -> Color {
        let l = (lab.l + 0.396_337_777_4 * lab.a + 0.215_803_757_3 * lab.b).powi(3);
        let m = (lab.l - 0.105_561_345_8 * lab.a - 0.063_854_172_8 * lab.b).powi(3);
        let s = (lab.l - 0.089_484_177_5 * lab.a - 1.291_485_548_0 * lab.b).powi(3);
        Color {
            r: from_linear(4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s),
            g: from_linear(-1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s),
            b: from_linear(-0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s),
            a: alpha,
        }
    }

    pub fn to_oklch(self) -> Oklch {
        let lab = self.to_oklab();
        let c = lab.a.hypot(lab.b);
        // Hue is meaningless for greys; report 0 instead of rounding noise.
        let h = if c < 1e-4 {
            0.0
        } else {
            lab.b.atan2(lab.a).to_degrees().rem_euclid(360.0)
        };
        Oklch { l: lab.l, c, h }
    }

    /// May be outside sRGB; see [`Color::from_oklch_in_gamut`].
    pub fn from_oklch(lch: Oklch, alpha: f64) -> Color {
        let (sin, cos) = lch.h.to_radians().sin_cos();
        Color::from_oklab(
            Oklab {
                l: lch.l,
                a: lch.c * cos,
                b: lch.c * sin,
            },
            alpha,
        )
    }

    /// Like [`Color::from_oklch`], but lowers chroma until the color fits in sRGB, keeping
    /// its lightness and hue (the CSS Color 4 approach).
    pub fn from_oklch_in_gamut(lch: Oklch, alpha: f64) -> Color {
        let lch = Oklch {
            l: lch.l.clamp(0.0, 1.0),
            ..lch
        };
        let exact = Color::from_oklch(lch, alpha);
        if exact.in_gamut() {
            return exact.clamped();
        }
        let (mut low, mut high) = (0.0, lch.c);
        for _ in 0..24 {
            let mid = (low + high) / 2.0;
            if Color::from_oklch(Oklch { c: mid, ..lch }, alpha).in_gamut() {
                low = mid
            } else {
                high = mid
            }
        }
        Color::from_oklch(Oklch { c: low, ..lch }, alpha).clamped()
    }

    /// WCAG 2 relative luminance, 0 (black) to 1 (white).
    pub fn relative_luminance(self) -> f64 {
        0.2126 * to_linear(self.r) + 0.7152 * to_linear(self.g) + 0.0722 * to_linear(self.b)
    }

    pub fn format(self, format: Format) -> String {
        let alpha = (!self.is_opaque()).then(|| number(self.a, 3));
        match format {
            Format::Hex => self.hex(),
            Format::Rgb => {
                let [r, g, b, _] = self.to_rgba8();
                match alpha {
                    Some(a) => format!("rgba({r}, {g}, {b}, {a})"),
                    None => format!("rgb({r}, {g}, {b})"),
                }
            }
            Format::Hsl => {
                let (h, s, l) = self.to_hsl();
                let (h, s, l) = (number(h, 1), number(s * 100.0, 1), number(l * 100.0, 1));
                match alpha {
                    Some(a) => format!("hsla({h}, {s}%, {l}%, {a})"),
                    None => format!("hsl({h}, {s}%, {l}%)"),
                }
            }
            Format::Hsv => {
                let (h, s, v) = self.to_hsv();
                format!(
                    "hsv({}, {}%, {}%)",
                    number(h, 1),
                    number(s * 100.0, 1),
                    number(v * 100.0, 1)
                )
            }
            Format::Oklch => {
                let lch = self.to_oklch();
                let body = format!(
                    "{}% {} {}",
                    number(lch.l * 100.0, 1),
                    number(lch.c, 3),
                    number(lch.h, 1)
                );
                match alpha {
                    Some(a) => format!("oklch({body} / {a})"),
                    None => format!("oklch({body})"),
                }
            }
            Format::Oklab => {
                let lab = self.to_oklab();
                let body = format!(
                    "{}% {} {}",
                    number(lab.l * 100.0, 1),
                    number(lab.a, 3),
                    number(lab.b, 3)
                );
                match alpha {
                    Some(a) => format!("oklab({body} / {a})"),
                    None => format!("oklab({body})"),
                }
            }
            Format::Cmyk => {
                let (c, m, y, k) = self.to_cmyk();
                let percent = |v: f64| number(v * 100.0, 0);
                format!(
                    "cmyk({}%, {}%, {}%, {}%)",
                    percent(c),
                    percent(m),
                    percent(y),
                    percent(k)
                )
            }
            Format::SwiftUi => {
                let Color { r, g, b, a } = self.clamped();
                let rgb = format!(
                    "red: {}, green: {}, blue: {}",
                    number(r, 3),
                    number(g, 3),
                    number(b, 3)
                );
                if self.is_opaque() {
                    format!(
                        "Color(red: {}, green: {}, blue: {})",
                        number(r, 3),
                        number(g, 3),
                        number(b, 3)
                    )
                } else {
                    format!("Color({rgb}, opacity: {})", number(a, 3))
                }
            }
            Format::Flutter => {
                let [r, g, b, a] = self.to_rgba8();
                format!("Color(0x{a:02X}{r:02X}{g:02X}{b:02X})")
            }
        }
    }
}

/// Output formats, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Hex,
    Rgb,
    Hsl,
    Hsv,
    Oklch,
    Oklab,
    Cmyk,
    SwiftUi,
    Flutter,
}

impl Format {
    pub const ALL: [Format; 9] = [
        Format::Hex,
        Format::Rgb,
        Format::Hsl,
        Format::Hsv,
        Format::Oklch,
        Format::Oklab,
        Format::Cmyk,
        Format::SwiftUi,
        Format::Flutter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Format::Hex => "HEX",
            Format::Rgb => "RGB",
            Format::Hsl => "HSL",
            Format::Hsv => "HSV / HSB",
            Format::Oklch => "OKLCH",
            Format::Oklab => "OKLab",
            Format::Cmyk => "CMYK",
            Format::SwiftUi => "SwiftUI",
            Format::Flutter => "Flutter",
        }
    }
}

fn hue(r: f64, g: f64, b: f64, max: f64, delta: f64) -> f64 {
    let sector = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    sector * 60.0
}

/// Shared by HSL and HSV: places `chroma` in the hue's sector and adds `offset` to all channels.
fn from_hue_chroma(h: f64, chroma: f64, offset: f64, a: f64) -> Color {
    let sector = h.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    Color {
        r: r + offset,
        g: g + offset,
        b: b + offset,
        a: a.clamp(0.0, 1.0),
    }
}

fn to_linear(v: f64) -> f64 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(v: f64) -> f64 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// `value` rounded to `decimals`, without trailing zeros: 62.30 → "62.3", 1.0 → "1".
pub(crate) fn number(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    };
    if text == "-0" { "0".into() } else { text }
}

/// Parses any CSS color: hex (`#rgb`, `#rrggbbaa`, also without `#` or with `0x`), `rgb()`,
/// `hsl()`, `hsv()`, `oklch()`, `oklab()`, `cmyk()`, named colors and `transparent`.
pub fn parse(input: &str) -> Result<Color, ParseError> {
    let text = input
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_ascii_lowercase();
    if text.is_empty() {
        return Err(error("Enter a color"));
    }
    if text == "transparent" {
        return Ok(Color {
            a: 0.0,
            ..Color::BLACK
        });
    }
    if let Some(color) = named(&text) {
        return Ok(color);
    }
    if let Some(hex) = text.strip_prefix('#').or_else(|| text.strip_prefix("0x")) {
        return parse_hex(hex);
    }
    if let Some((function, rest)) = text.split_once('(') {
        let arguments = rest
            .strip_suffix(')')
            .ok_or_else(|| error("Missing closing parenthesis"))?;
        return parse_function(function.trim(), arguments);
    }
    if matches!(text.len(), 3 | 4 | 6 | 8) && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return parse_hex(&text);
    }
    Err(error(
        "Not a recognized color. Try #3b82f6, rgb(59, 130, 246), hsl(217, 91%, 60%), oklch(62% 0.19 260) or a name like tomato",
    ))
}

fn parse_hex(hex: &str) -> Result<Color, ParseError> {
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()
        .ok_or_else(|| error("Hex colors use only 0–9 and a–f"))?;
    let rgba = match digits.len() {
        3 | 4 => {
            let alpha = digits.get(3).map_or(255, |d| d * 17);
            [digits[0] * 17, digits[1] * 17, digits[2] * 17, alpha]
        }
        6 | 8 => {
            let byte = |i: usize| digits[i] * 16 + digits[i + 1];
            [
                byte(0),
                byte(2),
                byte(4),
                if digits.len() == 8 { byte(6) } else { 255 },
            ]
        }
        _ => return Err(error("Hex colors have 3, 4, 6 or 8 digits")),
    };
    Ok(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]))
}

/// One argument of a color function: a number, optionally with `%` or an angle unit.
#[derive(Clone, Copy)]
struct Argument {
    value: f64,
    percent: bool,
}

impl Argument {
    /// A percentage as a fraction; a plain number is taken as-is when `percent_only` is false,
    /// or as a percentage when true (CSS allows `hsl(217 91 60)`).
    fn fraction(self, percent_only: bool) -> f64 {
        if self.percent || percent_only {
            self.value / 100.0
        } else {
            self.value
        }
    }
}

fn parse_arguments(arguments: &str) -> Result<Vec<Argument>, ParseError> {
    arguments
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (number, percent, degrees_per_unit) = if let Some(n) = part.strip_suffix('%') {
                (n, true, 1.0)
            } else if let Some(n) = part.strip_suffix("deg") {
                (n, false, 1.0)
            } else if let Some(n) = part.strip_suffix("turn") {
                (n, false, 360.0)
            } else if let Some(n) = part.strip_suffix("rad") {
                (n, false, 180.0 / std::f64::consts::PI)
            } else {
                (part, false, 1.0)
            };
            let value: f64 = if number == "none" {
                0.0
            } else {
                number
                    .parse()
                    .map_err(|_| error(format!("'{part}' isn't a number")))?
            };
            Ok(Argument {
                value: value * degrees_per_unit,
                percent,
            })
        })
        .collect()
}

fn parse_function(function: &str, arguments: &str) -> Result<Color, ParseError> {
    let args = parse_arguments(arguments)?;
    let expected = if function == "cmyk" { 4 } else { 3 };
    if args.len() != expected && args.len() != expected + 1 {
        return Err(error(format!(
            "{function}() takes {expected} values, plus an optional alpha"
        )));
    }
    let alpha = args
        .get(expected)
        .map_or(1.0, |a| a.fraction(false))
        .clamp(0.0, 1.0);
    let color = match function {
        "rgb" | "rgba" => {
            let channel = |a: Argument| {
                if a.percent {
                    a.value / 100.0
                } else {
                    a.value / 255.0
                }
            };
            Color {
                r: channel(args[0]),
                g: channel(args[1]),
                b: channel(args[2]),
                a: alpha,
            }
            .clamped()
        }
        "hsl" | "hsla" => Color::from_hsl(
            args[0].value,
            args[1].fraction(true),
            args[2].fraction(true),
            alpha,
        ),
        "hsv" | "hsb" => Color::from_hsv(
            args[0].value,
            args[1].fraction(true),
            args[2].fraction(true),
            alpha,
        ),
        "oklch" => {
            // Chroma 100% is 0.4 in CSS.
            let c = if args[1].percent {
                args[1].value / 100.0 * 0.4
            } else {
                args[1].value
            };
            Color::from_oklch_in_gamut(
                Oklch {
                    l: args[0].fraction(false),
                    c,
                    h: args[2].value,
                },
                alpha,
            )
        }
        "oklab" => {
            let axis = |a: Argument| {
                if a.percent {
                    a.value / 100.0 * 0.4
                } else {
                    a.value
                }
            };
            let lab = Oklab {
                l: args[0].fraction(false),
                a: axis(args[1]),
                b: axis(args[2]),
            };
            Color::from_oklab(lab, alpha).clamped()
        }
        "cmyk" | "device-cmyk" => Color::from_cmyk(
            args[0].fraction(true),
            args[1].fraction(true),
            args[2].fraction(true),
            args[3].fraction(true),
            alpha,
        ),
        other => return Err(error(format!("Unknown color function {other}()"))),
    };
    Ok(color)
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

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn parses_hex_forms() {
        assert_eq!(parse("#3b82f6").unwrap().hex(), "#3b82f6");
        assert_eq!(parse("3B82F6").unwrap().hex(), "#3b82f6");
        assert_eq!(parse("0x3b82f6").unwrap().hex(), "#3b82f6");
        assert_eq!(parse("#fff").unwrap(), Color::WHITE);
        assert_eq!(parse("#3b82f680").unwrap().to_rgba8(), [59, 130, 246, 128]);
        assert_eq!(parse("#f008").unwrap().to_rgba8(), [255, 0, 0, 136]);
        assert!(parse("#12345").is_err());
        assert!(parse("#ggg").is_err());
    }

    #[test]
    fn parses_functions_and_names() {
        let expect =
            |input: &str, hex: &str| assert_eq!(parse(input).unwrap().hex(), hex, "{input}");
        expect("rgb(59, 130, 246)", "#3b82f6");
        expect("rgb(59 130 246 / 50%)", "#3b82f680");
        expect("rgba(100%, 0%, 0%, 0.5)", "#ff000080");
        expect("hsl(217.2, 91.2%, 59.8%)", "#3b82f6");
        expect("hsl(217.2 91.2 59.8)", "#3b82f6");
        expect("hsl(0.5turn, 100%, 50%)", "#00ffff");
        expect("hsv(217, 76%, 96%)", "#3b82f5");
        expect("oklch(62.8% 0.2577 29.23)", "#ff0000");
        expect("oklch(0.623 0.188 259.8)", "#3b82f6");
        expect("oklab(0.628 0.225 0.126)", "#ff0000");
        expect("cmyk(76%, 47%, 0%, 4%)", "#3b82f5");
        expect("RebeccaPurple", "#663399");
        expect("  tomato; ", "#ff6347");
        assert_eq!(parse("transparent").unwrap().a, 0.0);
        assert!(parse("rgb(1, 2)").is_err());
        assert!(parse("lab(50 20 30)").is_err());
        assert!(parse("not a color").is_err());
    }

    #[test]
    fn converts_hsl_and_hsv() {
        let (h, s, l) = BLUE_500.to_hsl();
        assert!(
            close(h, 217.2, 0.1) && close(s, 0.912, 0.001) && close(l, 0.598, 0.001),
            "{h} {s} {l}"
        );
        let (h, s, v) = BLUE_500.to_hsv();
        assert!(
            close(h, 217.2, 0.1) && close(s, 0.760, 0.001) && close(v, 0.965, 0.001),
            "{h} {s} {v}"
        );
        for color in [
            BLUE_500,
            Color::from_rgb8(200, 30, 90),
            Color::from_rgb8(12, 250, 100),
        ] {
            let (h, s, l) = color.to_hsl();
            assert_eq!(Color::from_hsl(h, s, l, 1.0).hex(), color.hex());
            let (h, s, v) = color.to_hsv();
            assert_eq!(Color::from_hsv(h, s, v, 1.0).hex(), color.hex());
        }
    }

    #[test]
    fn matches_css_oklch_reference_values() {
        // Reference values from the CSS Color 4 sample code.
        let red = Color::from_rgb8(255, 0, 0).to_oklch();
        assert!(
            close(red.l, 0.627_955, 1e-4)
                && close(red.c, 0.257_683, 1e-4)
                && close(red.h, 29.2339, 0.01),
            "{red:?}"
        );
        let blue = Color::from_rgb8(0, 0, 255).to_oklch();
        assert!(
            close(blue.l, 0.452_014, 1e-4)
                && close(blue.c, 0.313_214, 1e-4)
                && close(blue.h, 264.052, 0.01),
            "{blue:?}"
        );
        let white = Color::WHITE.to_oklch();
        assert!(
            close(white.l, 1.0, 1e-4) && white.c < 1e-4 && white.h == 0.0,
            "{white:?}"
        );
        for color in [
            BLUE_500,
            Color::from_rgb8(1, 2, 3),
            Color::from_rgb8(250, 200, 10),
        ] {
            assert_eq!(Color::from_oklch(color.to_oklch(), 1.0).hex(), color.hex());
        }
    }

    #[test]
    fn maps_out_of_gamut_colors_by_reducing_chroma() {
        let wild = Oklch {
            l: 0.7,
            c: 0.4,
            h: 150.0,
        };
        assert!(!Color::from_oklch(wild, 1.0).in_gamut());
        let mapped = Color::from_oklch_in_gamut(wild, 1.0);
        assert!(mapped.in_gamut());
        let lch = mapped.to_oklch();
        assert!(
            close(lch.l, 0.7, 0.01) && close(lch.h, 150.0, 1.0) && lch.c < 0.4,
            "{lch:?}"
        );
    }

    #[test]
    fn formats_every_model() {
        let format = |f: Format| BLUE_500.format(f);
        assert_eq!(format(Format::Hex), "#3b82f6");
        assert_eq!(format(Format::Rgb), "rgb(59, 130, 246)");
        assert_eq!(format(Format::Hsl), "hsl(217.2, 91.2%, 59.8%)");
        assert_eq!(format(Format::Hsv), "hsv(217.2, 76%, 96.5%)");
        assert_eq!(format(Format::Oklch), "oklch(62.3% 0.188 259.8)");
        assert_eq!(format(Format::Cmyk), "cmyk(76%, 47%, 0%, 4%)");
        assert_eq!(
            format(Format::SwiftUi),
            "Color(red: 0.231, green: 0.51, blue: 0.965)"
        );
        assert_eq!(format(Format::Flutter), "Color(0xFF3B82F6)");
        let translucent = BLUE_500.with_alpha(0.5);
        assert_eq!(translucent.format(Format::Rgb), "rgba(59, 130, 246, 0.5)");
        assert_eq!(
            translucent.format(Format::Oklch),
            "oklch(62.3% 0.188 259.8 / 0.5)"
        );
        assert_eq!(translucent.format(Format::Hex), "#3b82f680");
        // CMYK is shown in whole percentages, so it can't round-trip exactly.
        let exact = |f: &Format| {
            !matches!(
                f,
                Format::SwiftUi | Format::Flutter | Format::Hsv | Format::Cmyk
            )
        };
        for f in Format::ALL.into_iter().filter(exact) {
            assert_eq!(
                parse(&BLUE_500.format(f)).unwrap().hex(),
                "#3b82f6",
                "{f:?} round trip"
            );
        }
    }

    #[test]
    fn composites_alpha() {
        let half_black = Color::BLACK.with_alpha(0.5);
        assert_eq!(half_black.over(Color::WHITE).hex(), "#808080");
    }
}
