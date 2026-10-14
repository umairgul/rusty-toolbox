use toolbox_core::json_format::{self, Indent, SyntaxError};

#[derive(Clone, Copy, uniffi::Enum)]
pub enum JsonIndent {
    TwoSpaces,
    FourSpaces,
    Tab,
}

impl From<JsonIndent> for Indent {
    fn from(indent: JsonIndent) -> Self {
        match indent {
            JsonIndent::TwoSpaces => Indent::TwoSpaces,
            JsonIndent::FourSpaces => Indent::FourSpaces,
            JsonIndent::Tab => Indent::Tab,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum JsonMode {
    Format { indent: JsonIndent, sort_keys: bool },
    Minify,
}

/// Why the input isn't valid JSON. `line` and `column` start at 1, and are 0 when the error
/// has no position (e.g. empty input).
#[derive(uniffi::Record)]
pub struct JsonSyntaxError {
    pub line: u32,
    pub column: u32,
    /// e.g. "Line 3, column 7: expected `,` or `}`".
    pub message: String,
    /// The offending line with a `^` under the error column.
    pub context: Option<String>,
}

#[derive(uniffi::Enum)]
pub enum JsonOutcome {
    Ok { output: String },
    Invalid { error: JsonSyntaxError },
}

#[uniffi::export]
pub fn json_process(input: &str, mode: JsonMode) -> JsonOutcome {
    let result = match mode {
        JsonMode::Minify => json_format::minify(input),
        JsonMode::Format {
            indent,
            sort_keys: true,
        } => json_format::format_sorted(input, indent.into()),
        JsonMode::Format { indent, .. } => json_format::format(input, indent.into()),
    };
    match result {
        Ok(output) => JsonOutcome::Ok { output },
        Err(error) => JsonOutcome::Invalid {
            error: syntax_error(&error, input),
        },
    }
}

fn syntax_error(error: &SyntaxError, input: &str) -> JsonSyntaxError {
    JsonSyntaxError {
        line: error.line as u32,
        column: error.column as u32,
        message: error.to_string(),
        context: error.context(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(outcome: JsonOutcome) -> String {
        match outcome {
            JsonOutcome::Ok { output } => output,
            JsonOutcome::Invalid { error } => panic!("unexpected error: {}", error.message),
        }
    }

    #[test]
    fn formats_minifies_and_sorts() {
        let input = r#"{"b":1.50,"a":[1,2]}"#;
        let format = |sort_keys| JsonMode::Format {
            indent: JsonIndent::TwoSpaces,
            sort_keys,
        };
        assert_eq!(
            output(json_process(input, format(false))),
            "{\n  \"b\": 1.50,\n  \"a\": [\n    1,\n    2\n  ]\n}"
        );
        assert!(output(json_process(input, format(true))).starts_with("{\n  \"a\""));
        assert_eq!(
            output(json_process(" { \"a\" : 1 } ", JsonMode::Minify)),
            r#"{"a":1}"#
        );
    }

    #[test]
    fn reports_position_and_context() {
        let JsonOutcome::Invalid { error } = json_process("{\n  \"a\": tru\n}", JsonMode::Minify)
        else {
            panic!("expected an error");
        };
        assert_eq!(error.line, 2);
        assert!(error.message.starts_with("Line 2, column"));
        assert!(error.context.unwrap().starts_with("  \"a\": tru\n"));
    }

    #[test]
    fn empty_input_has_no_position() {
        let JsonOutcome::Invalid { error } = json_process("  ", JsonMode::Minify) else {
            panic!("expected an error");
        };
        assert_eq!((error.line, error.column), (0, 0));
        assert_eq!(error.context, None);
    }
}
