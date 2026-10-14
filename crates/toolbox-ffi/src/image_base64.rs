//! Base64 Images: images as `data:` URIs for HTML, CSS and Markdown, and back.

use crate::ToolError;
use std::path::Path;
use toolbox_core::images::{self, data_uri};

/// A ready-to-paste use of a data URI, e.g. label "CSS".
#[derive(uniffi::Record)]
pub struct ImageBase64Snippet {
    pub label: String,
    pub value: String,
}

#[derive(uniffi::Record)]
pub struct ImageBase64Encoded {
    /// `data:<mime>;base64,<data>`.
    pub uri: String,
    pub file_size: u64,
    /// CSS, HTML and Markdown using the URI.
    pub snippets: Vec<ImageBase64Snippet>,
}

/// Reads an image file as a data URI, with snippets using it.
#[uniffi::export]
pub fn image_base64_encode_file(path: &str) -> Result<ImageBase64Encoded, ToolError> {
    let bytes = std::fs::read(Path::new(path))
        .map_err(|error| ToolError::new(format!("Couldn't open: {error}")))?;
    let uri = data_uri::encode(&bytes);
    let snippets = data_uri::snippets(&uri)
        .into_iter()
        .map(|(label, value)| ImageBase64Snippet {
            label: label.into(),
            value,
        })
        .collect();
    Ok(ImageBase64Encoded {
        uri,
        file_size: bytes.len() as u64,
        snippets,
    })
}

#[derive(uniffi::Record)]
pub struct ImageBase64Decoded {
    pub mime: String,
    pub bytes: Vec<u8>,
    /// The usual file extension for the type, e.g. "png".
    pub extension: String,
    /// `None` if the image couldn't be decoded to measure it. SVGs are measured as rendered
    /// (at least 1024 px on the longer side).
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Decodes a data URI (also inside CSS `url(…)` or `<img src=…>`) or plain Base64 into image
/// bytes. Slow for large images: call it off the main thread.
#[uniffi::export]
pub fn image_base64_decode(text: &str) -> Result<ImageBase64Decoded, ToolError> {
    let (mime, bytes) = data_uri::decode(text).map_err(ToolError::new)?;
    let dimensions = images::decode(&bytes)
        .ok()
        .map(|loaded| (loaded.image.width(), loaded.image.height()));
    Ok(ImageBase64Decoded {
        extension: images::extension_for_mime(&mime).into(),
        mime,
        bytes,
        width: dimensions.map(|(width, _)| width),
        height: dimensions.map(|(_, height)| height),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12"/>"##;

    #[test]
    fn round_trips_a_file() {
        let path =
            std::env::temp_dir().join(format!("toolbox-ffi-base64-{}.svg", std::process::id()));
        std::fs::write(&path, SVG).unwrap();
        let encoded = image_base64_encode_file(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(encoded.uri.starts_with("data:image/svg+xml;base64,"));
        assert_eq!(encoded.file_size, SVG.len() as u64);
        let labels: Vec<&str> = encoded.snippets.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["CSS", "HTML", "Markdown"]);

        let decoded = image_base64_decode(&encoded.snippets[1].value).unwrap();
        assert_eq!(decoded.mime, "image/svg+xml");
        assert_eq!(decoded.extension, "svg");
        assert_eq!(decoded.bytes, SVG.as_bytes());
        assert_eq!((decoded.width, decoded.height), (Some(1024), Some(512)));
    }

    #[test]
    fn rejects_non_images() {
        assert!(image_base64_decode("data:text/plain;base64,aGVsbG8=").is_err());
        assert!(image_base64_encode_file("/nonexistent.png").is_err());
    }
}
