//! Regex tester. The core reports byte ranges; this module converts them to UTF-16 offsets,
//! which is what `NSString`/`NSRange` (and so the Swift text views) use.

use crate::ToolError;
use toolbox_core::regex_test::{self, Flags};

#[derive(Clone, Copy, Default, uniffi::Record)]
pub struct RegexFlags {
    /// `i`: letters match regardless of case.
    pub case_insensitive: bool,
    /// `m`: `^` and `$` match at line breaks.
    pub multi_line: bool,
    /// `s`: `.` also matches newlines.
    pub dot_all: bool,
    /// `x`: whitespace in the pattern is ignored and `#` starts a comment.
    pub extended: bool,
}

impl From<RegexFlags> for Flags {
    fn from(flags: RegexFlags) -> Self {
        Flags {
            case_insensitive: flags.case_insensitive,
            multi_line: flags.multi_line,
            dot_all: flags.dot_all,
            extended: flags.extended,
        }
    }
}

/// A matched range of the searched text.
#[derive(uniffi::Record)]
pub struct RegexSpan {
    /// Start and end in UTF-16 code units, ready for `NSRange`.
    pub start: u64,
    pub end: u64,
    /// The matched text.
    pub text: String,
    /// The matched text as a quoted, escaped string literal, e.g. `"a\nb"`.
    pub quoted: String,
}

#[derive(uniffi::Record)]
pub struct RegexGroup {
    /// The group's name, or its number (from 1) when it has none.
    pub label: String,
    /// `None` if the group didn't take part in this match.
    pub span: Option<RegexSpan>,
}

#[derive(uniffi::Record)]
pub struct RegexMatch {
    pub span: RegexSpan,
    /// Capture groups 1 and up.
    pub groups: Vec<RegexGroup>,
}

#[derive(uniffi::Record)]
pub struct RegexMatches {
    pub matches: Vec<RegexMatch>,
    /// True if there were more than the requested limit.
    pub truncated: bool,
}

/// Up to `limit` matches of `pattern` in `text`. An empty pattern finds nothing. Throws for an
/// invalid pattern, or when matching hits the backtracking limit.
#[uniffi::export]
pub fn regex_find(
    pattern: &str,
    flags: RegexFlags,
    text: &str,
    limit: u32,
) -> Result<RegexMatches, ToolError> {
    let found =
        regex_test::find(pattern, flags.into(), text, limit as usize).map_err(ToolError::new)?;
    let offsets = Utf16Offsets::new(
        text,
        found
            .matches
            .iter()
            .flat_map(|m| {
                let groups = m.groups.iter().filter_map(|g| g.range.clone());
                std::iter::once(m.range.clone()).chain(groups)
            })
            .flat_map(|range| [range.start, range.end])
            .collect(),
    );
    let span = |range: std::ops::Range<usize>| {
        let matched = &text[range.clone()];
        RegexSpan {
            start: offsets.get(range.start),
            end: offsets.get(range.end),
            text: matched.to_owned(),
            quoted: format!("{matched:?}"),
        }
    };
    let matches = found
        .matches
        .into_iter()
        .map(|found| RegexMatch {
            span: span(found.range),
            groups: found
                .groups
                .into_iter()
                .enumerate()
                .map(|(index, group)| RegexGroup {
                    label: group.name.unwrap_or_else(|| (index + 1).to_string()),
                    span: group.range.map(span),
                })
                .collect(),
        })
        .collect();
    Ok(RegexMatches {
        matches,
        truncated: found.truncated,
    })
}

/// Replaces every match. In `replacement`, `$1` or `${1}` inserts a numbered group, `${name}` a
/// named one, and `$$` a literal `$`. An empty pattern returns the text unchanged.
#[uniffi::export]
pub fn regex_replace(
    pattern: &str,
    flags: RegexFlags,
    text: &str,
    replacement: &str,
) -> Result<String, ToolError> {
    regex_test::replace(pattern, flags.into(), text, replacement).map_err(ToolError::new)
}

/// Maps byte offsets in a string to UTF-16 offsets, for a known set of offsets, in one pass.
struct Utf16Offsets {
    bytes: Vec<usize>,
    utf16: Vec<u64>,
}

impl Utf16Offsets {
    /// `offsets` must be char boundaries of `text`, in any order.
    fn new(text: &str, mut offsets: Vec<usize>) -> Self {
        offsets.sort_unstable();
        offsets.dedup();
        let mut utf16 = Vec::with_capacity(offsets.len());
        let (mut position, mut units) = (0, 0u64);
        for &offset in &offsets {
            units += text[position..offset]
                .chars()
                .map(|c| c.len_utf16() as u64)
                .sum::<u64>();
            position = offset;
            utf16.push(units);
        }
        Utf16Offsets {
            bytes: offsets,
            utf16,
        }
    }

    fn get(&self, byte: usize) -> u64 {
        let index = self
            .bytes
            .binary_search(&byte)
            .expect("offset was registered");
        self.utf16[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_slice(text: &str, span: &RegexSpan) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[span.start as usize..span.end as usize]).unwrap()
    }

    #[test]
    fn converts_byte_ranges_to_utf16() {
        // é is 2 bytes / 1 unit, 😀 is 4 bytes / 2 units.
        let text = "é😀 café=1 😀x=22";
        let found = regex_find(r"(\w+)=(?<n>\d+)", RegexFlags::default(), text, 10).unwrap();
        assert_eq!(found.matches.len(), 2);
        let first = &found.matches[0];
        assert_eq!(first.span.text, "café=1");
        assert_eq!((first.span.start, first.span.end), (4, 10));
        assert_eq!(utf16_slice(text, &first.span), "café=1");
        assert_eq!(first.groups[0].label, "1");
        assert_eq!(first.groups[1].label, "n");
        let second = &found.matches[1];
        assert_eq!(utf16_slice(text, &second.span), "x=22");
        assert_eq!(
            utf16_slice(text, second.groups[1].span.as_ref().unwrap()),
            "22"
        );
    }

    #[test]
    fn quotes_and_reports_missing_groups() {
        let found = regex_find("a(b)?\n", RegexFlags::default(), "a\n", 10).unwrap();
        let only = &found.matches[0];
        assert_eq!(only.span.quoted, r#""a\n""#);
        assert!(only.groups[0].span.is_none());
    }

    #[test]
    fn flags_limit_and_errors() {
        let flags = RegexFlags {
            case_insensitive: true,
            ..RegexFlags::default()
        };
        let found = regex_find("A", flags, "a a a", 2).unwrap();
        assert_eq!(found.matches.len(), 2);
        assert!(found.truncated);
        assert!(regex_find("(", RegexFlags::default(), "x", 10).is_err());
        assert_eq!(
            regex_replace(r"(\d)", RegexFlags::default(), "a1", "<$1>").unwrap(),
            "a<1>"
        );
    }
}
