//! Image conversion: re-encoding to another format, with resizing and quality control.
//! Re-encoding drops EXIF and other metadata but keeps the color profile.

use super::{Error, Loaded};
use image::codecs::avif::AvifEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageEncoder, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// ICO files hold images up to 256×256.
const ICO_MAX: u32 = 256;
/// The largest output, in pixels, so a huge resize can't exhaust memory. The same as the
/// image crate's default decoding limit: 128 megapixels is 512 MB as RGBA.
const MAX_OUTPUT_PIXELS: u64 = 128 * 1024 * 1024;
/// The largest output side, which is also the most JPEG and WebP can store.
const MAX_OUTPUT_SIDE: u32 = 16_383;
/// AVIF encoder speed, 1 (slowest, smallest) to 10 (fastest).
const AVIF_SPEED: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

impl OutputFormat {
    pub const ALL: [OutputFormat; 8] = [
        OutputFormat::Png,
        OutputFormat::Jpeg,
        OutputFormat::Webp,
        OutputFormat::Avif,
        OutputFormat::Gif,
        OutputFormat::Bmp,
        OutputFormat::Tiff,
        OutputFormat::Ico,
    ];

    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Png => "PNG",
            OutputFormat::Jpeg => "JPEG",
            OutputFormat::Webp => "WebP (lossless)",
            OutputFormat::Avif => "AVIF",
            OutputFormat::Gif => "GIF",
            OutputFormat::Bmp => "BMP",
            OutputFormat::Tiff => "TIFF",
            OutputFormat::Ico => "ICO (up to 256 px)",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Webp => "webp",
            OutputFormat::Avif => "avif",
            OutputFormat::Gif => "gif",
            OutputFormat::Bmp => "bmp",
            OutputFormat::Tiff => "tiff",
            OutputFormat::Ico => "ico",
        }
    }

    /// Whether the quality setting applies (lossy formats).
    pub fn has_quality(self) -> bool {
        matches!(self, OutputFormat::Jpeg | OutputFormat::Avif)
    }

    /// Whether transparency survives; JPEG is flattened onto the background color.
    pub fn has_alpha(self) -> bool {
        self != OutputFormat::Jpeg
    }

    /// A caveat to show next to the format, if any.
    pub fn note(self) -> Option<&'static str> {
        match self {
            OutputFormat::Webp => Some(
                "Saved lossless. Lossy WebP needs a C library, so use AVIF or JPEG for smaller photos.",
            ),
            OutputFormat::Avif => Some("Best compression for photos. Slow to encode large images."),
            OutputFormat::Gif => Some("GIF has only 256 colors; photos will look banded."),
            OutputFormat::Jpeg => {
                Some("No transparency: transparent areas are filled with the background color.")
            }
            OutputFormat::Ico => Some("Larger images are scaled down to fit 256×256."),
            _ => None,
        }
    }
}

/// How to resize before encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resize {
    Original,
    /// Fit within this box, keeping the aspect ratio. 0 means no limit on that side.
    Fit {
        width: u32,
        height: u32,
    },
    /// Scale by a percentage.
    Scale(u32),
    /// Exactly this size, stretching if the aspect ratio differs.
    Exact {
        width: u32,
        height: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub format: OutputFormat,
    /// 1–100, for JPEG and AVIF.
    pub quality: u8,
    pub resize: Resize,
    /// Whether `Fit` and `Scale` may make an image larger.
    pub allow_upscale: bool,
    /// What transparent areas become in JPEG.
    pub background: [u8; 3],
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: OutputFormat::Jpeg,
            quality: 85,
            resize: Resize::Original,
            allow_upscale: false,
            background: [255, 255, 255],
        }
    }
}

