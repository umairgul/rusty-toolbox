use toolbox_core::encoding::{self, Codec, Error, Options};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum EncodeCodec {
    Base64,
    Url,
    Html,
    Hex,
}

impl From<EncodeCodec> for Codec {
    fn from(codec: EncodeCodec) -> Self {
        match codec {
            EncodeCodec::Base64 => Codec::Base64,
            EncodeCodec::Url => Codec::Url,
            EncodeCodec::Html => Codec::Html,
            EncodeCodec::Hex => Codec::Hex,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum EncodeDirection {
    Encode,
    Decode,
}

/// Each option applies to one codec only; the others ignore it.
#[derive(Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct EncodeOptions {
    /// Base64: use the URL-safe alphabet (`-` and `_` instead of `+` and `/`).
    pub base64_url_safe: bool,
    /// Base64: end with `=` padding when encoding.
    pub base64_padding: bool,
    /// URL: encode spaces as `+` and decode `+` as a space, as HTML forms do.
    pub url_plus_for_space: bool,
    /// HTML: also encode every non-ASCII character as a numeric entity.
    pub html_encode_non_ascii: bool,
}

impl From<Options> for EncodeOptions {
    fn from(options: Options) -> Self {
        EncodeOptions {
            base64_url_safe: options.base64_url_safe,
            base64_padding: options.base64_padding,
            url_plus_for_space: options.url_plus_for_space,
            html_encode_non_ascii: options.html_encode_non_ascii,
        }
    }
}

impl From<EncodeOptions> for Options {
    fn from(options: EncodeOptions) -> Self {
        Options {
            base64_url_safe: options.base64_url_safe,
            base64_padding: options.base64_padding,
            url_plus_for_space: options.url_plus_for_space,
            html_encode_non_ascii: options.html_encode_non_ascii,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum EncodeOutcome {
    Ok {
        output: String,
    },
    /// Decoding worked, but the bytes aren't UTF-8 text, so they're shown as hex instead.
    NotText {
        message: String,
        hex: String,
    },
    Invalid {
        message: String,
    },
}

/// e.g. "Base64" or "HTML entities".
#[uniffi::export]
pub fn encode_codec_label(codec: EncodeCodec) -> String {
    Codec::from(codec).label().into()
}

#[uniffi::export]
pub fn encode_default_options() -> EncodeOptions {
    Options::default().into()
}

/// Encodes or decodes `input`. Can be slow for very large input: call it off the main thread.
#[uniffi::export]
pub fn encode_process(
    codec: EncodeCodec,
    direction: EncodeDirection,
    input: &str,
    options: EncodeOptions,
) -> EncodeOutcome {
    let (codec, options) = (codec.into(), options.into());
    let result = match direction {
        EncodeDirection::Encode => Ok(encoding::encode(codec, input, &options)),
        EncodeDirection::Decode => encoding::decode(codec, input, &options),
    };
    let error = match result {
        Ok(output) => return EncodeOutcome::Ok { output },
        Err(error) => error,
    };
    let message = error.to_string();
    match error {
        Error::NotText { hex, .. } => EncodeOutcome::NotText { message, hex },
        _ => EncodeOutcome::Invalid { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_binary_as_hex() {
        let outcome = encode_process(
            EncodeCodec::Base64,
            EncodeDirection::Decode,
            "/w==",
            encode_default_options(),
        );
        let EncodeOutcome::NotText { message, hex } = outcome else {
            panic!("expected NotText")
        };
        assert_eq!(hex, "ff");
        assert_eq!(message, "Decoded 1 bytes of binary data, not text");
    }

    #[test]
    fn passes_options_through() {
        let options = EncodeOptions {
            url_plus_for_space: true,
            ..encode_default_options()
        };
        let outcome = encode_process(EncodeCodec::Url, EncodeDirection::Encode, "a b", options);
        assert!(matches!(outcome, EncodeOutcome::Ok { output } if output == "a+b"));
        let outcome = encode_process(
            EncodeCodec::Hex,
            EncodeDirection::Decode,
            "zz",
            encode_default_options(),
        );
        assert!(matches!(outcome, EncodeOutcome::Invalid { .. }));
    }
}
