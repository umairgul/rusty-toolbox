//! QR code generator for text, URLs and Wi-Fi credentials, rendered as pixels or SVG.

use qrcode::types::Color;
use qrcode::{EcLevel, QrCode};
use std::fmt;

/// Light modules around the code, which scanners need. Four is the standard minimum.
pub const QUIET_ZONE: usize = 4;

/// The largest side, in pixels, that [`QrMatrix::to_pixels`] renders: bigger scales are
/// reduced to fit, so a huge scale can't overflow or exhaust memory.
pub const MAX_PIXEL_SIZE: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

impl ErrorCorrection {
    pub const ALL: [ErrorCorrection; 4] = [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ErrorCorrection::Low => "Low (7%)",
            ErrorCorrection::Medium => "Medium (15%)",
            ErrorCorrection::Quartile => "Quartile (25%)",
            ErrorCorrection::High => "High (30%)",
        }
    }

    fn level(self) -> EcLevel {
        match self {
            ErrorCorrection::Low => EcLevel::L,
            ErrorCorrection::Medium => EcLevel::M,
            ErrorCorrection::Quartile => EcLevel::Q,
            ErrorCorrection::High => EcLevel::H,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiSecurity {
    Wpa,
    Wep,
    Open,
}

impl WifiSecurity {
    pub const ALL: [WifiSecurity; 3] = [WifiSecurity::Wpa, WifiSecurity::Wep, WifiSecurity::Open];

    pub fn label(self) -> &'static str {
        match self {
            WifiSecurity::Wpa => "WPA/WPA2/WPA3",
            WifiSecurity::Wep => "WEP",
            WifiSecurity::Open => "None (open)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    TooLong,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "Enter something to encode"),
            Error::TooLong => write!(
                f,
                "Too much data for a QR code. Shorten it or lower the error correction"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// The text a phone's camera expects for joining a Wi-Fi network:
/// `WIFI:T:WPA;S:<ssid>;P:<password>;;`, with `\ ; , : "` escaped by a backslash.
pub fn wifi_payload(ssid: &str, password: &str, security: WifiSecurity, hidden: bool) -> String {
    fn escape(text: &str) -> String {
        text.chars().fold(String::new(), |mut out, c| {
            if matches!(c, '\\' | ';' | ',' | ':' | '"') {
                out.push('\\');
            }
            out.push(c);
            out
        })
    }
    let mut payload = String::from("WIFI:");
    match security {
        WifiSecurity::Wpa => payload.push_str("T:WPA;"),
        WifiSecurity::Wep => payload.push_str("T:WEP;"),
        WifiSecurity::Open => payload.push_str("T:nopass;"),
    }
    payload.push_str(&format!("S:{};", escape(ssid)));
    if security != WifiSecurity::Open {
        payload.push_str(&format!("P:{};", escape(password)));
    }
    if hidden {
        payload.push_str("H:true;");
    }
    payload.push(';');
    payload
}

/// An encoded QR code: a square grid of dark and light modules, without the quiet zone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrMatrix {
    width: usize,
    dark: Vec<bool>,
}

impl QrMatrix {
    pub fn encode(data: &str, error_correction: ErrorCorrection) -> Result<Self, Error> {
        if data.is_empty() {
            return Err(Error::Empty);
        }
        // Data is always encoded in byte mode at an automatic version, so too much data is the
        // only error that can happen.
        let code = QrCode::with_error_correction_level(data, error_correction.level())
            .map_err(|_| Error::TooLong)?;
        let dark = code
            .to_colors()
            .into_iter()
            .map(|color| color == Color::Dark)
            .collect();
        Ok(Self {
            width: code.width(),
            dark,
        })
    }

    /// Modules per side, without the quiet zone.
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.dark[y * self.width + x]
    }

    /// The scale actually used for `scale`: at least 1, and small enough to keep the side
    /// within [`MAX_PIXEL_SIZE`].
    fn clamp_scale(&self, scale: usize) -> usize {
        let modules = self.width + 2 * QUIET_ZONE;
        scale.clamp(1, (MAX_PIXEL_SIZE / modules).max(1))
    }

    /// Side length in pixels of [`Self::to_pixels`] output.
    pub fn pixel_size(&self, scale: usize) -> usize {
        (self.width + 2 * QUIET_ZONE) * self.clamp_scale(scale)
    }

    /// Greyscale pixels (0 = dark, 255 = light), `scale` pixels per module, row by row,
    /// including the quiet zone.
    pub fn to_pixels(&self, scale: usize) -> Vec<u8> {
        let scale = self.clamp_scale(scale);
        let size = self.pixel_size(scale);
        let mut pixels = vec![255u8; size * size];
        for y in 0..self.width {
            for x in 0..self.width {
                if self.is_dark(x, y) {
                    for py in 0..scale {
                        let row = (y + QUIET_ZONE) * scale + py;
                        let start = row * size + (x + QUIET_ZONE) * scale;
                        pixels[start..start + scale].fill(0);
                    }
                }
            }
        }
        pixels
    }

    /// A scalable SVG with one module per user unit and runs of dark modules merged.
    pub fn to_svg(&self) -> String {
        let size = self.width + 2 * QUIET_ZONE;
        let mut path = String::new();
        for y in 0..self.width {
            let mut x = 0;
            while x < self.width {
                if self.is_dark(x, y) {
                    let start = x;
                    while x < self.width && self.is_dark(x, y) {
                        x += 1;
                    }
                    path.push_str(&format!(
                        "M{} {}h{}v1h-{}z",
                        start + QUIET_ZONE,
                        y + QUIET_ZONE,
                        x - start,
                        x - start
                    ));
                } else {
                    x += 1;
                }
            }
        }
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {size} {size}\" shape-rendering=\"crispEdges\">\
             <rect width=\"{size}\" height=\"{size}\" fill=\"#fff\"/><path fill=\"#000\" d=\"{path}\"/></svg>\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_scales_are_capped() {
        let matrix = QrMatrix::encode("hi", ErrorCorrection::Low).unwrap();
        let size = matrix.pixel_size(usize::MAX);
        assert!(size <= MAX_PIXEL_SIZE);
        assert_eq!(matrix.to_pixels(usize::MAX).len(), size * size);
        assert_eq!(matrix.pixel_size(0), matrix.pixel_size(1));
    }

    /// Decodes rendered pixels with an independent QR reader.
    fn scan(matrix: &QrMatrix) -> String {
        let scale = 4;
        let size = matrix.pixel_size(scale);
        let pixels = matrix.to_pixels(scale);
        let mut image =
            rqrr::PreparedImage::prepare_from_greyscale(size, size, |x, y| pixels[y * size + x]);
        let grids = image.detect_grids();
        assert_eq!(grids.len(), 1, "expected exactly one QR code");
        grids[0].decode().expect("QR code decodes").1
    }

    #[test]
    fn round_trips_through_a_scanner() {
        for text in [
            "hello",
            "https://example.com/path?query=1&x=ü",
            &"long text ".repeat(100),
        ] {
            for level in ErrorCorrection::ALL {
                let matrix = QrMatrix::encode(text, level).unwrap();
                assert_eq!(scan(&matrix), text);
            }
        }
    }

    #[test]
    fn short_text_is_version_one() {
        assert_eq!(
            QrMatrix::encode("hello", ErrorCorrection::Medium)
                .unwrap()
                .width(),
            21
        );
    }

    #[test]
    fn rejects_empty_and_oversized_data() {
        assert_eq!(
            QrMatrix::encode("", ErrorCorrection::Low),
            Err(Error::Empty)
        );
        assert_eq!(
            QrMatrix::encode(&"x".repeat(3000), ErrorCorrection::Low),
            Err(Error::TooLong)
        );
    }

    #[test]
    fn wifi_payload_escapes_special_characters() {
        assert_eq!(
            wifi_payload("Home", "pa;ss:word", WifiSecurity::Wpa, false),
            "WIFI:T:WPA;S:Home;P:pa\\;ss\\:word;;"
        );
        assert_eq!(
            wifi_payload("Café, \"Guest\"", "ignored", WifiSecurity::Open, true),
            "WIFI:T:nopass;S:Café\\, \\\"Guest\\\";H:true;;"
        );
        let payload = wifi_payload("Office", "secret", WifiSecurity::Wpa, false);
        assert_eq!(
            scan(&QrMatrix::encode(&payload, ErrorCorrection::Medium).unwrap()),
            payload
        );
    }

    #[test]
    fn svg_draws_every_dark_module() {
        let matrix = QrMatrix::encode("svg", ErrorCorrection::Low).unwrap();
        let svg = matrix.to_svg();
        assert!(svg.starts_with("<svg") && svg.contains("viewBox=\"0 0 29 29\""));
        // Each run "M.. ..h{n}v1h-{n}z" covers n modules.
        let path = svg.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
        let drawn: usize = path
            .split('h')
            .skip(1)
            .step_by(2)
            .map(|part| part.split('v').next().unwrap().parse::<usize>().unwrap())
            .sum();
        let dark = (0..21)
            .flat_map(|y| (0..21).map(move |x| (x, y)))
            .filter(|&(x, y)| matrix.is_dark(x, y))
            .count();
        assert_eq!(drawn, dark);
    }
}