/// The size `resize` produces for a `width`×`height` image.
pub fn target_size(width: u32, height: u32, resize: Resize, allow_upscale: bool) -> (u32, u32) {
    let (w, h) = (f64::from(width), f64::from(height));
    let (new_w, new_h) = match resize {
        Resize::Original => return (width, height),
        Resize::Exact { width, height } => return (width.max(1), height.max(1)),
        Resize::Scale(percent) => {
            let factor = f64::from(percent) / 100.0;
            let factor = if allow_upscale {
                factor
            } else {
                factor.min(1.0)
            };
            (w * factor, h * factor)
        }
        Resize::Fit {
            width: max_w,
            height: max_h,
        } => {
            let limit = |max: u32, size: f64| {
                if max == 0 {
                    f64::INFINITY
                } else {
                    f64::from(max) / size
                }
            };
            let factor = limit(max_w, w).min(limit(max_h, h));
            let factor = if factor.is_infinite() {
                1.0
            } else if allow_upscale {
                factor
            } else {
                factor.min(1.0)
            };
            (w * factor, h * factor)
        }
    };
    ((new_w.round() as u32).max(1), (new_h.round() as u32).max(1))
}

pub fn resize(image: &DynamicImage, resize: Resize, allow_upscale: bool) -> DynamicImage {
    let (width, height) = target_size(image.width(), image.height(), resize, allow_upscale);
    if (width, height) == (image.width(), image.height()) {
        image.clone()
    } else {
        image.resize_exact(width, height, FilterType::Lanczos3)
    }
}

/// Flattens transparency onto `background`, for formats without alpha.
fn flatten(image: &DynamicImage, background: [u8; 3]) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let alpha = f32::from(a) / 255.0;
        let mix = |front: u8, back: u8| {
            (f32::from(front) * alpha + f32::from(back) * (1.0 - alpha)).round() as u8
        };
        Rgb([
            mix(r, background[0]),
            mix(g, background[1]),
            mix(b, background[2]),
        ])
    })
}

/// Re-encodes a loaded image with `options`, returning the new file's bytes.
pub fn encode(loaded: &Loaded, options: &Options) -> Result<Vec<u8>, Error> {
    let (width, height) = target_size(
        loaded.image.width(),
        loaded.image.height(),
        options.resize,
        options.allow_upscale,
    );
    if width > MAX_OUTPUT_SIDE
        || height > MAX_OUTPUT_SIDE
        || u64::from(width) * u64::from(height) > MAX_OUTPUT_PIXELS
    {
        return Err(Error(format!(
            "{width}×{height} is too large: the output can be at most {MAX_OUTPUT_SIDE} pixels on a side and {} megapixels",
            MAX_OUTPUT_PIXELS / (1024 * 1024)
        )));
    }
    let mut image = resize(&loaded.image, options.resize, options.allow_upscale);
    if options.format == OutputFormat::Ico && (image.width() > ICO_MAX || image.height() > ICO_MAX)
    {
        image = resize(
            &image,
            Resize::Fit {
                width: ICO_MAX,
                height: ICO_MAX,
            },
            false,
        );
    }
    let quality = options.quality.clamp(1, 100);
    let mut out = Cursor::new(Vec::new());
    let profile = loaded.icc_profile.as_deref();
    match options.format {
        OutputFormat::Png => {
            let encoder =
                PngEncoder::new_with_quality(&mut out, CompressionType::Best, PngFilter::Adaptive);
            write_with_profile(encoder, &image, profile)?;
        }
        OutputFormat::Jpeg => {
            let flat = DynamicImage::ImageRgb8(flatten(&image, options.background));
            write_with_profile(
                JpegEncoder::new_with_quality(&mut out, quality),
                &flat,
                profile,
            )?;
        }
        OutputFormat::Webp => {
            let image = if image.color().has_alpha() {
                DynamicImage::ImageRgba8(image.to_rgba8())
            } else {
                DynamicImage::ImageRgb8(image.to_rgb8())
            };
            write_with_profile(WebPEncoder::new_lossless(&mut out), &image, profile)?;
        }
        OutputFormat::Avif => {
            let encoder = AvifEncoder::new_with_speed_quality(&mut out, AVIF_SPEED, quality);
            write_with_profile(
                encoder,
                &DynamicImage::ImageRgba8(image.to_rgba8()),
                profile,
            )?;
        }
        OutputFormat::Gif => {
            DynamicImage::ImageRgba8(image.to_rgba8()).write_to(&mut out, ImageFormat::Gif)?
        }
        OutputFormat::Bmp => {
            DynamicImage::ImageRgba8(image.to_rgba8()).write_to(&mut out, ImageFormat::Bmp)?
        }
        OutputFormat::Tiff => image.write_to(&mut out, ImageFormat::Tiff)?,
        OutputFormat::Ico => {
            DynamicImage::ImageRgba8(image.to_rgba8()).write_to(&mut out, ImageFormat::Ico)?
        }
    }
    Ok(out.into_inner())
}

