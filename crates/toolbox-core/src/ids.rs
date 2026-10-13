//! Unique ID generator: UUID v4 and v7, ULID and Nano ID.

use rand::{CryptoRng, RngExt};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_COUNT: usize = 1000;
pub const MAX_NANOID_LENGTH: usize = 255;

/// Nano ID's default URL-safe alphabet.
pub const NANOID_ALPHABET: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
/// Crockford's base32, used by ULID: no I, L, O or U.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    UuidV4,
    UuidV7,
    Ulid,
    NanoId,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::UuidV4, Kind::UuidV7, Kind::Ulid, Kind::NanoId];

    pub fn label(self) -> &'static str {
        match self {
            Kind::UuidV4 => "UUID v4",
            Kind::UuidV7 => "UUID v7",
            Kind::Ulid => "ULID",
            Kind::NanoId => "Nano ID",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Kind::UuidV4 => "Fully random. The most widely supported UUID.",
            Kind::UuidV7 => {
                "Starts with a timestamp, so IDs sort by creation time. Good database keys."
            }
            Kind::Ulid => "26 characters, sortable by creation time, case-insensitive.",
            Kind::NanoId => "Short, URL-safe random IDs with a configurable length and alphabet.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub kind: Kind,
    pub count: usize,
    pub uuid_uppercase: bool,
    pub uuid_hyphens: bool,
    pub ulid_lowercase: bool,
    pub nanoid_length: usize,
    pub nanoid_alphabet: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            kind: Kind::UuidV4,
            count: 1,
            uuid_uppercase: false,
            uuid_hyphens: true,
            ulid_lowercase: false,
            nanoid_length: 21,
            nanoid_alphabet: NANOID_ALPHABET.to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidCount,
    InvalidNanoIdLength,
    AlphabetTooSmall,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidCount => write!(f, "Count must be between 1 and {MAX_COUNT}"),
            Error::InvalidNanoIdLength => {
                write!(f, "Length must be between 1 and {MAX_NANOID_LENGTH}")
            }
            Error::AlphabetTooSmall => {
                write!(f, "The alphabet needs at least 2 different characters")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Generates `options.count` IDs using the thread-local cryptographically secure RNG.
pub fn generate(options: &Options) -> Result<Vec<String>, Error> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    generate_with(options, now_ms, &mut rand::rng())
}

/// Like [`generate`], with the ULID clock time (milliseconds since the Unix epoch) and RNG supplied.
pub fn generate_with(
    options: &Options,
    now_ms: u64,
    rng: &mut impl CryptoRng,
) -> Result<Vec<String>, Error> {
    if !(1..=MAX_COUNT).contains(&options.count) {
        return Err(Error::InvalidCount);
    }
    match options.kind {
        Kind::UuidV4 | Kind::UuidV7 => Ok((0..options.count)
            .map(|_| {
                let id = if options.kind == Kind::UuidV4 {
                    uuid::Uuid::new_v4()
                } else {
                    uuid::Uuid::now_v7()
                };
                let text = if options.uuid_hyphens {
                    id.hyphenated().to_string()
                } else {
                    id.simple().to_string()
                };
                if options.uuid_uppercase {
                    text.to_uppercase()
                } else {
                    text
                }
            })
            .collect()),
        Kind::Ulid => {
            // Within one millisecond, ULIDs increment the random part so they still sort in order.
            let mut random: u128 = rng.random::<u128>() >> 48;
            Ok((0..options.count)
                .map(|i| {
                    if i > 0 {
                        random = (random + 1) & ((1 << 80) - 1);
                    }
                    let text = encode_ulid(now_ms, random);
                    if options.ulid_lowercase {
                        text.to_lowercase()
                    } else {
                        text
                    }
                })
                .collect())
        }
        Kind::NanoId => {
            if !(1..=MAX_NANOID_LENGTH).contains(&options.nanoid_length) {
                return Err(Error::InvalidNanoIdLength);
            }
            let mut alphabet: Vec<char> = options.nanoid_alphabet.chars().collect();
            alphabet.sort_unstable();
            alphabet.dedup();
            if alphabet.len() < 2 {
                return Err(Error::AlphabetTooSmall);
            }
            Ok((0..options.count)
                .map(|_| {
                    (0..options.nanoid_length)
                        .map(|_| alphabet[rng.random_range(0..alphabet.len())])
                        .collect()
                })
                .collect())
        }
    }
}

/// Encodes a 48-bit millisecond timestamp and 80 random bits as a 26-character ULID.
fn encode_ulid(timestamp_ms: u64, random: u128) -> String {
    let value = (u128::from(timestamp_ms & 0xFFFF_FFFF_FFFF) << 80) | (random & ((1 << 80) - 1));
    (0..26)
        .map(|i| CROCKFORD[((value >> (125 - 5 * i)) & 31) as usize] as char)
        .collect()
}

/// The timestamp (milliseconds since the Unix epoch) stored in a ULID, if it is valid.
pub fn ulid_timestamp_ms(ulid: &str) -> Option<u64> {
    if ulid.len() != 26 {
        return None;
    }
    let mut value: u128 = 0;
    for c in ulid.bytes() {
        let digit = CROCKFORD
            .iter()
            .position(|&d| d == c.to_ascii_uppercase())?;
        value = (value << 5) | digit as u128;
    }
    Some((value >> 80) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn options(kind: Kind, count: usize) -> Options {
        Options {
            kind,
            count,
            ..Default::default()
        }
    }

    #[test]
    fn uuid_formats() {
        let v4 = &generate(&options(Kind::UuidV4, 1)).unwrap()[0];
        assert_eq!(v4.len(), 36);
        assert_eq!(&v4[14..15], "4", "{v4}");
        assert!(matches!(&v4[19..20], "8" | "9" | "a" | "b"), "{v4}");

        let v7 = &generate(&options(Kind::UuidV7, 1)).unwrap()[0];
        assert_eq!(&v7[14..15], "7", "{v7}");

        let plain = &generate(&Options {
            uuid_hyphens: false,
            uuid_uppercase: true,
            ..options(Kind::UuidV4, 1)
        })
        .unwrap()[0];
        assert_eq!(plain.len(), 32);
        assert!(
            plain
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase()),
            "{plain}"
        );
    }

    #[test]
    fn uuid_v7_sorts_by_creation() {
        let ids = generate(&options(Kind::UuidV7, 500)).unwrap();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn ulid_encodes_timestamp_and_sorts() {
        // Example from the ULID spec: 01ARZ3NDEK is 1469922850259 ms.
        let now = 1_469_922_850_259;
        let ids = generate_with(
            &options(Kind::Ulid, 200),
            now,
            &mut StdRng::seed_from_u64(5),
        )
        .unwrap();
        for id in &ids {
            assert_eq!(id.len(), 26);
            assert!(id.starts_with("01ARZ3NDEK"), "{id}");
            assert_eq!(ulid_timestamp_ms(id), Some(now));
        }
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            encode_ulid((1 << 48) - 1, (1 << 80) - 1),
            "7ZZZZZZZZZZZZZZZZZZZZZZZZZ"
        );
    }

    #[test]
    fn nanoid_uses_length_and_alphabet() {
        let custom = Options {
            nanoid_length: 10,
            nanoid_alphabet: "abc".into(),
            ..options(Kind::NanoId, 50)
        };
        for id in generate(&custom).unwrap() {
            assert_eq!(id.len(), 10);
            assert!(id.chars().all(|c| "abc".contains(c)), "{id}");
        }
        let default = &generate(&options(Kind::NanoId, 1)).unwrap()[0];
        assert_eq!(default.len(), 21);
        assert!(default.chars().all(|c| NANOID_ALPHABET.contains(c)));
    }

    #[test]
    fn rejects_invalid_options() {
        assert_eq!(
            generate(&options(Kind::UuidV4, 0)),
            Err(Error::InvalidCount)
        );
        assert_eq!(
            generate(&options(Kind::UuidV4, MAX_COUNT + 1)),
            Err(Error::InvalidCount)
        );
        let bad = Options {
            nanoid_alphabet: "aaaa".into(),
            ..options(Kind::NanoId, 1)
        };
        assert_eq!(generate(&bad), Err(Error::AlphabetTooSmall));
        let bad = Options {
            nanoid_length: 0,
            ..options(Kind::NanoId, 1)
        };
        assert_eq!(generate(&bad), Err(Error::InvalidNanoIdLength));
    }
}
