//! Image helpers shared by the image tools. Paths are passed as strings.

use crate::ToolError;
use std::path::Path;
use toolbox_core::images;

/// What an image file is, read from its header without decoding the pixels.
#[derive(uniffi::Record)]
pub struct ImageProbe {
    pub mime: Option<String>,
    /// `None` for formats sized only when decoded (SVG, HEIC).
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: u64,
}

#[uniffi::export]
pub fn image_probe(path: &str) -> Result<ImageProbe, ToolError> {
    let probe = images::probe(Path::new(path)).map_err(ToolError::new)?;
    Ok(ImageProbe {
        mime: probe.mime.map(Into::into),
        width: probe.dimensions.map(|(width, _)| width),
        height: probe.dimensions.map(|(_, height)| height),
        file_size: probe.file_size,
    })
}

/// File extensions the image tools accept, for open panels and drops.
#[uniffi::export]
pub fn image_input_extensions() -> Vec<String> {
    images::INPUT_EXTENSIONS.iter().map(|&e| e.into()).collect()
}