/// Encodes with the color profile embedded; encoders that can't embed one just skip it.
fn write_with_profile<E: ImageEncoder>(
    mut encoder: E,
    image: &DynamicImage,
    profile: Option<&[u8]>,
) -> Result<(), Error> {
    if let Some(profile) = profile {
        let _ = encoder.set_icc_profile(profile.to_vec());
    }
    image.write_with_encoder(encoder).map_err(Error::from)
}

/// `folder/<name>.<ext>`, or `<name>-2.<ext>`, `<name>-3.<ext>`, … if taken. Never the input.
pub fn output_path(input: &Path, folder: &Path, format: OutputFormat) -> PathBuf {
    let stem = input
        .file_stem()
        .map_or_else(|| "image".into(), |s| s.to_string_lossy().into_owned());
    let extension = format.extension();
    let mut candidate = folder.join(format!("{stem}.{extension}"));
    let mut n = 2;
    while candidate.exists() || same_file(&candidate, input) {
        candidate = folder.join(format!("{stem}-{n}.{extension}"));
        n += 1;
    }
    candidate
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::{decode, mime_type, test_images};

    fn loaded() -> Loaded {
        decode(&test_images::encoded(ImageFormat::Png)).unwrap()
    }

    fn options(format: OutputFormat) -> Options {
        Options {
            format,
            ..Options::default()
        }
    }

    #[test]
    fn every_format_encodes() {
        for format in OutputFormat::ALL {
            let bytes = encode(&loaded(), &options(format)).unwrap();
            if format == OutputFormat::Avif {
                // AVIF can be written but not read back without a C decoder.
                assert_eq!(mime_type(&bytes), Some("image/avif"));
                continue;
            }
            let back = decode(&bytes).unwrap_or_else(|e| panic!("{format:?}: {e}"));
            assert_eq!(
                (back.image.width(), back.image.height()),
                (40, 30),
                "{format:?}"
            );
        }
    }

    #[test]
    fn jpeg_flattens_transparency_onto_background() {
        let black = Options {
            background: [0, 0, 0],
            ..options(OutputFormat::Jpeg)
        };
        let image = decode(&encode(&loaded(), &black).unwrap())
            .unwrap()
            .image
            .to_rgb8();
        // The right half is blue at 50% alpha: on black that is about (0, 0, 128).
        let [r, g, b] = image.get_pixel(35, 15).0;
        assert!(r < 20 && g < 20 && (110..150).contains(&b), "{r} {g} {b}");
    }

    #[test]
    fn transparency_survives_in_png_and_webp() {
        for format in [OutputFormat::Png, OutputFormat::Webp] {
            let image = decode(&encode(&loaded(), &options(format)).unwrap())
                .unwrap()
                .image
                .to_rgba8();
            assert_eq!(image.get_pixel(35, 15).0[3], 128, "{format:?}");
        }
    }

    #[test]
    fn quality_changes_size() {
        let photo = Loaded {
            image: DynamicImage::ImageRgb8(RgbImage::from_fn(200, 200, |x, y| {
                Rgb([
                    (x * 7 % 256) as u8,
                    (y * 3 % 256) as u8,
                    ((x + y) % 256) as u8,
                ])
            })),
            icc_profile: None,
            format: None,
            file_size: 0,
        };
        let small = encode(
            &photo,
            &Options {
                quality: 20,
                ..options(OutputFormat::Jpeg)
            },
        )
        .unwrap();
        let large = encode(
            &photo,
            &Options {
                quality: 95,
                ..options(OutputFormat::Jpeg)
            },
        )
        .unwrap();
        assert!(
            small.len() * 2 < large.len(),
            "{} vs {}",
            small.len(),
            large.len()
        );
    }

    #[test]
    fn keeps_the_color_profile() {
        let profile = b"fake ICC profile for the round trip".to_vec();
        let with_profile = Loaded {
            icc_profile: Some(profile.clone()),
            ..loaded()
        };
        let back = decode(&encode(&with_profile, &options(OutputFormat::Png)).unwrap()).unwrap();
        assert_eq!(back.icc_profile, Some(profile));
    }

    #[test]
    fn computes_target_sizes() {
        assert_eq!(
            target_size(
                4000,
                3000,
                Resize::Fit {
                    width: 1920,
                    height: 1080
                },
                false
            ),
            (1440, 1080)
        );
        assert_eq!(
            target_size(
                4000,
                3000,
                Resize::Fit {
                    width: 1000,
                    height: 0
                },
                false
            ),
            (1000, 750)
        );
        assert_eq!(
            target_size(
                400,
                300,
                Resize::Fit {
                    width: 1000,
                    height: 1000
                },
                false
            ),
            (400, 300)
        );
        assert_eq!(
            target_size(
                400,
                300,
                Resize::Fit {
                    width: 1000,
                    height: 1000
                },
                true
            ),
            (1000, 750)
        );
        assert_eq!(
            target_size(
                400,
                300,
                Resize::Fit {
                    width: 0,
                    height: 0
                },
                true
            ),
            (400, 300)
        );
        assert_eq!(target_size(400, 300, Resize::Scale(50), false), (200, 150));
        assert_eq!(target_size(400, 300, Resize::Scale(200), false), (400, 300));
        assert_eq!(
            target_size(
                400,
                300,
                Resize::Exact {
                    width: 64,
                    height: 64
                },
                false
            ),
            (64, 64)
        );
        assert_eq!(target_size(3, 3, Resize::Scale(1), false), (1, 1));
        let resized = resize(&loaded().image, Resize::Scale(50), false);
        assert_eq!((resized.width(), resized.height()), (20, 15));
    }

    #[test]
    fn huge_resizes_are_errors() {
        let huge = [
            Resize::Exact {
                width: u32::MAX,
                height: u32::MAX,
            },
            Resize::Exact {
                width: 100_000,
                height: 100_000,
            },
            Resize::Fit {
                width: u32::MAX,
                height: 0,
            },
            Resize::Scale(u32::MAX),
        ];
        for resize in huge {
            let options = Options {
                resize,
                allow_upscale: true,
                ..options(OutputFormat::Png)
            };
            assert!(encode(&loaded(), &options).is_err(), "{resize:?}");
        }
        // Without upscaling, the same requests keep the original size.
        let options = Options {
            resize: Resize::Scale(u32::MAX),
            ..options(OutputFormat::Png)
        };
        assert!(encode(&loaded(), &options).is_ok());
    }

    #[test]
    fn ico_is_limited_to_256() {
        let big = Loaded {
            image: DynamicImage::new_rgba8(600, 300),
            ..loaded()
        };
        let back = decode(&encode(&big, &options(OutputFormat::Ico)).unwrap()).unwrap();
        assert_eq!((back.image.width(), back.image.height()), (256, 128));
    }

    #[test]
    fn output_paths_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("toolbox-convert-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("photo.png");
        std::fs::write(&input, b"x").unwrap();
        assert_eq!(
            output_path(&input, &dir, OutputFormat::Jpeg),
            dir.join("photo.jpg")
        );
        // Converting PNG to PNG in the same folder must not target the original.
        assert_eq!(
            output_path(&input, &dir, OutputFormat::Png),
            dir.join("photo-2.png")
        );
        std::fs::write(dir.join("photo.jpg"), b"x").unwrap();
        assert_eq!(
            output_path(&input, &dir, OutputFormat::Jpeg),
            dir.join("photo-2.jpg")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
