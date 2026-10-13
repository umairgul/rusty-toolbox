//! Images as `data:` URIs, for embedding in HTML, CSS and Markdown, and back.

use super::{Error, mime_type};
use crate::encoding::decode_base64;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// `data:<mime>;base64,<data>`, with the MIME type detected from the bytes.
pub fn encode(bytes: &[u8]) -> String {
    let mime = mime_type(bytes).unwrap_or("application/octet-stream");
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

/// Ready-to-paste snippets using a data URI: CSS, an HTML `<img>` and Markdown.
pub fn snippets(uri: &str) -> [(&'static str, String); 3] {
    [
        ("CSS", format!("background-image: url(\"{uri}\");")),
        ("HTML", format!("<img src=\"{uri}\" alt=\"\">")),
        ("Markdown", format!("![]({uri})")),
    ]
}

/// Decodes a data URI (also inside `url(…)` or `src="…"`) or plain Base64 into image bytes
/// and their MIME type.
pub fn decode(text: &str) -> Result<(String, Vec<u8>), Error> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Error("Paste a data URI or Base64".into()));
    }
    let (declared, bytes) = match text.find("data:") {
        Some(start) => {
            let uri = &text[start + 5..];
            // Stop at the quote or bracket that closes a CSS url() or HTML attribute.
            let uri = uri.split(['"', '\'', ')', ' ', '\n']).next().unwrap_or(uri);
            let (header, payload) = uri
                .split_once(',')
                .ok_or_else(|| Error("A data URI needs a comma before the data".into()))?;
            let mime = header
                .split(';')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let bytes = if header.to_ascii_lowercase().ends_with(";base64") {
                decode_base64(payload).map_err(|_| Error("The data isn't valid Base64".into()))?
            } else {
                percent_decode(payload)
            };
            (Some(mime).filter(|m| !m.is_empty()), bytes)
        }
        None => (
            None,
            decode_base64(text).map_err(|_| Error("Not a data URI or valid Base64".into()))?,
        ),
    };
    let mime = mime_type(&bytes)
        .map(str::to_owned)
        .or(declared.filter(|m| m.starts_with("image/")));
    match mime {
        Some(mime) => Ok((mime, bytes)),
        None => Err(Error("The decoded data isn't a recognized image".into())),
    }
}

/// Undoes `%XX` escapes (invalid ones are kept as-is), as in `data:image/svg+xml,%3Csvg…`.
fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::test_images;
    use image::ImageFormat;

    #[test]
    fn round_trips() {
        let png = test_images::encoded(ImageFormat::Png);
        let uri = encode(&png);
        assert!(
            uri.starts_with("data:image/png;base64,iVBORw0KGgo"),
            "{}",
            &uri[..40]
        );
        assert_eq!(decode(&uri).unwrap(), ("image/png".into(), png.clone()));
        // Inside CSS or HTML, with whitespace, or as bare Base64.
        let [(_, css), (_, html), (_, markdown)] = snippets(&uri);
        for wrapped in [css, html, markdown, format!("  {uri}\n")] {
            assert_eq!(decode(&wrapped).unwrap().1, png);
        }
        let bare = uri.split_once(',').unwrap().1;
        assert_eq!(decode(bare).unwrap().0, "image/png");
    }

    #[test]
    fn decodes_percent_encoded_svg() {
        let (mime, bytes) =
            decode("data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'/%3E").unwrap();
        assert_eq!(mime, "image/svg+xml");
        assert!(bytes.starts_with(b"<svg"));
    }

    #[test]
    fn rejects_non_images() {
        assert!(decode("").is_err());
        assert!(decode("data:text/plain;base64,aGVsbG8=").is_err());
        assert!(decode("data:image/png;base64").is_err());
        assert!(decode("!!!").is_err());
    }
}
