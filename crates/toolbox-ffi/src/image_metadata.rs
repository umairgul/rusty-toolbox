//! Metadata Remover: what a photo reveals, and a clean copy without it.

use crate::ToolError;
use std::path::Path;
use toolbox_core::images::metadata::{self, Container, Field, Gps, Report};

/// The order groups are shown in: the most revealing first.
const GROUP_ORDER: [&str; 4] = ["Location", "Camera", "Image", "Other"];

#[derive(uniffi::Record)]
pub struct ImageMetadataField {
    pub name: String,
    pub value: String,
    /// Personal or identifying: location, device, serial numbers, owner, dates.
    pub sensitive: bool,
}

impl From<&Field> for ImageMetadataField {
    fn from(field: &Field) -> Self {
        ImageMetadataField {
            name: field.name.clone(),
            value: field.value.clone(),
            sensitive: field.sensitive,
        }
    }
}

/// EXIF fields of one kind: "Location", "Camera", "Image" or "Other".
#[derive(uniffi::Record)]
pub struct ImageMetadataGroup {
    pub name: String,
    pub fields: Vec<ImageMetadataField>,
}

#[derive(uniffi::Record)]
pub struct ImageMetadataLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// Meters above sea level.
    pub altitude: Option<f64>,
    /// The spot on openstreetmap.org.
    pub open_street_map_url: String,
    /// The spot in Apple Maps (opens the Maps app on macOS).
    pub apple_maps_url: String,
}

impl From<Gps> for ImageMetadataLocation {
    fn from(gps: Gps) -> Self {
        ImageMetadataLocation {
            latitude: gps.latitude,
            longitude: gps.longitude,
            altitude: gps.altitude,
            open_street_map_url: gps.maps_url(),
            apple_maps_url: apple_maps_url(gps),
        }
    }
}

fn apple_maps_url(gps: Gps) -> String {
    format!(
        "https://maps.apple.com/?ll={:.6},{:.6}&q=Photo%20location",
        gps.latitude, gps.longitude
    )
}

#[derive(uniffi::Record)]
pub struct ImageMetadataReport {
    pub file_size: u64,
    /// EXIF fields of the main image, grouped, in display order. Empty groups are left out.
    pub groups: Vec<ImageMetadataGroup>,
    pub field_count: u32,
    /// How many fields are personal or identifying.
    pub sensitive_count: u32,
    pub location: Option<ImageMetadataLocation>,
    /// Other metadata blocks found, e.g. "XMP metadata" or "PNG text: Software".
    pub other: Vec<String>,
    /// Nothing but (at most) the orientation.
    pub is_clean: bool,
    /// Whether a clean copy can be saved losslessly (JPEG, PNG and WebP).
    pub can_clean: bool,
}

fn groups(fields: &[Field]) -> Vec<ImageMetadataGroup> {
    let known = |group: &str| GROUP_ORDER[..3].contains(&group);
    GROUP_ORDER
        .iter()
        .map(|&name| ImageMetadataGroup {
            name: name.into(),
            // Anything in a group the UI doesn't know about goes under "Other".
            fields: fields
                .iter()
                .filter(|f| f.group == name || (name == "Other" && !known(f.group)))
                .map(Into::into)
                .collect(),
        })
        .filter(|group| !group.fields.is_empty())
        .collect()
}

