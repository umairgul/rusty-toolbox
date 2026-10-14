//! Random passwords and passphrases, with an entropy-based strength rating.

use crate::ToolError;
use toolbox_core::passphrase::{self, Separator};
use toolbox_core::password;
use toolbox_core::strength::Strength;

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum PasswordStrength {
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

impl From<Strength> for PasswordStrength {
    fn from(strength: Strength) -> Self {
        match strength {
            Strength::Weak => PasswordStrength::Weak,
            Strength::Fair => PasswordStrength::Fair,
            Strength::Strong => PasswordStrength::Strong,
            Strength::VeryStrong => PasswordStrength::VeryStrong,
        }
    }
}

/// How strong a secret made with some options is.
#[derive(uniffi::Record)]
pub struct PasswordStrengthInfo {
    pub strength: PasswordStrength,
    /// e.g. "Very strong".
    pub label: String,
    /// Bits of entropy.
    pub bits: f64,
}

impl PasswordStrengthInfo {
    fn new(strength: Strength, bits: f64) -> Self {
        PasswordStrengthInfo {
            strength: strength.into(),
            label: strength.label().into(),
            bits,
        }
    }
}

// ---- Passwords ----------------------------------------------------------------------------

#[derive(uniffi::Record)]
pub struct PasswordOptions {
    pub length: u32,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    /// Leave out characters that are easy to confuse: 0 O 1 l I |.
    pub exclude_ambiguous: bool,
}

impl From<password::Options> for PasswordOptions {
    fn from(options: password::Options) -> Self {
        PasswordOptions {
            length: options.length as u32,
            lowercase: options.lowercase,
            uppercase: options.uppercase,
            digits: options.digits,
            symbols: options.symbols,
            exclude_ambiguous: options.exclude_ambiguous,
        }
    }
}

impl From<PasswordOptions> for password::Options {
    fn from(options: PasswordOptions) -> Self {
        password::Options {
            length: options.length as usize,
            lowercase: options.lowercase,
            uppercase: options.uppercase,
            digits: options.digits,
            symbols: options.symbols,
            exclude_ambiguous: options.exclude_ambiguous,
        }
    }
}

#[uniffi::export]
pub fn password_default_options() -> PasswordOptions {
    password::Options::default().into()
}

#[uniffi::export]
pub fn password_min_length() -> u32 {
    password::MIN_LENGTH as u32
}

#[uniffi::export]
pub fn password_max_length() -> u32 {
    password::MAX_LENGTH as u32
}

/// A random password with at least one character from every enabled set. Fails when no set
/// is enabled or the length is out of range.
#[uniffi::export]
pub fn password_generate(options: PasswordOptions) -> Result<String, ToolError> {
    password::generate(&options.into()).map_err(ToolError::new)
}

#[uniffi::export]
pub fn password_strength(options: PasswordOptions) -> PasswordStrengthInfo {
    let options = password::Options::from(options);
    PasswordStrengthInfo::new(options.strength(), options.entropy_bits())
}

// ---- Passphrases --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum PassphraseSeparator {
    Hyphen,
    Space,
    Period,
    Underscore,
    /// Words run together. Not named `None`, which clashes with `Optional.none` in Swift.
    NoSeparator,
}

impl From<PassphraseSeparator> for Separator {
    fn from(separator: PassphraseSeparator) -> Self {
        match separator {
            PassphraseSeparator::Hyphen => Separator::Hyphen,
            PassphraseSeparator::Space => Separator::Space,
            PassphraseSeparator::Period => Separator::Period,
            PassphraseSeparator::Underscore => Separator::Underscore,
            PassphraseSeparator::NoSeparator => Separator::None,
        }
    }
}

impl From<Separator> for PassphraseSeparator {
    fn from(separator: Separator) -> Self {
        match separator {
            Separator::Hyphen => PassphraseSeparator::Hyphen,
            Separator::Space => PassphraseSeparator::Space,
            Separator::Period => PassphraseSeparator::Period,
            Separator::Underscore => PassphraseSeparator::Underscore,
            Separator::None => PassphraseSeparator::NoSeparator,
        }
    }
}

/// e.g. "Hyphen ( - )".
#[uniffi::export]
pub fn passphrase_separator_label(separator: PassphraseSeparator) -> String {
    Separator::from(separator).label().into()
}

#[derive(uniffi::Record)]
pub struct PassphraseOptions {
    pub words: u32,
    pub separator: PassphraseSeparator,
    pub capitalize: bool,
    /// Append one random digit to one random word.
    pub include_number: bool,
}

impl From<passphrase::Options> for PassphraseOptions {
    fn from(options: passphrase::Options) -> Self {
        PassphraseOptions {
            words: options.words as u32,
            separator: options.separator.into(),
            capitalize: options.capitalize,
            include_number: options.include_number,
        }
    }
}

impl From<PassphraseOptions> for passphrase::Options {
    fn from(options: PassphraseOptions) -> Self {
        passphrase::Options {
            words: options.words as usize,
            separator: options.separator.into(),
            capitalize: options.capitalize,
            include_number: options.include_number,
        }
    }
}

#[uniffi::export]
pub fn passphrase_default_options() -> PassphraseOptions {
    passphrase::Options::default().into()
}

#[uniffi::export]
pub fn passphrase_min_words() -> u32 {
    passphrase::MIN_WORDS as u32
}

#[uniffi::export]
pub fn passphrase_max_words() -> u32 {
    passphrase::MAX_WORDS as u32
}

/// How many words passphrases are picked from. The core keeps its word list private, so this
/// is worked out from the entropy of a one-word passphrase (`log2(list size)`).
#[uniffi::export]
pub fn passphrase_word_list_size() -> u32 {
    let one_word = passphrase::Options {
        words: 1,
        include_number: false,
        ..Default::default()
    };
    one_word.entropy_bits().exp2().round() as u32
}

/// A passphrase of random words from the EFF long word list.
#[uniffi::export]
pub fn passphrase_generate(options: PassphraseOptions) -> String {
    passphrase::generate(&options.into())
}

#[uniffi::export]
pub fn passphrase_strength(options: PassphraseOptions) -> PasswordStrengthInfo {
    let options = passphrase::Options::from(options);
    PasswordStrengthInfo::new(options.strength(), options.entropy_bits())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_list_size_matches_the_eff_list() {
        assert_eq!(passphrase_word_list_size(), 7772);
    }

    #[test]
    fn separators_round_trip() {
        for separator in Separator::ALL {
            assert_eq!(
                Separator::from(PassphraseSeparator::from(separator)),
                separator
            );
        }
    }

    #[test]
    fn options_round_trip() {
        let options = password::Options {
            length: 33,
            symbols: false,
            exclude_ambiguous: true,
            ..Default::default()
        };
        assert_eq!(
            password::Options::from(PasswordOptions::from(options.clone())),
            options
        );
        let options = passphrase::Options {
            words: 9,
            separator: Separator::Underscore,
            capitalize: true,
            include_number: true,
        };
        assert_eq!(
            passphrase::Options::from(PassphraseOptions::from(options.clone())),
            options
        );
    }

    #[test]
    fn generates_and_rates() {
        let password = password_generate(password_default_options()).unwrap();
        assert_eq!(password.chars().count(), 20);
        let mut none = password_default_options();
        none.lowercase = false;
        none.uppercase = false;
        none.digits = false;
        none.symbols = false;
        assert!(password_generate(none).is_err());

        let strength = passphrase_strength(passphrase_default_options());
        assert!(strength.strength == PasswordStrength::Strong);
        assert_eq!(strength.label, "Strong");
    }
}
