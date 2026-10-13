//! Image metadata: report what a file reveals (camera, dates, GPS location, …) and remove it
//! losslessly from JPEG, PNG and WebP.
//!
//! Removal rewrites the file container without decoding the picture, so the image data is
//! byte-for-byte the same. Color profiles are kept, since they're needed to show colors
//! correctly. If the photo is stored rotated, a minimal EXIF block with just the orientation is
//! written back so it still displays upright.

use super::Error;
use exif::{Context, In, Tag, Value};
use std::io::Cursor;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// "Image", "Camera", "Location" or "Other".
    pub group: &'static str,
    pub name: String,
    pub value: String,
    /// Personal or identifying: location, device, serial numbers, owner, dates.
    pub sensitive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gps {
    pub latitude: f64,
    pub longitude: f64,
    /// Meters above sea level.
    pub altitude: Option<f64>,
}

impl Gps {
    pub fn maps_url(self) -> String {
        format!(
            "https://www.openstreetmap.org/?mlat={:.6}&mlon={:.6}#map=16/{:.6}/{:.6}",
            self.latitude, self.longitude, self.latitude, self.longitude
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Jpeg,
    Png,
    Webp,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub container: Container,
    /// EXIF fields of the main image.
    pub fields: Vec<Field>,
    pub gps: Option<Gps>,
    /// Other metadata blocks found, e.g. "XMP metadata" or "PNG text: Software".
    pub other: Vec<String>,
    /// EXIF orientation, 1 (upright) to 8.
    pub orientation: u16,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.fields.iter().all(|f| f.tag_is_orientation()) && self.other.is_empty()
    }

    pub fn sensitive_count(&self) -> usize {
        self.fields.iter().filter(|f| f.sensitive).count()
    }
}

impl Field {
    fn tag_is_orientation(&self) -> bool {
        self.name == "Orientation"
    }
}

/// Values longer than this are cut short in the report (maker notes can be kilobytes).
const MAX_VALUE_CHARS: usize = 160;

pub fn read(bytes: &[u8]) -> Report {
    let container = container(bytes);
    let mut report = Report {
        container,
        fields: Vec::new(),
        gps: None,
        other: Vec::new(),
        orientation: 1,
    };

    if let Ok(exif) = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) {
        for field in exif.fields().filter(|f| f.ifd_num == In::PRIMARY) {
            if field.tag == Tag::MakerNote {
                report.fields.push(Field {
                    group: "Camera",
                    name: "Maker note".into(),
                    value: format!("{} bytes of manufacturer data", raw_len(&field.value)),
                    sensitive: true,
                });
                continue;
            }
            let mut value = field.display_value().with_unit(&exif).to_string();
            if value.chars().count() > MAX_VALUE_CHARS {
                value = value.chars().take(MAX_VALUE_CHARS).collect::<String>() + "…";
            }
            report.fields.push(Field {
                group: group(field.tag),
                name: field.tag.to_string(),
                value,
                sensitive: is_sensitive(field.tag),
            });
        }
        if exif.fields().any(|f| f.ifd_num == In::THUMBNAIL) {
            report
                .other
                .push("Embedded thumbnail (can show the original before edits or crops)".into());
        }
        report.orientation = exif
            .get_field(Tag::Orientation, In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
            .map_or(1, |o| o as u16);
        report.gps = gps(&exif);
    }

    report.other.extend(match container {
        Container::Jpeg => jpeg::other_metadata(bytes),
        Container::Png => png::other_metadata(bytes),
        Container::Webp => webp::other_metadata(bytes),
        Container::Other => Vec::new(),
    });
    report
}

fn raw_len(value: &Value) -> usize {
    match value {
        Value::Undefined(bytes, _) => bytes.len(),
        Value::Byte(bytes) => bytes.len(),
        _ => 0,
    }
}

fn container(bytes: &[u8]) -> Container {
    match super::mime_type(bytes) {
        Some("image/jpeg") => Container::Jpeg,
        Some("image/png") => Container::Png,
        Some("image/webp") => Container::Webp,
        _ => Container::Other,
    }
}

fn group(tag: Tag) -> &'static str {
    match tag.context() {
        Context::Gps => "Location",
        Context::Exif => "Camera",
        Context::Tiff => "Image",
        _ => "Other",
    }
}

