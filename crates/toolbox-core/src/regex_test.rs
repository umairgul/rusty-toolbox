//! Regex tester: matches with capture groups, and replace, using `fancy-regex` (Rust regex
//! syntax plus lookaround and backreferences). A backtracking limit turns catastrophic
//! patterns into an error instead of a hang.

use fancy_regex::{Regex, RegexBuilder};
use std::ops::Range;

/// Steps the backtracking engine may take per match attempt before giving up.
const BACKTRACK_LIMIT: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// `i`: letters match regardless of case.
    pub case_insensitive: bool,
    /// `m`: `^` and `$` match at line breaks.
    pub multi_line: bool,
    /// `s`: `.` also matches newlines.
    pub dot_all: bool,
    /// `x`: whitespace in the pattern is ignored and `#` starts a comment.
    pub extended: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The group's name, for `(?P<name>…)` or `(?<name>…)`.
    pub name: Option<String>,
    /// `None` if the group didn't take part in this match.
    pub range: Option<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// Byte range in the searched text.
    pub range: Range<usize>,
    /// Capture groups 1 and up.
    pub groups: Vec<Group>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Matches {
    pub matches: Vec<Match>,
    /// True if there were more than the requested limit.
    pub truncated: bool,
}

pub fn compile(pattern: &str, flags: Flags) -> Result<Regex, String> {
    RegexBuilder::new(pattern)
        .case_insensitive(flags.case_insensitive)
        .multi_line(flags.multi_line)
        .dot_matches_new_line(flags.dot_all)
        .verbose_mode(flags.extended)
        .backtrack_limit(BACKTRACK_LIMIT)
        .build()
        .map_err(|error| error.to_string())
}

/// Up to `limit` matches of `pattern` in `text`. An empty pattern finds nothing.
pub fn find(pattern: &str, flags: Flags, text: &str, limit: usize) -> Result<Matches, String> {
    if pattern.is_empty() {
        return Ok(Matches::default());
    }
    let regex = compile(pattern, flags)?;
    let names: Vec<Option<String>> = regex
        .capture_names()
        .skip(1)
        .map(|name| name.map(str::to_owned))
        .collect();
    let mut result = Matches::default();
    for captures in regex.captures_iter(text) {
        let captures = captures.map_err(|error| error.to_string())?;
        if result.matches.len() == limit {
            result.truncated = true;
            break;
        }
        let whole = captures.get(0).expect("group 0 is the whole match");
        let groups = names
            .iter()
            .enumerate()
            .map(|(i, name)| Group {
                name: name.clone(),
                range: captures.get(i + 1).map(|m| m.range()),
            })
            .collect();
        result.matches.push(Match {
            range: whole.range(),
            groups,
        });
    }
    Ok(result)
}

/// Replaces every match. In `replacement`, `$1` or `${1}` inserts a numbered group,
/// `${name}` a named one, and `$$` a literal `$`.
pub fn replace(
    pattern: &str,
    flags: Flags,
    text: &str,
    replacement: &str,
) -> Result<String, String> {
    if pattern.is_empty() {
        return Ok(text.to_owned());
    }
    let regex = compile(pattern, flags)?;
    // `replace_all` panics on runtime errors such as the backtrack limit; `try_replacen` with
    // a limit of 0 replaces every match and returns them instead.
    regex
        .try_replacen(text, 0, replacement)
        .map(|out| out.into_owned())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'t>(text: &'t str, matches: &Matches) -> Vec<&'t str> {
        matches
            .matches
            .iter()
            .map(|m| &text[m.range.clone()])
            .collect()
    }

    #[test]
    fn finds_matches_and_groups() {
        let text = "alice@example.com, bob@test.org";
        let found = find(r"(?<user>\w+)@(\w+)\.\w+", Flags::default(), text, 100).unwrap();
        assert_eq!(texts(text, &found), ["alice@example.com", "bob@test.org"]);
        let groups = &found.matches[1].groups;
        assert_eq!(groups[0].name.as_deref(), Some("user"));
        assert_eq!(&text[groups[0].range.clone().unwrap()], "bob");
        assert_eq!(groups[1].name, None);
        assert_eq!(&text[groups[1].range.clone().unwrap()], "test");
    }

    #[test]
    fn applies_flags() {
        let text = "Line one\nline two";
        assert_eq!(
            find("^line", Flags::default(), text, 10)
                .unwrap()
                .matches
                .len(),
            0
        );
        let both = Flags {
            case_insensitive: true,
            multi_line: true,
            ..Flags::default()
        };
        assert_eq!(find("^line", both, text, 10).unwrap().matches.len(), 2);
        let dot_all = Flags {
            dot_all: true,
            ..Flags::default()
        };
        assert_eq!(
            find("one.line", dot_all, text, 10).unwrap().matches.len(),
            1
        );
        let extended = Flags {
            extended: true,
            ..Flags::default()
        };
        assert_eq!(
            find(r"t w o  # spaced out", extended, text, 10)
                .unwrap()
                .matches
                .len(),
            1
        );
    }

    #[test]
    fn supports_lookaround_and_backreferences() {
        let text = "price: $42, cost: 17, $9";
        assert_eq!(
            texts(
                text,
                &find(r"(?<=\$)\d+", Flags::default(), text, 10).unwrap()
            ),
            ["42", "9"]
        );
        let doubled = "this is is a test test";
        assert_eq!(
            texts(
                doubled,
                &find(r"\b(\w+) \1\b", Flags::default(), doubled, 10).unwrap()
            ),
            ["is is", "test test"]
        );
    }

    #[test]
    fn limits_and_optional_groups() {
        let found = find(r"\d", Flags::default(), "1 2 3 4", 2).unwrap();
        assert_eq!(found.matches.len(), 2);
        assert!(found.truncated);
        let optional = find(r"a(b)?", Flags::default(), "a", 10).unwrap();
        assert_eq!(optional.matches[0].groups[0].range, None);
        assert!(
            find("", Flags::default(), "abc", 10)
                .unwrap()
                .matches
                .is_empty()
        );
    }

    #[test]
    fn replaces_with_group_references() {
        let text = "2024-05-15 and 2023-01-02";
        let swapped = replace(
            r"(?<y>\d{4})-(\d{2})-(\d{2})",
            Flags::default(),
            text,
            "$3/$2/${y}",
        )
        .unwrap();
        assert_eq!(swapped, "15/05/2024 and 02/01/2023");
        assert_eq!(replace("x", Flags::default(), "x", "$$").unwrap(), "$");
    }

    #[test]
    fn reports_errors() {
        assert!(find("(unclosed", Flags::default(), "x", 10).is_err());
        // Catastrophic backtracking hits the limit instead of hanging.
        let text = format!("{}b", "a".repeat(40));
        let evil = find(r"(a|aa)+\1$", Flags::default(), &text, 10);
        assert!(evil.is_err(), "{evil:?}");
        assert!(replace(r"(a|aa)+\1$", Flags::default(), &text, "x").is_err());
    }
}
