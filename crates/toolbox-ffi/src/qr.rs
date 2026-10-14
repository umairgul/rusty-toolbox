use crate::ToolError;
use std::sync::Arc;
use toolbox_core::qr::{self, ErrorCorrection, QrMatrix, WifiSecurity};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum QrErrorCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

impl From<QrErrorCorrection> for ErrorCorrection {
    fn from(level: QrErrorCorrection) -> Self {
        match level {
            QrErrorCorrection::Low => ErrorCorrection::Low,
            QrErrorCorrection::Medium => ErrorCorrection::Medium,
            QrErrorCorrection::Quartile => ErrorCorrection::Quartile,
            QrErrorCorrection::High => ErrorCorrection::High,
        }
    }
}

/// e.g. "Medium (15%)".
#[uniffi::export]
pub fn qr_error_correction_label(level: QrErrorCorrection) -> String {
    ErrorCorrection::from(level).label().into()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum QrWifiSecurity {
    Wpa,
    Wep,
    Open,
}

impl From<QrWifiSecurity> for WifiSecurity {
    fn from(security: QrWifiSecurity) -> Self {
        match security {
            QrWifiSecurity::Wpa => WifiSecurity::Wpa,
            QrWifiSecurity::Wep => WifiSecurity::Wep,
            QrWifiSecurity::Open => WifiSecurity::Open,
        }
    }
}

/// e.g. "WPA/WPA2/WPA3".
#[uniffi::export]
pub fn qr_wifi_security_label(security: QrWifiSecurity) -> String {
    WifiSecurity::from(security).label().into()
}

/// The text a phone's camera expects for joining a Wi-Fi network. The password is left out
/// for open networks.
#[uniffi::export]
pub fn qr_wifi_payload(
    ssid: &str,
    password: &str,
    security: QrWifiSecurity,
    hidden: bool,
) -> String {
    qr::wifi_payload(ssid, password, security.into(), hidden)
}

/// An encoded QR code.
#[derive(uniffi::Object)]
pub struct QrCode(QrMatrix);

#[uniffi::export]
impl QrCode {
    /// Modules per side, without the quiet zone.
    pub fn modules(&self) -> u32 {
        self.0.width() as u32
    }

    /// Side length in pixels of [`Self::pixels`] output, including the quiet zone.
    pub fn pixel_size(&self, scale: u32) -> u32 {
        self.0.pixel_size(scale.max(1) as usize) as u32
    }

    /// Greyscale pixels (0 = dark, 255 = light), `scale` pixels per module, row by row,
    /// including the quiet zone.
    pub fn pixels(&self, scale: u32) -> Vec<u8> {
        self.0.to_pixels(scale.max(1) as usize)
    }

    /// A scalable SVG document.
    pub fn svg(&self) -> String {
        self.0.to_svg()
    }
}

/// Encodes `data`. Fails when it's empty or too long for a QR code at this error correction.
#[uniffi::export]
pub fn qr_encode(
    data: &str,
    error_correction: QrErrorCorrection,
) -> Result<Arc<QrCode>, ToolError> {
    QrMatrix::encode(data, error_correction.into())
        .map(|matrix| Arc::new(QrCode(matrix)))
        .map_err(ToolError::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_match_their_size_and_scale_is_at_least_one() {
        let code = qr_encode("hello", QrErrorCorrection::Medium).unwrap();
        assert_eq!(code.modules(), 21);
        assert_eq!(code.pixel_size(0), code.pixel_size(1));
        let size = code.pixel_size(3) as usize;
        assert_eq!(code.pixels(3).len(), size * size);
        assert!(qr_encode("", QrErrorCorrection::Low).is_err());
    }
}
