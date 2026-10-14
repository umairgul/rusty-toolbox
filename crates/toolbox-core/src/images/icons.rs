//! Icon sets from one image: favicons for websites, a macOS `.icns`, a Windows `.ico` and
//! plain PNGs at the usual sizes.

use super::Error;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, RgbaImage};

/// How a non-square image becomes square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// Add transparent space around the shorter side.
    Pad,
    /// Cut the longer side, keeping the center.
    Crop,
}

impl Fit {
    pub const ALL: [Fit; 2] = [Fit::Pad, Fit::Crop];

    pub fn label(self) -> &'static str {
        match self {
            Fit::Pad => "Pad with transparency",
            Fit::Crop => "Crop to center",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSet {
    Favicons,
    MacOs,
    Windows,
    PngSizes,
}

impl IconSet {
    pub const ALL: [IconSet; 4] = [
        IconSet::Favicons,
        IconSet::MacOs,
        IconSet::Windows,
        IconSet::PngSizes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            IconSet::Favicons => "Website favicons",
            IconSet::MacOs => "macOS app icon (.icns)",
            IconSet::Windows => "Windows icon (.ico)",
            IconSet::PngSizes => "PNG sizes (16–1024)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            IconSet::Favicons => {
                "favicon.ico, 16 and 32 px PNGs, apple-touch-icon, Android icons and site.webmanifest"
            }
            IconSet::MacOs => "AppIcon.icns with every size from 16 to 1024 px",
            IconSet::Windows => "app.ico with 16, 24, 32, 48, 64, 128 and 256 px",
            IconSet::PngSizes => "icon-16.png up to icon-1024.png",
        }
    }
}

/// A file to write, named relative to the output folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The largest side [`square`] works at: twice the largest icon.
const MAX_SQUARE: u32 = 2048;

const WINDOWS_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
const FAVICON_ICO_SIZES: [u32; 3] = [16, 32, 48];
const PNG_SIZES: [u32; 8] = [16, 32, 48, 64, 128, 256, 512, 1024];

/// `.icns` entry types and their pixel sizes. The @2x types hold double-resolution images.
const ICNS_ENTRIES: [(&[u8; 4], u32); 10] = [
    (b"icp4", 16),
    (b"ic11", 32), // 16@2x
    (b"icp5", 32),
    (b"ic12", 64), // 32@2x
    (b"ic07", 128),
    (b"ic13", 256), // 128@2x
    (b"ic08", 256),
    (b"ic14", 512), // 256@2x
    (b"ic09", 512),
    (b"ic10", 1024), // 512@2x
];

/// Link tags for the favicon set, to paste into a page's `<head>`.
pub const FAVICON_HTML: &str = r#"<link rel="icon" href="/favicon.ico" sizes="48x48">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">"#;

/// The image made square, as RGBA.
pub fn square(image: &DynamicImage, fit: Fit) -> RgbaImage {
    // Padding a long, thin image would make a huge canvas (1×60000 pixels pads to 60000²), so
    // shrink big images first. Icons are at most 1024 pixels, so this loses nothing.
    let shrunk;
    let image = if image.width().max(image.height()) > MAX_SQUARE {
        shrunk = image.resize(MAX_SQUARE, MAX_SQUARE, FilterType::Lanczos3);
        &shrunk
    } else {
        image
    };
    let (width, height) = (image.width(), image.height());
    match fit {
        Fit::Pad => {
            let side = width.max(height);
            let mut canvas = RgbaImage::new(side, side);
            image::imageops::overlay(
                &mut canvas,
                &image.to_rgba8(),
                i64::from((side - width) / 2),
                i64::from((side - height) / 2),
            );
            canvas
        }
        Fit::Crop => {
            let side = width.min(height);
            image
                .crop_imm((width - side) / 2, (height - side) / 2, side, side)
                .to_rgba8()
        }
    }
}

fn scaled(square: &RgbaImage, size: u32) -> RgbaImage {
    image::imageops::resize(square, size, size, FilterType::Lanczos3)
}

fn png(image: &RgbaImage) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )?;
    Ok(out)
}

