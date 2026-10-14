//! Icon Generator: favicons, a macOS `.icns`, a Windows `.ico` and PNGs from one image.

use crate::ToolError;
use std::path::Path;
use toolbox_core::images::icons::{self, FAVICON_HTML, Fit, IconSet};
use toolbox_core::images::{self, Loaded};

/// How a non-square image becomes square.
#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum IconFit {
    /// Add transparent space around the shorter side.
    Pad,
    /// Cut the longer side, keeping the center.
    Crop,
}

impl From<IconFit> for Fit {
    fn from(fit: IconFit) -> Self {
        match fit {
            IconFit::Pad => Fit::Pad,
            IconFit::Crop => Fit::Crop,
        }
    }
}

#[uniffi::export]
pub fn icon_fit_label(fit: IconFit) -> String {
    Fit::from(fit).label().into()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum IconSetKind {
    Favicons,
    MacOs,
    Windows,
    PngSizes,
}

impl From<IconSetKind> for IconSet {
    fn from(set: IconSetKind) -> Self {
        match set {
            IconSetKind::Favicons => IconSet::Favicons,
            IconSetKind::MacOs => IconSet::MacOs,
            IconSetKind::Windows => IconSet::Windows,
            IconSetKind::PngSizes => IconSet::PngSizes,
        }
    }
}

#[derive(uniffi::Record)]
pub struct IconSetInfo {
    /// e.g. "Website favicons".
    pub label: String,
    /// The files it makes.
    pub description: String,
}

#[uniffi::export]
pub fn icon_set_info(set: IconSetKind) -> IconSetInfo {
    let set = IconSet::from(set);
    IconSetInfo {
        label: set.label().into(),
        description: set.description().into(),
    }
}

/// Link tags for the favicon set, to paste into a page's `<head>`.
#[uniffi::export]
pub fn icon_favicon_html() -> String {
    FAVICON_HTML.into()
}

/// The smallest source side that keeps the largest icons sharp.
#[uniffi::export]
pub fn icon_recommended_size() -> u32 {
    1024
}

/// A decoded source image, kept so generating doesn't decode it again.
#[derive(uniffi::Object)]
pub struct IconSource {
    loaded: Loaded,
}

#[uniffi::export]
impl IconSource {
    /// Loads any supported image, including SVG (rendered at 1024 px or more) and HEIC. Slow
    /// for large images: call it off the main thread.
    #[uniffi::constructor]
    pub fn open(path: &str) -> Result<Self, ToolError> {
        let loaded = images::load(Path::new(path)).map_err(ToolError::new)?;
        Ok(IconSource { loaded })
    }

    pub fn width(&self) -> u32 {
        self.loaded.image.width()
    }

    pub fn height(&self) -> u32 {
        self.loaded.image.height()
    }

    /// Makes the files for `sets` and writes them into `folder`, replacing files with the same
    /// names. Returns the names written, in order. Slow: call it off the main thread, one set
    /// at a time to show progress.
    pub fn generate(
        &self,
        folder: &str,
        fit: IconFit,
        sets: Vec<IconSetKind>,
        app_name: &str,
    ) -> Result<Vec<String>, ToolError> {
        let sets: Vec<IconSet> = sets.into_iter().map(Into::into).collect();
        let files = icons::generate(&self.loaded.image, fit.into(), &sets, app_name)
            .map_err(ToolError::new)?;
        let folder = Path::new(folder);
        files
            .into_iter()
            .map(|file| {
                std::fs::write(folder.join(&file.name), &file.bytes).map_err(|error| {
                    ToolError::new(format!("Couldn't save {}: {error}", file.name))
                })?;
                Ok(file.name)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_into_a_folder() {
        let dir = std::env::temp_dir().join(format!("toolbox-ffi-icons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let svg = dir.join("source.svg");
        std::fs::write(
            &svg,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="30"><rect width="40" height="30" fill="#ff0000"/></svg>"##,
        )
        .unwrap();
        let source = IconSource::open(svg.to_str().unwrap()).unwrap();
        assert_eq!((source.width(), source.height()), (1024, 768));
        let written = source
            .generate(
                dir.to_str().unwrap(),
                IconFit::Crop,
                vec![IconSetKind::Windows, IconSetKind::MacOs],
                "Test",
            )
            .unwrap();
        // In the core's order, not the order asked for.
        assert_eq!(written, ["AppIcon.icns", "app.ico"]);
        assert!(dir.join("app.ico").exists());
        assert!(
            source
                .generate(
                    "/nonexistent/folder",
                    IconFit::Pad,
                    vec![IconSetKind::Windows],
                    ""
                )
                .is_err()
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(IconSource::open("/nonexistent.png").is_err());
    }

    #[test]
    fn labels_come_from_the_core() {
        assert_eq!(
            icon_set_info(IconSetKind::Favicons).label,
            "Website favicons"
        );
        assert_eq!(icon_fit_label(IconFit::Crop), "Crop to center");
        assert!(icon_favicon_html().contains("site.webmanifest"));
    }
}