fn report(report: Report, file_size: u64) -> ImageMetadataReport {
    ImageMetadataReport {
        file_size,
        groups: groups(&report.fields),
        field_count: report.fields.len() as u32,
        sensitive_count: report.sensitive_count() as u32,
        location: report.gps.map(Into::into),
        is_clean: report.is_clean(),
        can_clean: report.container != Container::Other,
        other: report.other,
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, ToolError> {
    std::fs::read(path).map_err(|error| ToolError::new(format!("Couldn't open: {error}")))
}

/// Reads what an image file reveals.
#[uniffi::export]
pub fn image_metadata_read(path: &str) -> Result<ImageMetadataReport, ToolError> {
    let bytes = read_file(Path::new(path))?;
    Ok(report(metadata::read(&bytes), bytes.len() as u64))
}

#[derive(uniffi::Record)]
pub struct ImageMetadataSaved {
    pub output_path: String,
    /// What was removed, for display.
    pub removed: Vec<String>,
    /// The saved file was read back and has no metadata left.
    pub verified_clean: bool,
    /// Only the orientation was kept, so the photo still displays upright.
    pub kept_orientation: bool,
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Saves a copy of `input` without metadata to `output`, losslessly (JPEG, PNG and WebP),
/// then reads it back to confirm nothing is left. Refuses to overwrite the original.
#[uniffi::export]
pub fn image_metadata_save_clean(
    input: &str,
    output: &str,
) -> Result<ImageMetadataSaved, ToolError> {
    let (input, output) = (Path::new(input), Path::new(output));
    if same_file(input, output) {
        return Err(ToolError::new(
            "Choose a different name: the original is kept untouched.",
        ));
    }
    let stripped = metadata::strip(&read_file(input)?).map_err(ToolError::new)?;
    std::fs::write(output, &stripped.bytes)
        .map_err(|error| ToolError::new(format!("Couldn't save: {error}")))?;
    let check = metadata::read(&read_file(output)?);
    Ok(ImageMetadataSaved {
        output_path: output.to_string_lossy().into_owned(),
        removed: stripped.removed,
        verified_clean: check.is_clean(),
        kept_orientation: check.orientation != 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(group: &'static str, name: &str, sensitive: bool) -> Field {
        Field {
            group,
            name: name.into(),
            value: "x".into(),
            sensitive,
        }
    }

    #[test]
    fn groups_in_display_order() {
        let fields = [
            field("Image", "Orientation", false),
            field("Camera", "Model", true),
            field("Location", "GPSLatitude", true),
            field("Interop", "InteroperabilityIndex", false),
            field("Image", "XResolution", false),
        ];
        let groups = groups(&fields);
        let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["Location", "Camera", "Image", "Other"]);
        assert_eq!(groups[2].fields.len(), 2);
        assert_eq!(groups[3].fields[0].name, "InteroperabilityIndex");
        assert!(super::groups(&[]).is_empty());
    }

    #[test]
    fn links_to_the_location() {
        let location = ImageMetadataLocation::from(Gps {
            latitude: 38.7223,
            longitude: -9.1393,
            altitude: None,
        });
        assert_eq!(
            location.apple_maps_url,
            "https://maps.apple.com/?ll=38.722300,-9.139300&q=Photo%20location"
        );
        assert!(location.open_street_map_url.contains("openstreetmap.org"));
    }

    /// A 1×1 PNG with a `tEXt` chunk ("Software").
    const PNG_WITH_TEXT: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAFnRFWHRTb2Z0d2FyZQBUb29sYm94IHRlc3RzWrZAdwAAAA1JREFUeJxj+M/A8B8ABQAB/4mZPR0AAAAASUVORK5CYII=";

    #[test]
    fn saves_a_verified_clean_copy() {
        let png = toolbox_core::images::data_uri::decode(PNG_WITH_TEXT)
            .unwrap()
            .1;
        let dir = std::env::temp_dir().join(format!("toolbox-ffi-metadata-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("in.png");
        let output = dir.join("in-clean.png");
        std::fs::write(&input, &png).unwrap();
        let (input, output) = (input.to_str().unwrap(), output.to_str().unwrap());

        let before = image_metadata_read(input).unwrap();
        assert!(before.can_clean);
        assert!(!before.is_clean, "{:?}", before.other);
        assert!(image_metadata_save_clean(input, input).is_err());

        let saved = image_metadata_save_clean(input, output).unwrap();
        assert!(saved.verified_clean);
        assert!(!saved.kept_orientation);
        assert!(!saved.removed.is_empty());
        assert!(image_metadata_read(output).unwrap().is_clean);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
