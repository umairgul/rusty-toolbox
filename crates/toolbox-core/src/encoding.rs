//! Text encoders and decoders: Base64, URL (percent) encoding, HTML entities and hex.

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    Base64,
    Url,
    Html,
    Hex,
}

impl Codec {
    pub const ALL: [Codec; 4] = [Codec::Base64, Codec::Url, Codec::Html, Codec::Hex];

    pub fn label(self) -> &'static str {
        match self {
            Codec::Base64 => "Base64",
            Codec::Url => "URL",
            Codec::Html => "HTML entities",
            Codec::Hex => "Hex",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Base64: use the URL-safe alphabet (`-` and `_` instead of `+` and `/`).
    pub base64_url_safe: bool,
    /// Base64: end with `=` padding.
    pub base64_padding: bool,
    /// URL: encode spaces as `+` and decode `+` as a space, as HTML forms do.
    pub url_plus_for_space: bool,
    /// HTML: also encode every non-ASCII character as a numeric entity.
    pub html_encode_non_ascii: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            base64_url_safe: false,
            base64_padding: true,
            url_plus_for_space: false,
            html_encode_non_ascii: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidBase64,
    InvalidHex,
    InvalidPercentEncoding,
    /// Decoding worked, but the bytes aren't UTF-8 text. Holds them as hex.
    NotText {
        hex: String,
        len: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidBase64 => write!(f, "Not valid Base64"),
            Error::InvalidHex => write!(f, "Not valid hex: use pairs of 0-9 and a-f"),
            Error::InvalidPercentEncoding => {
                write!(f, "Invalid % escape: expected % followed by two hex digits")
            }
            Error::NotText { len, .. } => write!(f, "Decoded {len} bytes of binary data, not text"),
        }
    }
}

impl std::error::Error for Error {}

pub fn encode(codec: Codec, text: &str, options: &Options) -> String {
    match codec {
        Codec::Base64 => {
            let engine = match (options.base64_url_safe, options.base64_padding) {
                (false, true) => STANDARD,
                (false, false) => STANDARD_NO_PAD,
                (true, true) => URL_SAFE,
                (true, false) => URL_SAFE_NO_PAD,
            };
            engine.encode(text)
        }
        Codec::Url => url_encode(text, options.url_plus_for_space),
        Codec::Html => html_encode(text, options.html_encode_non_ascii),
        Codec::Hex => hex_encode(text.as_bytes()),
    }
}

pub fn decode(codec: Codec, text: &str, options: &Options) -> Result<String, Error> {
    let bytes = match codec {
        Codec::Base64 => decode_base64(text)?,
        Codec::Url => url_decode(text, options.url_plus_for_space)?,
        Codec::Html => return Ok(html_escape::decode_html_entities(text).into_owned()),
        Codec::Hex => hex_decode(text)?,
    };
    String::from_utf8(bytes).map_err(|error| {
        let bytes = error.into_bytes();
        Error::NotText {
            hex: hex_encode(&bytes),
            len: bytes.len(),
        }
    })
}

/// Decodes standard or URL-safe Base64, with or without padding, ignoring whitespace.
pub fn decode_base64(text: &str) -> Result<Vec<u8>, Error> {
    let normalized: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    STANDARD_NO_PAD
        .decode(normalized.trim_end_matches('='))
        .map_err(|_| Error::InvalidBase64)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        hex.push(DIGITS[usize::from(byte >> 4)] as char);
        hex.push(DIGITS[usize::from(byte & 0xf)] as char);
    }
    hex
}

/// Decodes hex, ignoring whitespace, `:` and `-` separators and a leading `0x`.
pub fn hex_decode(text: &str) -> Result<Vec<u8>, Error> {
    let text = text.trim();
    let text = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    let digits: Vec<u8> = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-')
        .map(|c| c.to_digit(16).map(|d| d as u8).ok_or(Error::InvalidHex))
        .collect::<Result<_, _>>()?;
    if !digits.len().is_multiple_of(2) {
        return Err(Error::InvalidHex);
    }
    Ok(digits
        .chunks(2)
        .map(|pair| (pair[0] << 4) | pair[1])
        .collect())
}

