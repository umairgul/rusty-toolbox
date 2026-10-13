//! JSON formatter, minifier and validator.
//!
//! Formatting and minifying re-indent the original text token by token instead of parsing
//! and re-serializing it, so numbers (`1.50`, `12345678901234567890123`), string escapes,
//! key order and duplicate keys come out exactly as written. Only sorting keys parses.

use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indent {
    TwoSpaces,
    FourSpaces,
    Tab,
}

impl Indent {
    pub const ALL: [Indent; 3] = [Indent::TwoSpaces, Indent::FourSpaces, Indent::Tab];

    pub fn label(self) -> &'static str {
        match self {
            Indent::TwoSpaces => "2 spaces",
            Indent::FourSpaces => "4 spaces",
            Indent::Tab => "Tabs",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Indent::TwoSpaces => "  ",
            Indent::FourSpaces => "    ",
            Indent::Tab => "\t",
        }
    }
}

/// Where and why the input isn't valid JSON. Line and column start at 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(
                f,
                "Line {}, column {}: {}",
                self.line, self.column, self.message
            )
        }
    }
}

impl std::error::Error for SyntaxError {}

impl SyntaxError {
    /// The offending line of `input` with a `^` under the error column.
    pub fn context(&self, input: &str) -> Option<String> {
        let line = input.lines().nth(self.line.checked_sub(1)?)?;
        let caret_at = line
            .chars()
            .take(self.column.saturating_sub(1))
            .map(|c| if c == '\t' { '\t' } else { ' ' });
        Some(format!("{line}\n{}^", caret_at.collect::<String>()))
    }
}

/// Checks that `input` is exactly one valid JSON value.
pub fn validate(input: &str) -> Result<(), SyntaxError> {
    if input.trim().is_empty() {
        return Err(SyntaxError {
            line: 0,
            column: 0,
            message: "Paste some JSON".into(),
        });
    }
    serde_json::from_str::<serde::de::IgnoredAny>(input)
        .map(|_| ())
        .map_err(|error| {
            // serde_json messages end with " at line L column C", which is shown separately.
            let message = error.to_string();
            let message = message.split(" at line ").next().unwrap_or(&message);
            let (mut line, mut column) = (error.line(), error.column());
            // Column 0 means the error was found on reading a line break: it belongs at the end
            // of the previous line.
            if column == 0 && line > 1 {
                line -= 1;
                column = input
                    .lines()
                    .nth(line - 1)
                    .map_or(1, |text| text.chars().count());
            }
            let message = if is_trailing_comma(input, line, column) {
                "Trailing comma: JSON doesn't allow a comma before ] or }".to_owned()
            } else {
                friendly_message(message)
            };
            SyntaxError {
                line,
                column,
                message,
            }
        })
}

/// Whether the character at `line`/`column` (1-based) is `]` or `}` and the last
/// non-whitespace character before it is a comma. serde_json reports this as "expected value"
/// in arrays and "key must be a string" in objects.
fn is_trailing_comma(input: &str, line: usize, column: usize) -> bool {
    let Some(text) = input.lines().nth(line.saturating_sub(1)) else {
        return false;
    };
    let Some(closing) = text.chars().nth(column.saturating_sub(1)) else {
        return false;
    };
    if closing != ']' && closing != '}' {
        return false;
    }
    let line_start: usize = input.lines().take(line - 1).map(|l| l.len() + 1).sum();
    let offset = line_start + text.char_indices().nth(column - 1).map_or(0, |(i, _)| i);
    input[..offset].trim_end().ends_with(',')
}

/// Plain-English versions of serde_json's most common messages.
fn friendly_message(message: &str) -> String {
    match message {
        "expected ident" => {
            "Unknown word: only true, false and null can appear without quotes".into()
        }
        "key must be a string" => {
            "Expected a key in double quotes (JSON doesn't allow trailing commas)".into()
        }
        "trailing comma" => "Trailing comma: JSON doesn't allow a comma before ] or }".into(),
        "expected value" => {
            "Expected a value: a string, number, object, array, true, false or null".into()
        }
        "trailing characters" => "Unexpected text after the end of the JSON value".into(),
        message if message.starts_with("EOF while parsing") => {
            "The JSON ends too early: a bracket, brace or string isn't closed".into()
        }
        message => capitalize(message),
    }
}

/// Pretty-prints `input`, keeping every token exactly as written.
pub fn format(input: &str, indent: Indent) -> Result<String, SyntaxError> {
    validate(input)?;
    Ok(reflow(input, Some(indent.as_str())))
}

/// Removes all whitespace outside strings.
pub fn minify(input: &str) -> Result<String, SyntaxError> {
    validate(input)?;
    Ok(reflow(input, None))
}