fn is_sensitive(tag: Tag) -> bool {
    tag.context() == Context::Gps
        || [
            Tag::Make,
            Tag::Model,
            Tag::BodySerialNumber,
            Tag::LensSerialNumber,
            Tag::LensMake,
            Tag::LensModel,
            Tag::CameraOwnerName,
            Tag::Artist,
            Tag::Copyright,
            Tag::DateTime,
            Tag::DateTimeOriginal,
            Tag::DateTimeDigitized,
            Tag::OffsetTime,
            Tag::OffsetTimeOriginal,
            Tag::OffsetTimeDigitized,
            Tag::ImageUniqueID,
            Tag::UserComment,
            Tag::ImageDescription,
            Tag::Software,
        ]
        .contains(&tag)
}

fn gps(exif: &exif::Exif) -> Option<Gps> {
    let degrees = |tag: Tag, negative_ref: &[u8]| -> Option<f64> {
        let Value::Rational(parts) = &exif.get_field(tag, In::PRIMARY)?.value else {
            return None;
        };
        let value = parts
            .iter()
            .zip([1.0, 60.0, 3600.0])
            .map(|(part, divisor)| part.to_f64() / divisor)
            .sum::<f64>();
        let reference_tag = if tag == Tag::GPSLatitude {
            Tag::GPSLatitudeRef
        } else {
            Tag::GPSLongitudeRef
        };
        let negative = matches!(&exif.get_field(reference_tag, In::PRIMARY).map(|f| &f.value), Some(Value::Ascii(refs)) if refs.first().is_some_and(|r| r.as_slice() == negative_ref));
        Some(if negative { -value } else { value })
    };
    let latitude = degrees(Tag::GPSLatitude, b"S")?;
    let longitude = degrees(Tag::GPSLongitude, b"W")?;
    // Altitude is optional: many photos record a position without it.
    let altitude = match exif
        .get_field(Tag::GPSAltitude, In::PRIMARY)
        .map(|f| &f.value)
    {
        Some(Value::Rational(parts)) => parts.first().map(|r| {
            let below = exif
                .get_field(Tag::GPSAltitudeRef, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                == Some(1);
            if below { -r.to_f64() } else { r.to_f64() }
        }),
        _ => None,
    };
    Some(Gps {
        latitude,
        longitude,
        altitude,
    })
}

/// A minimal big-endian TIFF/EXIF block holding only the orientation tag.
fn orientation_tiff(orientation: u16) -> Vec<u8> {
    let mut tiff = Vec::with_capacity(26);
    tiff.extend_from_slice(b"MM\0\x2a"); // big-endian TIFF header
    tiff.extend_from_slice(&8u32.to_be_bytes()); // IFD0 starts right after the header
    tiff.extend_from_slice(&1u16.to_be_bytes()); // one entry
    tiff.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    tiff.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    tiff.extend_from_slice(&1u32.to_be_bytes()); // count
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0]); // value padding
    tiff.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    tiff
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stripped {
    pub bytes: Vec<u8>,
    /// What was removed, for display.
    pub removed: Vec<String>,
}

/// Removes metadata from a JPEG, PNG or WebP file without re-encoding the image.
pub fn strip(bytes: &[u8]) -> Result<Stripped, Error> {
    let orientation = read(bytes).orientation;
    match container(bytes) {
        Container::Jpeg => jpeg::strip(bytes, orientation),
        Container::Png => png::strip(bytes, orientation),
        Container::Webp => webp::strip(bytes, orientation),
        Container::Other => Err(Error(
            "Lossless cleaning works for JPEG, PNG and WebP. For other formats, use the Image Converter: re-encoding removes metadata.".into(),
        )),
    }
}

fn add_unique(list: &mut Vec<String>, item: impl Into<String>) {
    let item = item.into();
    if !list.contains(&item) {
        list.push(item);
    }
}

mod jpeg {
    use super::{Error, Stripped, add_unique, orientation_tiff};

    const SOI: u8 = 0xd8;
    const EOI: u8 = 0xd9;
    const SOS: u8 = 0xda;
    const APP0: u8 = 0xe0;
    const APP1: u8 = 0xe1;
    const APP2: u8 = 0xe2;
    const APP13: u8 = 0xed;
    const APP14: u8 = 0xee;
    const COM: u8 = 0xfe;

