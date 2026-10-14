//! Image tools: loading any common image, conversion, metadata removal, icon sets and data
//! URIs.

pub mod convert;
pub mod data_uri;
pub mod icons;
pub mod metadata;

use image::{DynamicImage, ImageDecoder, ImageReader};
use std::fmt;
use std::io::Cursor;
use std::path::Path;

/// File extensions the tools accept, for open dialogs.
pub const INPUT_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "ico", "tif", "tiff", "heic", "heif", "svg",
];

/// SVGs have no pixel size of their own; they are rendered with this many pixels on the
/// longer side, or their stated size if larger (up to 8192).
pub const SVG_MIN_SIZE: u32 = 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<image::ImageError> for Error {
    fn from(error: image::ImageError) -> Self {
        Error(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error(error.to_string())
    }
}

/// A decoded image, upright, with the color profile it was saved with.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub image: DynamicImage,
    /// ICC color profile. Keeping it when re-encoding keeps colors right, e.g. for Display P3
    /// photos from iPhones.
    pub icc_profile: Option<Vec<u8>>,
    /// The detected format, or `None` for HEIC (decoded by the system).
    pub format: Option<image::ImageFormat>,
    pub file_size: u64,
}

/// What an image file is, from its header, without decoding the pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    pub mime: Option<&'static str>,
    /// `None` for formats sized only when decoded (SVG, HEIC).
    pub dimensions: Option<(u32, u32)>,
    pub file_size: u64,
}

pub fn probe(path: &Path) -> Result<Probe, Error> {
    use std::io::Read;
    let file_size = std::fs::metadata(path)?.len();
    let mut head = vec![0; 1024];
    let read = std::fs::File::open(path)?.read(&mut head)?;
    head.truncate(read);
    let mime = mime_type(&head);
    let dimensions = match mime {
        Some("image/svg+xml" | "image/heic") | None => None,
        Some(_) => ImageReader::open(path)?
            .with_guessed_format()?
            .into_dimensions()
            .ok(),
    };
    Ok(Probe {
        mime,
        dimensions,
        file_size,
    })
}

/// Loads an image file, applying its EXIF orientation. HEIC/HEIF photos are decoded with
/// macOS's `sips`, since no pure-Rust HEIC decoder exists.
pub fn load(path: &Path) -> Result<Loaded, Error> {
    let bytes = std::fs::read(path)?;
    let file_size = bytes.len() as u64;
    if is_heif(&bytes) {
        let mut loaded = decode_heif(path)?;
        loaded.file_size = file_size;
        return Ok(loaded);
    }
    let mut loaded = decode(&bytes)?;
    loaded.file_size = file_size;
    Ok(loaded)
}

/// Decodes image bytes in any supported format (not HEIC), applying EXIF orientation.
pub fn decode(bytes: &[u8]) -> Result<Loaded, Error> {
    if is_svg(bytes) {
        return render_svg(bytes);
    }
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader.format();
    if format.is_none() {
        return Err(Error("Not a supported image format".into()));
    }
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let icc_profile = decoder.icc_profile()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(Loaded {
        image,
        icc_profile,
        format,
        file_size: bytes.len() as u64,
    })
}

/// usvg rejects SVGs nested deeper than this, but only after the XML parser has recursed
/// through all of it.
const SVG_MAX_DEPTH: usize = 1024;

fn render_svg(bytes: &[u8]) -> Result<Loaded, Error> {
    if xml_depth(bytes) > SVG_MAX_DEPTH {
        return Err(Error("Invalid SVG: elements are nested too deeply".into()));
    }
    // usvg and resvg recurse per nesting level, deeper than a small thread stack allows.
    crate::stack::with_large_stack(|| render_svg_on_this_thread(bytes))
}

