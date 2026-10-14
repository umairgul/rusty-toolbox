use toolbox_core::formats::{self, Format, Options};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum DataFormat {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl From<DataFormat> for Format {
    fn from(format: DataFormat) -> Self {
        match format {
            DataFormat::Json => Format::Json,
            DataFormat::Yaml => Format::Yaml,
            DataFormat::Toml => Format::Toml,
            DataFormat::Csv => Format::Csv,
        }
    }
}

impl From<Format> for DataFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Json => DataFormat::Json,
            Format::Yaml => DataFormat::Yaml,
            Format::Toml => DataFormat::Toml,
            Format::Csv => DataFormat::Csv,
        }
    }
}

#[derive(uniffi::Record)]
pub struct DataFormatInfo {
    /// e.g. "YAML".
    pub label: String,
    /// e.g. "yaml", without the dot.
    pub extension: String,
}

/// Every format, in display order.
#[uniffi::export]
pub fn data_formats_all() -> Vec<DataFormat> {
    Format::ALL.into_iter().map(Into::into).collect()
}

#[uniffi::export]
pub fn data_format_info(format: DataFormat) -> DataFormatInfo {
    let format = Format::from(format);
    DataFormatInfo {
        label: format.label().into(),
        extension: format.extension().into(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, uniffi::Record)]
pub struct DataFormatsOptions {
    /// JSON output: indented instead of on one line.
    pub pretty: bool,
    /// CSV input: read `42`, `3.5`, `true` and `false` as numbers and booleans.
    pub infer_csv_types: bool,
}

impl From<DataFormatsOptions> for Options {
    fn from(options: DataFormatsOptions) -> Self {
        Options {
            pretty: options.pretty,
            infer_csv_types: options.infer_csv_types,
        }
    }
}

#[uniffi::export]
pub fn data_formats_default_options() -> DataFormatsOptions {
    let Options {
        pretty,
        infer_csv_types,
    } = Options::default();
    DataFormatsOptions {
        pretty,
        infer_csv_types,
    }
}

#[derive(uniffi::Enum)]
pub enum DataFormatsOutcome {
    Ok { output: String },
    Invalid { message: String },
}

#[derive(uniffi::Record)]
pub struct DataFormatsConversion {
    /// What the input looks like, whether or not it was read that way.
    pub detected: DataFormat,
    /// The format the input was read as: `from`, or `detected` when `from` is `None`.
    pub from: DataFormat,
    pub outcome: DataFormatsOutcome,
}

/// Converts `input` to `to`. With `from` as `None`, the input format is detected. Slow for
/// large inputs: call it off the main thread.
#[uniffi::export]
pub fn data_formats_convert(
    input: &str,
    from: Option<DataFormat>,
    to: DataFormat,
    options: DataFormatsOptions,
) -> DataFormatsConversion {
    let detected = formats::detect(input);
    let source = from.map_or(detected, Format::from);
    let outcome = match formats::convert(input, source, to.into(), &options.into()) {
        Ok(output) => DataFormatsOutcome::Ok { output },
        Err(error) => DataFormatsOutcome::Invalid {
            message: error.to_string(),
        },
    };
    DataFormatsConversion {
        detected: detected.into(),
        from: source.into(),
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_when_from_is_none() {
        let conversion = data_formats_convert(
            "a: 1",
            None,
            DataFormat::Json,
            DataFormatsOptions {
                pretty: false,
                infer_csv_types: true,
            },
        );
        assert_eq!(conversion.detected, DataFormat::Yaml);
        assert_eq!(conversion.from, DataFormat::Yaml);
        assert!(
            matches!(conversion.outcome, DataFormatsOutcome::Ok { output } if output == r#"{"a":1}"#)
        );
    }

    #[test]
    fn forced_format_reports_errors() {
        let conversion = data_formats_convert(
            "a: 1",
            Some(DataFormat::Json),
            DataFormat::Yaml,
            data_formats_default_options(),
        );
        assert_eq!(conversion.detected, DataFormat::Yaml);
        assert_eq!(conversion.from, DataFormat::Json);
        assert!(
            matches!(conversion.outcome, DataFormatsOutcome::Invalid { message } if message.starts_with("Invalid JSON"))
        );
    }
}