    struct Segment<'a> {
        marker: u8,
        /// The whole segment, marker and length included.
        raw: &'a [u8],
        /// Contents after the length field.
        contents: &'a [u8],
    }

    /// A JPEG split into the segments before the image data, the image data itself (SOS to
    /// EOI), and anything after EOI.
    struct Parsed<'a> {
        segments: Vec<Segment<'a>>,
        image_data: &'a [u8],
        trailing: &'a [u8],
    }

    fn parse(bytes: &[u8]) -> Result<Parsed<'_>, Error> {
        let invalid = || Error("This JPEG file is damaged".into());
        if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != SOI {
            return Err(invalid());
        }
        let mut segments = Vec::new();
        let mut pos = 2;
        loop {
            if pos + 4 > bytes.len() || bytes[pos] != 0xff {
                return Err(invalid());
            }
            let marker = bytes[pos + 1];
            if marker == 0xff {
                pos += 1; // fill byte
                continue;
            }
            let length = usize::from(u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]));
            let end = pos + 2 + length;
            if length < 2 || end > bytes.len() {
                return Err(invalid());
            }
            if marker == SOS {
                let image_end = find_eoi(bytes, end).ok_or_else(invalid)?;
                return Ok(Parsed {
                    segments,
                    image_data: &bytes[pos..image_end],
                    trailing: &bytes[image_end..],
                });
            }
            segments.push(Segment {
                marker,
                raw: &bytes[pos..end],
                contents: &bytes[pos + 4..end],
            });
            pos = end;
        }
    }

    /// The offset just past the EOI marker ending the entropy-coded data that starts at `from`.
    /// Inside that data, 0xFF is followed by 0x00 (stuffing) or a restart marker; progressive
    /// JPEGs also contain further table and scan segments.
    fn find_eoi(bytes: &[u8], from: usize) -> Option<usize> {
        let mut i = from;
        while i + 1 < bytes.len() {
            if bytes[i] != 0xff {
                i += 1;
                continue;
            }
            match bytes[i + 1] {
                EOI => return Some(i + 2),
                0x00 | 0xd0..=0xd7 | 0xff => i += 1,
                _ => i += 2,
            }
        }
        None
    }

    fn describe(segment: &Segment<'_>) -> Option<String> {
        let starts = |prefix: &[u8]| segment.contents.starts_with(prefix);
        Some(match segment.marker {
            APP1 if starts(b"Exif\0") => "EXIF metadata".into(),
            APP1 if starts(b"http://ns.adobe.com/xap/1.0/\0") => "XMP metadata".into(),
            APP1 if starts(b"http://ns.adobe.com/xmp/extension/\0") => {
                "Extended XMP metadata".into()
            }
            APP2 if starts(b"MPF\0") => {
                "Multi-picture data (extra images such as depth maps)".into()
            }
            APP13 => "IPTC / Photoshop metadata".into(),
            COM => format!(
                "Comment: \"{}\"",
                String::from_utf8_lossy(segment.contents).trim()
            ),
            0xe1..=0xef if !keep(segment) => format!("APP{} block", segment.marker - APP0),
            _ => return None,
        })
    }

    /// JFIF (APP0), the ICC color profile (APP2) and Adobe color info (APP14) are kept; every
    /// other application block and comments are removed.
    fn keep(segment: &Segment<'_>) -> bool {
        match segment.marker {
            APP0 | APP14 => true,
            APP2 => segment.contents.starts_with(b"ICC_PROFILE\0"),
            0xe1..=0xef | COM => false,
            _ => true,
        }
    }

    pub fn other_metadata(bytes: &[u8]) -> Vec<String> {
        let Ok(Parsed {
            segments, trailing, ..
        }) = parse(bytes)
        else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for segment in &segments {
            if let Some(description) = describe(segment)
                && description != "EXIF metadata"
            {
                add_unique(&mut found, description);
            }
        }
        if !trailing.is_empty() {
            found.push(format!(
                "{} bytes of extra data after the image",
                trailing.len()
            ));
        }
        found
    }

    pub fn strip(bytes: &[u8], orientation: u16) -> Result<Stripped, Error> {
        let Parsed {
            segments,
            image_data,
            trailing,
        } = parse(bytes)?;
        let mut out = Vec::with_capacity(bytes.len());
        out.extend_from_slice(&[0xff, SOI]);
        let mut removed = Vec::new();
        let mut orientation_written = orientation == 1;
        for segment in &segments {
            if !orientation_written && segment.marker != APP0 {
                let mut payload = b"Exif\0\0".to_vec();
                payload.extend(orientation_tiff(orientation));
                out.extend_from_slice(&[0xff, APP1]);
                out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
                out.extend(payload);
                orientation_written = true;
            }
            if keep(segment) {
                out.extend_from_slice(segment.raw);
            } else if let Some(description) = describe(segment) {
                let description = if description.starts_with("Comment") {
                    "Comment".to_owned()
                } else {
                    description
                };
                add_unique(&mut removed, description);
            }
        }
        out.extend_from_slice(image_data);
        if !trailing.is_empty() {
            removed.push(format!(
                "{} bytes of extra data after the image",
                trailing.len()
            ));
        }
        Ok(Stripped {
            bytes: out,
            removed,
        })
    }
}