/// The deepest element nesting in an XML document, roughly: tags inside comments and quoted
/// attribute values count too, which can only overestimate.
fn xml_depth(bytes: &[u8]) -> usize {
    let (mut depth, mut deepest) = (0usize, 0);
    for (i, &byte) in bytes.iter().enumerate() {
        match (byte, bytes.get(i + 1)) {
            (b'<', Some(b'/')) => depth = depth.saturating_sub(1),
            (b'<', Some(b'?' | b'!')) => {}
            (b'<', _) => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            (b'/', Some(b'>')) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

fn render_svg_on_this_thread(bytes: &[u8]) -> Result<Loaded, Error> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|e| Error(format!("Invalid SVG: {e}")))?;
    let size = tree.size();
    let longer = size.width().max(size.height());
    let target = longer.max(SVG_MIN_SIZE as f32).min(8192.0);
    let scale = target / longer;
    let (width, height) = (
        (size.width() * scale).round().max(1.0) as u32,
        (size.height() * scale).round().max(1.0) as u32,
    );
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| Error("SVG is too large".into()))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied alpha; image wants straight alpha.
    let pixels = pixmap.pixels().iter().flat_map(|p| {
        let c = p.demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    });
    let image = image::RgbaImage::from_raw(width, height, pixels.collect())
        .ok_or_else(|| Error("SVG render failed".into()))?;
    Ok(Loaded {
        image: DynamicImage::ImageRgba8(image),
        icc_profile: None,
        format: None,
        file_size: bytes.len() as u64,
    })
}

/// HEIF/HEIC files are ISO media files whose `ftyp` brand names an image codec.
pub fn is_heif(bytes: &[u8]) -> bool {
    bytes.len() > 12
        && &bytes[4..8] == b"ftyp"
        && matches!(
            &bytes[8..12],
            b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"mif1" | b"msf1"
        )
}

