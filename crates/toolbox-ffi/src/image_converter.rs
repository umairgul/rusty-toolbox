use crate::ToolError;
use std::path::Path;
use toolbox_core::images::{self, convert};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ImageOutputFormat {
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

impl From<ImageOutputFormat> for convert::OutputFormat {
    fn from(format: ImageOutputFormat) -> Self {
        use convert::OutputFormat as Out;
        match format {
            ImageOutputFormat::Png => Out::Png,
            ImageOutputFormat::Jpeg => Out::Jpeg,
            ImageOutputFormat::Webp => Out::Webp,
            ImageOutputFormat::Avif => Out::Avif,
            ImageOutputFormat::Gif => Out::Gif,
            ImageOutputFormat::Bmp => Out::Bmp,
            ImageOutputFormat::Tiff => Out::Tiff,
            ImageOutputFormat::Ico => Out::Ico,
        }
    }
}

#[derive(uniffi::Record)]
pub struct ImageFormatInfo {
    pub label: String,
    pub extension: String,
    /// Whether the quality setting applies (lossy formats).
    pub has_quality: bool,
    /// Whether transparency survives; otherwise it's filled with the background color.
    pub has_alpha: bool,
    /// A caveat to show next to the format, if any.
    pub note: Option<String>,
}

#[uniffi::export]
pub fn image_format_info(format: ImageOutputFormat) -> ImageFormatInfo {
    let format = convert::OutputFormat::from(format);
    ImageFormatInfo {
        label: format.label().into(),
        extension: format.extension().into(),
        has_quality: format.has_quality(),
        has_alpha: format.has_alpha(),
        note: format.note().map(Into::into),
    }
}

#[derive(Clone, Copy, uniffi::Enum)]
pub enum ImageResize {
    Original,
    /// Fit within this box, keeping the aspect ratio. 0 means no limit on that side.
    Fit {
        width: u32,
        height: u32,
    },
    Scale {
        percent: u32,
    },
    /// Exactly this size, stretching if the aspect ratio differs.
    Exact {
        width: u32,
        height: u32,
    },
}

impl From<ImageResize> for convert::Resize {
    fn from(resize: ImageResize) -> Self {
        match resize {
            ImageResize::Original => convert::Resize::Original,
            ImageResize::Fit { width, height } => convert::Resize::Fit { width, height },
            ImageResize::Scale { percent } => convert::Resize::Scale(percent),
            ImageResize::Exact { width, height } => convert::Resize::Exact { width, height },
        }
    }
}

#[derive(uniffi::Record)]
pub struct ImageConvertOptions {
    pub format: ImageOutputFormat,
    /// 1–100, for JPEG and AVIF.
    pub quality: u8,
    pub resize: ImageResize,
    /// Whether `Fit` and `Scale` may make an image larger.
    pub allow_upscale: bool,
    /// Red, green and blue (0–255) that transparent areas become in formats without alpha.
    pub background: Vec<u8>,
}

#[derive(uniffi::Record)]
pub struct ConvertedImage {
    pub output_path: String,
    pub input_size: u64,
    pub output_size: u64,
}

/// Converts one image and saves it in `folder`, named after the input with the new extension
/// (adding `-2`, `-3`, … rather than overwriting anything). Slow for large images: call it off
/// the main thread.
#[uniffi::export]
pub fn image_convert_file(
    input: &str,
    folder: &str,
    options: ImageConvertOptions,
) -> Result<ConvertedImage, ToolError> {
    let [r, g, b] = match options.background[..] {
        [r, g, b, ..] => [r, g, b],
        _ => [255, 255, 255],
    };
    let options = convert::Options {
        format: options.format.into(),
        quality: options.quality.clamp(1, 100),
        resize: options.resize.into(),
        allow_upscale: options.allow_upscale,
        background: [r, g, b],
    };
    let input = Path::new(input);
    let loaded = images::load(input).map_err(ToolError::new)?;
    let bytes = convert::encode(&loaded, &options).map_err(ToolError::new)?;
    let output = convert::output_path(input, Path::new(folder), options.format);
    std::fs::write(&output, &bytes)
        .map_err(|error| ToolError::new(format!("Couldn't save: {error}")))?;
    Ok(ConvertedImage {
        output_path: output.to_string_lossy().into_owned(),
        input_size: loaded.file_size,
        output_size: bytes.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_next_to_folder_without_overwriting() {
        let icon = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/icon/toolbox-512.png"
        );
        let folder = std::env::temp_dir().join(format!("toolbox-ffi-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let options = || ImageConvertOptions {
            format: ImageOutputFormat::Jpeg,
            quality: 80,
            resize: ImageResize::Fit {
                width: 64,
                height: 0,
            },
            allow_upscale: false,
            background: vec![255, 255, 255],
        };
        let folder_path = folder.to_str().unwrap();
        let first = image_convert_file(icon, folder_path, options()).unwrap();
        let second = image_convert_file(icon, folder_path, options()).unwrap();
        assert!(first.output_path.ends_with("toolbox-512.jpg"));
        assert!(second.output_path.ends_with("toolbox-512-2.jpg"));
        assert!(first.output_size > 0 && first.output_size < first.input_size);
        let probe = crate::images::image_probe(&first.output_path).unwrap();
        assert_eq!((probe.width, probe.height), (Some(64), Some(64)));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn reports_unreadable_files() {
        let error = image_convert_file(
            "/nonexistent/image.png",
            "/tmp",
            ImageConvertOptions {
                format: ImageOutputFormat::Png,
                quality: 85,
                resize: ImageResize::Original,
                allow_upscale: false,
                background: vec![],
            },
        );
        assert!(error.is_err());
    }
}