mod png {
    use super::{Error, Stripped, add_unique, orientation_tiff};

    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    /// Chunks that carry metadata rather than pixels or color information.
    const METADATA: [&[u8; 4]; 5] = [b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"tIME"];

    struct Chunk<'a> {
        kind: [u8; 4],
        data: &'a [u8],
        raw: &'a [u8],
    }

    fn parse(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, Error> {
        let invalid = || Error("This PNG file is damaged".into());
        if !bytes.starts_with(SIGNATURE) {
            return Err(invalid());
        }
        let mut chunks = Vec::new();
        let mut pos = SIGNATURE.len();
        while pos + 12 <= bytes.len() {
            let length =
                u32::from_be_bytes(bytes[pos..pos + 4].try_into().expect("4 bytes")) as usize;
            let end = pos + 12 + length;
            if end > bytes.len() {
                return Err(invalid());
            }
            let kind: [u8; 4] = bytes[pos + 4..pos + 8].try_into().expect("4 bytes");
            chunks.push(Chunk {
                kind,
                data: &bytes[pos + 8..pos + 8 + length],
                raw: &bytes[pos..end],
            });
            pos = end;
            if &kind == b"IEND" {
                break;
            }
        }
        Ok(chunks)
    }

    fn describe(chunk: &Chunk<'_>) -> Option<String> {
        let keyword = || {
            String::from_utf8_lossy(chunk.data.split(|&b| b == 0).next().unwrap_or_default())
                .into_owned()
        };
        Some(match &chunk.kind {
            b"iTXt" if keyword() == "XML:com.adobe.xmp" => "XMP metadata".into(),
            b"tEXt" => {
                let value = chunk.data.splitn(2, |&b| b == 0).nth(1).unwrap_or_default();
                let value = String::from_utf8_lossy(&value[..value.len().min(80)]).into_owned();
                format!("PNG text: {} = \"{}\"", keyword(), value)
            }
            b"zTXt" | b"iTXt" => format!("PNG text: {}", keyword()),
            b"tIME" => "Last-modified time".into(),
            b"eXIf" => "EXIF metadata".into(),
            _ => return None,
        })
    }

    pub fn other_metadata(bytes: &[u8]) -> Vec<String> {
        let mut found = Vec::new();
        for chunk in parse(bytes).unwrap_or_default() {
            if let Some(description) = describe(&chunk).filter(|d| d != "EXIF metadata") {
                add_unique(&mut found, description);
            }
        }
        found
    }

    pub(super) fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(data);
        out.extend_from_slice(&crc.finalize().to_be_bytes());
    }

    pub fn strip(bytes: &[u8], orientation: u16) -> Result<Stripped, Error> {
        let chunks = parse(bytes)?;
        let mut out = Vec::with_capacity(bytes.len());
        out.extend_from_slice(SIGNATURE);
        let mut removed = Vec::new();
        let mut orientation_written = orientation == 1;
        for chunk in &chunks {
            // eXIf must come before the image data.
            if !orientation_written && &chunk.kind == b"IDAT" {
                write_chunk(&mut out, b"eXIf", &orientation_tiff(orientation));
                orientation_written = true;
            }
            if METADATA.contains(&&chunk.kind) {
                if let Some(description) = describe(chunk) {
                    let description = if description.starts_with("PNG text") {
                        "PNG text".to_owned()
                    } else {
                        description
                    };
                    add_unique(&mut removed, description);
                }
            } else {
                out.extend_from_slice(chunk.raw);
            }
        }
        Ok(Stripped {
            bytes: out,
            removed,
        })
    }
}