/// A multi-size `.ico` with PNG-compressed entries.
pub fn ico(square: &RgbaImage, sizes: &[u32]) -> Result<Vec<u8>, Error> {
    let images: Vec<RgbaImage> = sizes.iter().map(|&size| scaled(square, size)).collect();
    let frames = images
        .iter()
        .map(|image| {
            IcoFrame::as_png(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgba8,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    IcoEncoder::new(&mut out).encode_images(&frames)?;
    Ok(out)
}

/// A macOS `.icns` with PNG entries for 16 to 1024 px, as `iconutil` writes them.
pub fn icns(square: &RgbaImage) -> Result<Vec<u8>, Error> {
    let mut entries = Vec::new();
    for (kind, size) in ICNS_ENTRIES {
        let data = png(&scaled(square, size))?;
        entries.extend_from_slice(kind);
        entries.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
        entries.extend(data);
    }
    let mut out = b"icns".to_vec();
    out.extend_from_slice(&((entries.len() + 8) as u32).to_be_bytes());
    out.extend(entries);
    Ok(out)
}

fn webmanifest(name: &str) -> String {
    let name = serde_json::to_string(if name.trim().is_empty() {
        "App"
    } else {
        name.trim()
    })
    .expect("strings serialize");
    format!(
        "{{\n  \"name\": {name},\n  \"short_name\": {name},\n  \"icons\": [\n    {{ \"src\": \"/android-chrome-192x192.png\", \"sizes\": \"192x192\", \"type\": \"image/png\" }},\n    {{ \"src\": \"/android-chrome-512x512.png\", \"sizes\": \"512x512\", \"type\": \"image/png\" }}\n  ],\n  \"display\": \"standalone\"\n}}\n"
    )
}

/// Every file for the chosen sets. `name` is the app name used in `site.webmanifest`.
pub fn generate(
    image: &DynamicImage,
    fit: Fit,
    sets: &[IconSet],
    name: &str,
) -> Result<Vec<GeneratedFile>, Error> {
    let square = square(image, fit);
    let file = |name: &str, bytes: Vec<u8>| GeneratedFile {
        name: name.to_owned(),
        bytes,
    };
    let mut files = Vec::new();
    for set in IconSet::ALL.into_iter().filter(|set| sets.contains(set)) {
        match set {
            IconSet::Favicons => {
                files.push(file("favicon.ico", ico(&square, &FAVICON_ICO_SIZES)?));
                for (name, size) in [
                    ("favicon-16x16.png", 16),
                    ("favicon-32x32.png", 32),
                    ("apple-touch-icon.png", 180),
                    ("android-chrome-192x192.png", 192),
                    ("android-chrome-512x512.png", 512),
                ] {
                    files.push(file(name, png(&scaled(&square, size))?));
                }
                files.push(file("site.webmanifest", webmanifest(name).into_bytes()));
            }
            IconSet::MacOs => files.push(file("AppIcon.icns", icns(&square)?)),
            IconSet::Windows => files.push(file("app.ico", ico(&square, &WINDOWS_SIZES)?)),
            IconSet::PngSizes => {
                for size in PNG_SIZES {
                    files.push(file(
                        &format!("icon-{size}.png"),
                        png(&scaled(&square, size))?,
                    ));
                }
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::{decode, test_images};

    fn source() -> DynamicImage {
        DynamicImage::ImageRgba8(test_images::sample())
    }

    #[test]
    fn thin_images_are_shrunk_before_padding() {
        let thin = DynamicImage::new_rgba8(1, 10_000);
        assert_eq!(
            square(&thin, Fit::Pad).dimensions(),
            (MAX_SQUARE, MAX_SQUARE)
        );
        assert_eq!(square(&thin, Fit::Crop).dimensions(), (1, 1));
    }

    #[test]
    fn squares_by_padding_or_cropping() {
        let padded = square(&source(), Fit::Pad);
        assert_eq!(padded.dimensions(), (40, 40));
        assert_eq!(padded.get_pixel(0, 0).0[3], 0, "padding is transparent");
        assert_eq!(padded.get_pixel(5, 20).0, [255, 0, 0, 255]);
        let cropped = square(&source(), Fit::Crop);
        assert_eq!(cropped.dimensions(), (30, 30));
        assert_eq!(cropped.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn ico_holds_every_size() {
        let bytes = ico(&square(&source(), Fit::Pad), &WINDOWS_SIZES).unwrap();
        // ICONDIR: reserved, type 1, count.
        assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 7);
        let largest = decode(&bytes).unwrap().image;
        assert_eq!((largest.width(), largest.height()), (256, 256));
    }

    #[test]
    fn icns_is_well_formed() {
        let bytes = icns(&square(&source(), Fit::Pad)).unwrap();
        assert_eq!(&bytes[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len()
        );
        let mut kinds = Vec::new();
        let mut pos = 8;
        while pos < bytes.len() {
            let length = u32::from_be_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
            kinds.push(String::from_utf8_lossy(&bytes[pos..pos + 4]).into_owned());
            let entry = decode(&bytes[pos + 8..pos + length]).unwrap().image;
            assert_eq!(entry.width(), entry.height());
            pos += length;
        }
        assert_eq!(
            kinds,
            [
                "icp4", "ic11", "icp5", "ic12", "ic07", "ic13", "ic08", "ic14", "ic09", "ic10"
            ]
        );
    }

    /// Apple's own tool must accept the file.
    #[cfg(target_os = "macos")]
    #[test]
    fn iconutil_reads_our_icns() {
        let dir = std::env::temp_dir().join(format!("toolbox-icns-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let icns_path = dir.join("AppIcon.icns");
        std::fs::write(&icns_path, icns(&square(&source(), Fit::Pad)).unwrap()).unwrap();
        let iconset = dir.join("AppIcon.iconset");
        let output = std::process::Command::new("iconutil")
            .arg("-c")
            .arg("iconset")
            .arg(&icns_path)
            .arg("-o")
            .arg(&iconset)
            .output()
            .unwrap();
        let listing: Vec<String> = std::fs::read_dir(&iconset)
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            output.status.success(),
            "iconutil failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["icon_16x16.png", "icon_16x16@2x.png", "icon_512x512@2x.png"] {
            assert!(
                listing.iter().any(|n| n == name),
                "{name} missing from {listing:?}"
            );
        }
    }

    #[test]
    fn generates_the_chosen_sets() {
        let files = generate(
            &source(),
            Fit::Pad,
            &[IconSet::Favicons, IconSet::Windows],
            "My \"App\"",
        )
        .unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "favicon.ico",
                "favicon-16x16.png",
                "favicon-32x32.png",
                "apple-touch-icon.png",
                "android-chrome-192x192.png",
                "android-chrome-512x512.png",
                "site.webmanifest",
                "app.ico"
            ]
        );
        let touch = files
            .iter()
            .find(|f| f.name == "apple-touch-icon.png")
            .unwrap();
        assert_eq!(decode(&touch.bytes).unwrap().image.width(), 180);
        let manifest: serde_json::Value = serde_json::from_slice(&files[6].bytes).unwrap();
        assert_eq!(manifest["name"], "My \"App\"");
        assert_eq!(
            generate(&source(), Fit::Crop, &[IconSet::PngSizes], "")
                .unwrap()
                .len(),
            8
        );
    }
}