#[cfg(target_os = "macos")]
fn decode_heif(path: &Path) -> Result<Loaded, Error> {
    let output = std::env::temp_dir().join(format!(
        "toolbox-heif-{}-{}.png",
        std::process::id(),
        unique()
    ));
    let status = std::process::Command::new("sips")
        .args(["-s", "format", "png"])
        .arg(path)
        .arg("--out")
        .arg(&output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    let result = if status.success() {
        std::fs::read(&output)
            .map_err(Error::from)
            .and_then(|png| decode(&png))
    } else {
        Err(Error("macOS couldn't decode this HEIC image".into()))
    };
    let _ = std::fs::remove_file(&output);
    let mut loaded = result?;
    loaded.format = None;
    Ok(loaded)
}

#[cfg(not(target_os = "macos"))]
fn decode_heif(_path: &Path) -> Result<Loaded, Error> {
    Err(Error("HEIC images can only be opened on macOS".into()))
}

/// A number that differs between calls, for temporary file names.
#[cfg(target_os = "macos")]
fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// The MIME type of image bytes, from their signature.
pub fn mime_type(bytes: &[u8]) -> Option<&'static str> {
    let starts = |prefix: &[u8]| bytes.starts_with(prefix);
    if starts(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if starts(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if starts(b"GIF87a") || starts(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() > 12 && starts(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.len() > 12
        && &bytes[4..8] == b"ftyp"
        && matches!(&bytes[8..12], b"avif" | b"avis")
    {
        Some("image/avif")
    } else if is_heif(bytes) {
        Some("image/heic")
    } else if starts(b"BM") {
        Some("image/bmp")
    } else if starts(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else if starts(b"II*\0") || starts(b"MM\0*") {
        Some("image/tiff")
    } else if is_svg(bytes) {
        Some("image/svg+xml")
    } else {
        None
    }
}

fn is_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let head = head.trim_start_matches('\u{feff}').trim_start();
    (head.starts_with("<?xml") || head.starts_with("<svg") || head.starts_with("<!--"))
        && head.contains("<svg")
}

/// The usual file extension for a MIME type.
pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/heic" => "heic",
        "image/bmp" => "bmp",
        "image/x-icon" => "ico",
        "image/tiff" => "tiff",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

#[cfg(test)]
pub(crate) mod test_images {
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    /// A 40×30 image: a red left half and a translucent blue right half.
    pub fn sample() -> RgbaImage {
        RgbaImage::from_fn(40, 30, |x, _| {
            if x < 20 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 128])
            }
        })
    }

    pub fn encoded(format: ImageFormat) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgba8(sample());
        let image = if format == ImageFormat::Jpeg {
            image::DynamicImage::ImageRgb8(image.to_rgb8())
        } else {
            image
        };
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, format).unwrap();
        out.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageFormat;

    #[test]
    fn deeply_nested_svg_is_an_error_not_a_crash() {
        let svg = |depth: usize, element: &str| {
            format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8'>{}<rect width='4' height='4'/>{}</svg>",
                format!("<{element}>").repeat(depth),
                format!("</{element}>").repeat(depth)
            )
        };
        // Swift runs work on threads with 512 KB stacks.
        let decode_small_stack = |svg: String| {
            std::thread::Builder::new()
                .stack_size(512 * 1024)
                .spawn(move || decode(svg.as_bytes()).map(|loaded| loaded.image.width()))
                .unwrap()
                .join()
                .unwrap()
        };
        assert!(decode_small_stack(svg(100_000, "a")).is_err());
        assert!(decode_small_stack(svg(1_000, "svg")).is_ok());
        assert!(decode_small_stack(svg(1_000, "g")).is_ok());
    }

    #[test]
    fn detects_mime_types() {
        assert_eq!(
            mime_type(&test_images::encoded(ImageFormat::Png)),
            Some("image/png")
        );
        assert_eq!(
            mime_type(&test_images::encoded(ImageFormat::Jpeg)),
            Some("image/jpeg")
        );
        assert_eq!(
            mime_type(&test_images::encoded(ImageFormat::WebP)),
            Some("image/webp")
        );
        assert_eq!(
            mime_type(&test_images::encoded(ImageFormat::Gif)),
            Some("image/gif")
        );
        assert_eq!(
            mime_type(b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            Some("image/svg+xml")
        );
        assert_eq!(mime_type(b"hello"), None);
        assert_eq!(extension_for_mime("image/jpeg"), "jpg");
    }

    #[test]
    fn decodes_and_reports_format() {
        let loaded = decode(&test_images::encoded(ImageFormat::Png)).unwrap();
        assert_eq!((loaded.image.width(), loaded.image.height()), (40, 30));
        assert_eq!(loaded.format, Some(ImageFormat::Png));
        assert!(decode(b"not an image").is_err());
    }

    #[test]
    fn probes_without_decoding() {
        let path = std::env::temp_dir().join(format!("toolbox-probe-{}.png", std::process::id()));
        std::fs::write(&path, test_images::encoded(ImageFormat::Png)).unwrap();
        let probe = probe(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(probe.mime, Some("image/png"));
        assert_eq!(probe.dimensions, Some((40, 30)));
        assert!(probe.file_size > 0);
    }

    #[test]
    fn renders_svg_at_a_useful_size() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12"><rect width="12" height="12" fill="#ff0000"/></svg>"##;
        let loaded = decode(svg).unwrap();
        assert_eq!((loaded.image.width(), loaded.image.height()), (1024, 512));
        let image = loaded.image.to_rgba8();
        assert_eq!(image.get_pixel(100, 100).0, [255, 0, 0, 255]);
        assert_eq!(
            image.get_pixel(900, 100).0[3],
            0,
            "outside the rect is transparent"
        );
        assert!(decode(b"<svg nope").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn opens_heic_through_sips() {
        let dir = std::env::temp_dir().join(format!("toolbox-heic-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("in.png");
        std::fs::write(&png, test_images::encoded(ImageFormat::Png)).unwrap();
        let heic = dir.join("in.heic");
        let made = std::process::Command::new("sips")
            .args(["-s", "format", "heic"])
            .arg(&png)
            .arg("--out")
            .arg(&heic)
            .output()
            .unwrap();
        assert!(made.status.success(), "sips couldn't make a HEIC test file");
        assert!(is_heif(&std::fs::read(&heic).unwrap()));
        let loaded = load(&heic).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!((loaded.image.width(), loaded.image.height()), (40, 30));
        assert_eq!(loaded.format, None);
    }
}