/// Percent-encodes everything except RFC 3986 unreserved characters, like JavaScript's
/// `encodeURIComponent`.
fn url_encode(text: &str, plus_for_space: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' if plus_for_space => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn url_decode(text: &str, plus_for_space: bool) -> Result<Vec<u8>, Error> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let pair = bytes
                    .get(i + 1..i + 3)
                    .ok_or(Error::InvalidPercentEncoding)?;
                let pair = std::str::from_utf8(pair).map_err(|_| Error::InvalidPercentEncoding)?;
                out.push(u8::from_str_radix(pair, 16).map_err(|_| Error::InvalidPercentEncoding)?);
                i += 3;
            }
            b'+' if plus_for_space => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    Ok(out)
}

fn html_encode(text: &str, encode_non_ascii: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if encode_non_ascii && !c.is_ascii() => {
                out.push_str(&format!("&#x{:X};", u32::from(c)))
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options::default()
    }

    #[test]
    fn base64_round_trips_and_is_lenient() {
        let text = "Hello, wörld? ~>";
        let standard = encode(Codec::Base64, text, &options());
        assert_eq!(standard, "SGVsbG8sIHfDtnJsZD8gfj4=");
        let url_safe = encode(
            Codec::Base64,
            text,
            &Options {
                base64_url_safe: true,
                base64_padding: false,
                ..options()
            },
        );
        assert_eq!(url_safe, "SGVsbG8sIHfDtnJsZD8gfj4");
        for encoded in [
            standard.as_str(),
            url_safe.as_str(),
            "SGVsbG8sIHfD\n  tnJsZD8gfj4=",
        ] {
            assert_eq!(decode(Codec::Base64, encoded, &options()).unwrap(), text);
        }
        assert_eq!(
            decode(Codec::Base64, "not base64!", &options()),
            Err(Error::InvalidBase64)
        );
    }

    #[test]
    fn base64_binary_is_reported_as_hex() {
        let error = decode(Codec::Base64, "/w==", &options()).unwrap_err();
        assert_eq!(
            error,
            Error::NotText {
                hex: "ff".into(),
                len: 1
            }
        );
    }

    #[test]
    fn url_encoding() {
        let text = "a b&c=d/é?~";
        assert_eq!(
            encode(Codec::Url, text, &options()),
            "a%20b%26c%3Dd%2F%C3%A9%3F~"
        );
        let form = Options {
            url_plus_for_space: true,
            ..options()
        };
        assert_eq!(encode(Codec::Url, text, &form), "a+b%26c%3Dd%2F%C3%A9%3F~");
        assert_eq!(
            decode(Codec::Url, "a+b%26c%3dd%2F%C3%A9", &form).unwrap(),
            "a b&c=d/é"
        );
        assert_eq!(decode(Codec::Url, "a+b", &options()).unwrap(), "a+b");
        assert_eq!(
            decode(Codec::Url, "100%", &options()),
            Err(Error::InvalidPercentEncoding)
        );
        assert_eq!(
            decode(Codec::Url, "%zz", &options()),
            Err(Error::InvalidPercentEncoding)
        );
    }

    #[test]
    fn html_entities() {
        assert_eq!(
            encode(Codec::Html, "<a href=\"x\">Tom & Jerry's</a>", &options()),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;"
        );
        assert_eq!(
            encode(
                Codec::Html,
                "café ✓",
                &Options {
                    html_encode_non_ascii: true,
                    ..options()
                }
            ),
            "caf&#xE9; &#x2713;"
        );
        assert_eq!(
            decode(
                Codec::Html,
                "&lt;p&gt;caf&eacute; &#x2713; &amp;amp;&nbsp;",
                &options()
            )
            .unwrap(),
            "<p>café ✓ &amp;\u{a0}"
        );
    }

    #[test]
    fn hex() {
        assert_eq!(encode(Codec::Hex, "Hi!", &options()), "486921");
        assert_eq!(decode(Codec::Hex, "0x48 69:21", &options()).unwrap(), "Hi!");
        assert_eq!(
            decode(Codec::Hex, "486", &options()),
            Err(Error::InvalidHex)
        );
        assert_eq!(decode(Codec::Hex, "zz", &options()), Err(Error::InvalidHex));
    }
}
