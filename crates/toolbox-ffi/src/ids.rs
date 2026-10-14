//! UUID v4 and v7, ULID and Nano ID generation.

use crate::ToolError;
use toolbox_core::ids::{self, Kind, MAX_COUNT, MAX_NANOID_LENGTH, NANOID_ALPHABET};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum IdsKind {
    UuidV4,
    UuidV7,
    Ulid,
    NanoId,
}

impl From<IdsKind> for Kind {
    fn from(kind: IdsKind) -> Self {
        match kind {
            IdsKind::UuidV4 => Kind::UuidV4,
            IdsKind::UuidV7 => Kind::UuidV7,
            IdsKind::Ulid => Kind::Ulid,
            IdsKind::NanoId => Kind::NanoId,
        }
    }
}

impl From<Kind> for IdsKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::UuidV4 => IdsKind::UuidV4,
            Kind::UuidV7 => IdsKind::UuidV7,
            Kind::Ulid => IdsKind::Ulid,
            Kind::NanoId => IdsKind::NanoId,
        }
    }
}

#[derive(uniffi::Record)]
pub struct IdsKindInfo {
    /// e.g. "UUID v7".
    pub label: String,
    /// One sentence on what the ID is good for.
    pub description: String,
}

#[uniffi::export]
pub fn ids_kind_info(kind: IdsKind) -> IdsKindInfo {
    let kind = Kind::from(kind);
    IdsKindInfo {
        label: kind.label().into(),
        description: kind.description().into(),
    }
}

#[derive(uniffi::Record)]
pub struct IdsOptions {
    pub kind: IdsKind,
    pub count: u32,
    pub uuid_uppercase: bool,
    pub uuid_hyphens: bool,
    pub ulid_lowercase: bool,
    pub nanoid_length: u32,
    pub nanoid_alphabet: String,
}

impl From<ids::Options> for IdsOptions {
    fn from(options: ids::Options) -> Self {
        IdsOptions {
            kind: options.kind.into(),
            count: options.count as u32,
            uuid_uppercase: options.uuid_uppercase,
            uuid_hyphens: options.uuid_hyphens,
            ulid_lowercase: options.ulid_lowercase,
            nanoid_length: options.nanoid_length as u32,
            nanoid_alphabet: options.nanoid_alphabet,
        }
    }
}

impl From<IdsOptions> for ids::Options {
    fn from(options: IdsOptions) -> Self {
        ids::Options {
            kind: options.kind.into(),
            count: options.count as usize,
            uuid_uppercase: options.uuid_uppercase,
            uuid_hyphens: options.uuid_hyphens,
            ulid_lowercase: options.ulid_lowercase,
            nanoid_length: options.nanoid_length as usize,
            nanoid_alphabet: options.nanoid_alphabet,
        }
    }
}

#[uniffi::export]
pub fn ids_default_options() -> IdsOptions {
    ids::Options::default().into()
}

#[uniffi::export]
pub fn ids_max_count() -> u32 {
    MAX_COUNT as u32
}

#[uniffi::export]
pub fn ids_max_nanoid_length() -> u32 {
    MAX_NANOID_LENGTH as u32
}

/// A named Nano ID alphabet, e.g. "Hex".
#[derive(uniffi::Record)]
pub struct IdsAlphabetPreset {
    pub name: String,
    pub alphabet: String,
}

/// Common Nano ID alphabets, starting with Nano ID's URL-safe default.
#[uniffi::export]
pub fn ids_nanoid_alphabet_presets() -> Vec<IdsAlphabetPreset> {
    [
        ("URL-safe (default)", NANOID_ALPHABET),
        (
            "Alphanumeric",
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        ),
        ("Lowercase + digits", "0123456789abcdefghijklmnopqrstuvwxyz"),
        ("Hex", "0123456789abcdef"),
        ("Digits", "0123456789"),
    ]
    .into_iter()
    .map(|(name, alphabet)| IdsAlphabetPreset {
        name: name.into(),
        alphabet: alphabet.into(),
    })
    .collect()
}

/// `options.count` IDs. Fails when the count, Nano ID length or alphabet is invalid.
#[uniffi::export]
pub fn ids_generate(options: IdsOptions) -> Result<Vec<String>, ToolError> {
    ids::generate(&options.into()).map_err(ToolError::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_round_trip() {
        let options = ids::Options {
            kind: Kind::NanoId,
            count: 12,
            uuid_uppercase: true,
            uuid_hyphens: false,
            ulid_lowercase: true,
            nanoid_length: 9,
            nanoid_alphabet: "abc".into(),
        };
        assert_eq!(
            ids::Options::from(IdsOptions::from(options.clone())),
            options
        );
        for kind in Kind::ALL {
            assert_eq!(Kind::from(IdsKind::from(kind)), kind);
        }
    }

    #[test]
    fn presets_generate() {
        let presets = ids_nanoid_alphabet_presets();
        assert_eq!(presets[0].alphabet, NANOID_ALPHABET);
        for preset in presets {
            let options = IdsOptions {
                kind: IdsKind::NanoId,
                count: 3,
                nanoid_alphabet: preset.alphabet.clone(),
                ..ids_default_options()
            };
            let ids = ids_generate(options).unwrap();
            assert_eq!(ids.len(), 3);
            assert!(
                ids.iter()
                    .flat_map(|id| id.chars())
                    .all(|c| preset.alphabet.contains(c))
            );
        }
    }

    #[test]
    fn reports_invalid_options() {
        let options = IdsOptions {
            count: 0,
            ..ids_default_options()
        };
        assert!(ids_generate(options).is_err());
    }
}