mod webp {
    use super::{Error, Stripped, orientation_tiff};

    /// VP8X flag bits for metadata.
    const FLAG_EXIF: u8 = 0x08;
    const FLAG_XMP: u8 = 0x04;

    /// Each chunk's four-character id and data.
    type Chunks<'a> = Vec<([u8; 4], &'a [u8])>;

    fn chunks(bytes: &[u8]) -> Result<Chunks<'_>, Error> {
        let invalid = || Error("This WebP file is damaged".into());
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
            return Err(invalid());
        }
        let mut found = Vec::new();
        let mut pos = 12;
        while pos + 8 <= bytes.len() {
            let id: [u8; 4] = bytes[pos..pos + 4].try_into().expect("4 bytes");
            let size =
                u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().expect("4 bytes")) as usize;
            let end = pos + 8 + size;
            if end > bytes.len() {
                return Err(invalid());
            }
            found.push((id, &bytes[pos + 8..end]));
            pos = end + size % 2; // chunks are padded to an even size
        }
        Ok(found)
    }

    pub fn other_metadata(bytes: &[u8]) -> Vec<String> {
        chunks(bytes)
            .unwrap_or_default()
            .iter()
            .filter(|(id, _)| id == b"XMP ")
            .map(|_| "XMP metadata".to_owned())
            .take(1)
            .collect()
    }

    fn write_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(id);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(0);
        }
    }

    pub fn strip(bytes: &[u8], orientation: u16) -> Result<Stripped, Error> {
        let chunks = chunks(bytes)?;
        let has_vp8x = chunks.iter().any(|(id, _)| id == b"VP8X");
        // Without a VP8X header the file can't declare EXIF, so orientation can't be kept.
        let keep_orientation = orientation != 1 && has_vp8x;
        let mut body = Vec::with_capacity(bytes.len());
        let mut removed = Vec::new();
        for (id, data) in &chunks {
            match id {
                b"EXIF" => removed.push("EXIF metadata".to_owned()),
                b"XMP " => removed.push("XMP metadata".to_owned()),
                b"VP8X" if !data.is_empty() => {
                    let mut header = data.to_vec();
                    header[0] &= !(FLAG_EXIF | FLAG_XMP);
                    if keep_orientation {
                        header[0] |= FLAG_EXIF;
                    }
                    write_chunk(&mut body, id, &header);
                }
                _ => write_chunk(&mut body, id, data),
            }
        }
        if keep_orientation {
            // EXIF goes after the image data in the extended format.
            write_chunk(&mut body, b"EXIF", &orientation_tiff(orientation));
        }
        removed.dedup();
        let mut out = Vec::with_capacity(body.len() + 12);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
        out.extend_from_slice(b"WEBP");
        out.extend(body);
        Ok(Stripped {
            bytes: out,
            removed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::{decode, test_images};
    use exif::experimental::Writer;
    use exif::{Field as ExifField, Rational};
    use image::ImageFormat;

    /// EXIF with a camera, a date, a location in Lisbon and the given orientation, as TIFF bytes.
    fn sample_exif(orientation: u16) -> Vec<u8> {
        let rational = |num: u32, denom: u32| Rational { num, denom };
        let fields = [
            ExifField {
                tag: Tag::Make,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"Toolbox Camera".to_vec()]),
            },
            ExifField {
                tag: Tag::Model,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"Model T".to_vec()]),
            },
            ExifField {
                tag: Tag::Orientation,
                ifd_num: In::PRIMARY,
                value: Value::Short(vec![orientation]),
            },
            ExifField {
                tag: Tag::DateTimeOriginal,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"2024:05:15 10:30:00".to_vec()]),
            },
            ExifField {
                tag: Tag::GPSLatitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"N".to_vec()]),
            },
            ExifField {
                tag: Tag::GPSLatitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![rational(38, 1), rational(42, 1), rational(3600, 100)]),
            },
            ExifField {
                tag: Tag::GPSLongitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"W".to_vec()]),
            },
            ExifField {
                tag: Tag::GPSLongitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![rational(9, 1), rational(8, 1), rational(2400, 100)]),
            },
            ExifField {
                tag: Tag::GPSAltitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Byte(vec![0]),
            },
            ExifField {
                tag: Tag::GPSAltitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![rational(125, 2)]),
            },
        ];
        let mut writer = Writer::new();
        for field in &fields {
            writer.push_field(field);
        }
        let mut tiff = Cursor::new(Vec::new());
        writer.write(&mut tiff, false).unwrap();
        tiff.into_inner()
    }

    /// A JPEG with EXIF (incl. GPS), XMP, a comment, IPTC and trailing data.
    fn tagged_jpeg(orientation: u16) -> Vec<u8> {
        let plain = test_images::encoded(ImageFormat::Jpeg);
        let segment = |marker: u8, contents: &[u8]| {
            let mut s = vec![0xff, marker];
            s.extend_from_slice(&((contents.len() + 2) as u16).to_be_bytes());
            s.extend_from_slice(contents);
            s
        };
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend(sample_exif(orientation));
        let mut out = plain[..2].to_vec();
        out.extend(segment(0xe1, &exif));
        out.extend(segment(
            0xe1,
            b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>secret</x:xmpmeta>",
        ));
        out.extend(segment(0xed, b"Photoshop 3.0\0IPTC data"));
        out.extend(segment(0xfe, b"Shot at home"));
        out.extend(segment(0xe2, b"ICC_PROFILE\0\x01\x01fake profile"));
        out.extend_from_slice(&plain[2..]);
        out.extend_from_slice(b"hidden trailing data");
        out
    }

    #[test]
    fn reports_camera_date_and_location() {
        let report = read(&tagged_jpeg(1));
        assert_eq!(report.container, Container::Jpeg);
        let field = |name: &str| {
            report
                .fields
                .iter()
                .find(|f| f.name == name)
                .unwrap_or_else(|| panic!("{name} missing: {:?}", report.fields))
        };
        assert!(field("Make").value.contains("Toolbox Camera") && field("Make").sensitive);
        assert_eq!(field("GPSLatitude").group, "Location");
        let gps = report.gps.unwrap();
        assert!((gps.latitude - 38.71).abs() < 1e-6, "{gps:?}");
        assert!((gps.longitude + 9.14).abs() < 1e-6, "{gps:?}");
        assert_eq!(gps.altitude, Some(62.5));
        assert!(gps.maps_url().contains("mlat=38.710000&mlon=-9.140000"));
        for expected in [
            "XMP metadata",
            "IPTC / Photoshop metadata",
            "Comment: \"Shot at home\"",
            "20 bytes of extra data after the image",
        ] {
            assert!(
                report.other.iter().any(|o| o == expected),
                "{expected} missing: {:?}",
                report.other
            );
        }
        assert!(!report.is_clean());
        assert!(report.sensitive_count() >= 8);
    }

    #[test]
    fn finds_location_without_altitude() {
        let mut writer = Writer::new();
        let fields = [
            ExifField {
                tag: Tag::GPSLatitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"S".to_vec()]),
            },
            ExifField {
                tag: Tag::GPSLatitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational { num: 33, denom: 1 },
                    Rational { num: 30, denom: 1 },
                    Rational { num: 0, denom: 1 },
                ]),
            },
            ExifField {
                tag: Tag::GPSLongitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"E".to_vec()]),
            },
            ExifField {
                tag: Tag::GPSLongitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational { num: 151, denom: 1 },
                    Rational { num: 12, denom: 1 },
                    Rational { num: 0, denom: 1 },
                ]),
            },
        ];
        for field in &fields {
            writer.push_field(field);
        }
        let mut tiff = Cursor::new(Vec::new());
        writer.write(&mut tiff, false).unwrap();
        let plain = test_images::encoded(ImageFormat::Jpeg);
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend(tiff.into_inner());
        let mut jpeg = plain[..2].to_vec();
        jpeg.extend([0xff, 0xe1]);
        jpeg.extend(((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend(app1);
        jpeg.extend_from_slice(&plain[2..]);
        let gps = read(&jpeg)
            .gps
            .expect("a position without altitude is still a location");
        assert!(
            (gps.latitude + 33.5).abs() < 1e-9 && (gps.longitude - 151.2).abs() < 1e-9,
            "{gps:?}"
        );
        assert_eq!(gps.altitude, None);
    }

    #[test]
    fn strips_jpeg_losslessly() {
        let original = tagged_jpeg(1);
        let stripped = strip(&original).unwrap();
        assert!(stripped.bytes.len() < original.len());
        for expected in [
            "EXIF metadata",
            "XMP metadata",
            "IPTC / Photoshop metadata",
            "Comment",
        ] {
            assert!(
                stripped.removed.iter().any(|r| r == expected),
                "{expected}: {:?}",
                stripped.removed
            );
        }
        let after = read(&stripped.bytes);
        assert!(after.is_clean(), "{after:?}");
        assert_eq!(after.gps, None);
        // The picture itself is untouched.
        assert_eq!(
            decode(&stripped.bytes).unwrap().image,
            decode(&original).unwrap().image
        );
        // The color profile stays.
        assert!(stripped.bytes.windows(12).any(|w| w == b"ICC_PROFILE\0"));
    }

    #[test]
    fn keeps_orientation_when_rotated() {
        let stripped = strip(&tagged_jpeg(6)).unwrap();
        let after = read(&stripped.bytes);
        assert_eq!(after.orientation, 6);
        assert!(
            after.is_clean(),
            "only orientation may remain: {:?}",
            after.fields
        );
        // Decoders still rotate it: 40×30 stored, shown as 30×40.
        let image = decode(&stripped.bytes).unwrap().image;
        assert_eq!((image.width(), image.height()), (30, 40));
    }

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        png::write_chunk(&mut out, kind, data);
        out
    }

    #[test]
    fn strips_png_text_and_exif() {
        let plain = test_images::encoded(ImageFormat::Png);
        // Insert metadata chunks right after IHDR (8 + 25 bytes).
        let mut tagged = plain[..33].to_vec();
        tagged.extend(png_chunk(b"tEXt", b"parameters\0a castle, by a prompt"));
        tagged.extend(png_chunk(b"eXIf", &sample_exif(1)));
        tagged.extend(png_chunk(b"tIME", &[7, 232, 5, 15, 10, 30, 0]));
        tagged.extend_from_slice(&plain[33..]);

        let report = read(&tagged);
        assert!(report.gps.is_some(), "{report:?}");
        assert!(
            report
                .other
                .iter()
                .any(|o| o.starts_with("PNG text: parameters = \"a castle")),
            "{:?}",
            report.other
        );

        let stripped = strip(&tagged).unwrap();
        assert_eq!(
            stripped.removed,
            ["PNG text", "EXIF metadata", "Last-modified time"]
        );
        assert!(read(&stripped.bytes).is_clean());
        assert_eq!(stripped.bytes, plain);
    }

    #[test]
    fn strips_webp_and_fixes_flags() {
        let plain = test_images::encoded(ImageFormat::WebP);
        let image_chunk = &plain[12..]; // the VP8L chunk
        let mut vp8x = vec![0x10 | 0x08 | 0x04, 0, 0, 0]; // alpha + EXIF + XMP
        vp8x.extend_from_slice(&[39, 0, 0, 29, 0, 0]); // canvas 40×30, stored minus one
        let mut body = b"WEBP".to_vec();
        webp_chunk(&mut body, b"VP8X", &vp8x);
        body.extend_from_slice(image_chunk);
        webp_chunk(&mut body, b"EXIF", &sample_exif(1));
        webp_chunk(&mut body, b"XMP ", b"<x:xmpmeta/>");
        let mut tagged = b"RIFF".to_vec();
        tagged.extend_from_slice(&(body.len() as u32).to_le_bytes());
        tagged.extend(body);

        let report = read(&tagged);
        assert!(report.gps.is_some(), "{report:?}");
        assert_eq!(report.other, ["XMP metadata"]);

        let stripped = strip(&tagged).unwrap();
        assert_eq!(stripped.removed, ["EXIF metadata", "XMP metadata"]);
        let bytes = &stripped.bytes;
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8
        );
        // Only the metadata flags were cleared; the alpha flag stays.
        assert_eq!(bytes[20], 0x10);
        assert!(read(bytes).is_clean());
        assert_eq!(decode(bytes).unwrap().image, decode(&plain).unwrap().image);
    }

    fn webp_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(id);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(0);
        }
    }

    #[test]
    fn rejects_other_formats() {
        assert!(strip(&test_images::encoded(ImageFormat::Gif)).is_err());
        assert!(strip(b"\xff\xd8\xff\xe1").is_err());
    }
}