/// Pretty-prints with object keys sorted alphabetically at every level. This parses the
/// JSON, so integers beyond 64 bits and floats are written back in standard form.
pub fn format_sorted(input: &str, indent: Indent) -> Result<String, SyntaxError> {
    validate(input)?;
    let mut value: Value = serde_json::from_str(input).expect("validated above");
    value.sort_all_objects();
    let mut out = Vec::new();
    let mut serializer = Serializer::with_formatter(
        &mut out,
        PrettyFormatter::with_indent(indent.as_str().as_bytes()),
    );
    serde::Serialize::serialize(&value, &mut serializer).expect("a Value always serializes");
    Ok(String::from_utf8(out).expect("serde_json writes UTF-8"))
}

/// Re-indents valid JSON. `indent: None` minifies. Every byte outside whitespace is copied,
/// and strings are copied untouched.
fn reflow(input: &str, indent: Option<&str>) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + bytes.len() / 4);
    let mut depth = 0usize;
    let newline = |out: &mut Vec<u8>, depth: usize| {
        if let Some(indent) = indent {
            out.push(b'\n');
            for _ in 0..depth {
                out.extend_from_slice(indent.as_bytes());
            }
        }
    };
    let next_token = |from: usize| {
        bytes[from..]
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .map(|offset| from + offset)
    };

    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        match byte {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                out.extend_from_slice(&bytes[start..=i.min(bytes.len() - 1)]);
            }
            b'{' | b'[' => {
                out.push(byte);
                let close = if byte == b'{' { b'}' } else { b']' };
                match next_token(i + 1) {
                    Some(next) if bytes[next] == close => {
                        out.push(close);
                        i = next;
                    }
                    _ => {
                        depth += 1;
                        newline(&mut out, depth);
                    }
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                newline(&mut out, depth);
                out.push(byte);
            }
            b',' => {
                out.push(b',');
                newline(&mut out, depth);
            }
            b':' => {
                out.push(b':');
                if indent.is_some() {
                    out.push(b' ');
                }
            }
            b' ' | b'\t' | b'\n' | b'\r' => {}
            _ => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8(out).expect("only ASCII whitespace was removed from valid UTF-8")
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str =
        r#" {"b": [1.50, 12345678901234567890123, {}], "a": "x\"y\\", "e": [] ,"n": null} "#;

    #[test]
    fn formats_preserving_tokens() {
        let expected = "{\n  \"b\": [\n    1.50,\n    12345678901234567890123,\n    {}\n  ],\n  \"a\": \"x\\\"y\\\\\",\n  \"e\": [],\n  \"n\": null\n}";
        assert_eq!(format(INPUT, Indent::TwoSpaces).unwrap(), expected);
        assert!(
            format(INPUT, Indent::Tab)
                .unwrap()
                .contains("\n\t\"b\": [\n\t\t1.50")
        );
    }

    #[test]
    fn minifies() {
        let minified = minify(INPUT).unwrap();
        assert_eq!(
            minified,
            r#"{"b":[1.50,12345678901234567890123,{}],"a":"x\"y\\","e":[],"n":null}"#
        );
        // Whitespace inside strings stays.
        assert_eq!(minify(r#"{ "a b" : " c " }"#).unwrap(), r#"{"a b":" c "}"#);
        assert_eq!(
            format(&minified, Indent::TwoSpaces).unwrap(),
            format(INPUT, Indent::TwoSpaces).unwrap()
        );
    }

    #[test]
    fn sorts_keys_recursively() {
        let sorted = format_sorted(
            r#"{"b": {"z": 1, "y": 2}, "a": [ {"d": 1, "c": 2} ]}"#,
            Indent::TwoSpaces,
        )
        .unwrap();
        assert_eq!(
            sorted,
            "{\n  \"a\": [\n    {\n      \"c\": 2,\n      \"d\": 1\n    }\n  ],\n  \"b\": {\n    \"y\": 2,\n    \"z\": 1\n  }\n}"
        );
    }

    #[test]
    fn reports_syntax_errors_with_position() {
        let input = "{\n  \"a\": 1,\n  \"b\": tru\n}";
        let error = format(input, Indent::TwoSpaces).unwrap_err();
        assert_eq!((error.line, error.column), (3, 10));
        assert!(error.message.starts_with("Unknown word"), "{error}");
        assert_eq!(error.context(input).unwrap(), "  \"b\": tru\n         ^");
        let trailing = |input: &str| {
            validate(input)
                .unwrap_err()
                .message
                .starts_with("Trailing comma")
        };
        assert!(trailing("{\"a\": 1,}"));
        assert!(trailing("[1,\n\n  ]"));
        let array = validate("{\n  \"tags\": [\"a\", \"b\",]\n}").unwrap_err();
        assert_eq!((array.line, array.column), (2, 21), "{array}");
        assert!(trailing("{\n  \"tags\": [\"a\", \"b\",]\n}"));
        // A missing value elsewhere keeps the general message.
        assert!(
            validate("[1, , 2]")
                .unwrap_err()
                .message
                .starts_with("Expected a value")
        );
        assert!(
            validate("[1, 2")
                .unwrap_err()
                .message
                .starts_with("The JSON ends too early")
        );
        assert!(validate("[1] [2]").is_err());
        assert_eq!(validate("  ").unwrap_err().to_string(), "Paste some JSON");
        assert!(validate("\"just a string\"").is_ok());
    }
}
