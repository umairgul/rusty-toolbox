//! Side-by-side text diff with word-level highlights, and unified diff output.

use similar::{Algorithm, ChangeTag, DiffOp, TextDiff, capture_diff_slices};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Treat lines as equal if they differ only in whitespace.
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Equal,
    /// The line exists on both sides but differs.
    Changed,
    Removed,
    Added,
}

/// A run of text in a line, `changed` if it differs from the other side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub changed: bool,
}

/// One side of a row: its 1-based line number and contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    pub line: usize,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: RowKind,
    pub left: Option<Side>,
    pub right: Option<Side>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub rows: Vec<Row>,
    /// Lines only on the right, counting changed lines.
    pub added: usize,
    /// Lines only on the left, counting changed lines.
    pub removed: usize,
}

impl Diff {
    pub fn is_identical(&self) -> bool {
        self.added == 0 && self.removed == 0
    }
}

pub fn diff(old: &str, new: &str, options: Options) -> Diff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let normalize = |line: &&str| -> String {
        let line = if options.ignore_whitespace {
            line.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            (*line).to_owned()
        };
        if options.ignore_case {
            line.to_lowercase()
        } else {
            line
        }
    };
    let old_keys: Vec<String> = old_lines.iter().map(normalize).collect();
    let new_keys: Vec<String> = new_lines.iter().map(normalize).collect();

    let whole = |line: &str| {
        vec![Segment {
            text: line.to_owned(),
            changed: false,
        }]
    };
    let mut result = Diff::default();
    for op in capture_diff_slices(Algorithm::Patience, &old_keys, &new_keys) {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for k in 0..len {
                    result.rows.push(Row {
                        kind: RowKind::Equal,
                        left: Some(Side {
                            line: old_index + k + 1,
                            segments: whole(old_lines[old_index + k]),
                        }),
                        right: Some(Side {
                            line: new_index + k + 1,
                            segments: whole(new_lines[new_index + k]),
                        }),
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                result.removed += old_len;
                for (k, line) in old_lines[old_index..old_index + old_len].iter().enumerate() {
                    let left = Side {
                        line: old_index + k + 1,
                        segments: whole(line),
                    };
                    result.rows.push(Row {
                        kind: RowKind::Removed,
                        left: Some(left),
                        right: None,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                result.added += new_len;
                for (k, line) in new_lines[new_index..new_index + new_len].iter().enumerate() {
                    let right = Side {
                        line: new_index + k + 1,
                        segments: whole(line),
                    };
                    result.rows.push(Row {
                        kind: RowKind::Added,
                        left: None,
                        right: Some(right),
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                result.removed += old_len;
                result.added += new_len;
                // Pair lines up as changes; leftovers are plain removals or additions.
                for k in 0..old_len.max(new_len) {
                    let old_line = (k < old_len).then(|| (old_index + k, old_lines[old_index + k]));
                    let new_line = (k < new_len).then(|| (new_index + k, new_lines[new_index + k]));
                    result.rows.push(match (old_line, new_line) {
                        (Some((oi, o)), Some((ni, n))) => {
                            let (left, right) = word_diff(o, n);
                            Row {
                                kind: RowKind::Changed,
                                left: Some(Side {
                                    line: oi + 1,
                                    segments: left,
                                }),
                                right: Some(Side {
                                    line: ni + 1,
                                    segments: right,
                                }),
                            }
                        }
                        (Some((oi, o)), None) => Row {
                            kind: RowKind::Removed,
                            left: Some(Side {
                                line: oi + 1,
                                segments: whole(o),
                            }),
                            right: None,
                        },
                        (None, Some((ni, n))) => Row {
                            kind: RowKind::Added,
                            left: None,
                            right: Some(Side {
                                line: ni + 1,
                                segments: whole(n),
                            }),
                        },
                        (None, None) => unreachable!("k is below the longer length"),
                    });
                }
            }
        }
    }
    result
}

/// Splits two versions of a line into segments, marking the words that differ.
fn word_diff(old: &str, new: &str) -> (Vec<Segment>, Vec<Segment>) {
    let mut left: Vec<Segment> = Vec::new();
    let mut right: Vec<Segment> = Vec::new();
    let push = |side: &mut Vec<Segment>, text: &str, changed: bool| match side.last_mut() {
        Some(last) if last.changed == changed => last.text.push_str(text),
        _ => side.push(Segment {
            text: text.to_owned(),
            changed,
        }),
    };
    for change in TextDiff::from_words(old, new).iter_all_changes() {
        let text = change.value();
        match change.tag() {
            ChangeTag::Equal => {
                push(&mut left, text, false);
                push(&mut right, text, false);
            }
            ChangeTag::Delete => push(&mut left, text, true),
            ChangeTag::Insert => push(&mut right, text, true),
        }
    }
    (left, right)
}

/// A unified diff (`diff -u` / `git diff` style) with 3 lines of context.
pub fn unified(old: &str, new: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header("original", "modified")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(diff: &Diff) -> Vec<RowKind> {
        diff.rows.iter().map(|row| row.kind).collect()
    }

    #[test]
    fn classifies_rows() {
        let old = "one\ntwo\nthree\nfour";
        let new = "one\n2\nthree\nfour\nfive";
        let diff = diff(old, new, Options::default());
        assert_eq!(
            kinds(&diff),
            [
                RowKind::Equal,
                RowKind::Changed,
                RowKind::Equal,
                RowKind::Equal,
                RowKind::Added
            ]
        );
        assert_eq!((diff.added, diff.removed), (2, 1));
        let added = diff.rows[4].right.as_ref().unwrap();
        assert_eq!(added.line, 5);
        assert!(diff.rows[4].left.is_none());
    }

    #[test]
    fn highlights_changed_words() {
        let diff = diff("let x = 1;", "let y = 1;", Options::default());
        let left = &diff.rows[0].left.as_ref().unwrap().segments;
        let right = &diff.rows[0].right.as_ref().unwrap().segments;
        let changed = |segments: &[Segment]| {
            segments
                .iter()
                .filter(|s| s.changed)
                .map(|s| s.text.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(left), ["x"]);
        assert_eq!(changed(right), ["y"]);
        let rebuilt: String = right.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(rebuilt, "let y = 1;");
    }

    #[test]
    fn ignore_options() {
        let old = "Hello   World\n  indented";
        let new = "hello world\nindented";
        assert_eq!(diff(old, new, Options::default()).removed, 2);
        let ignore_ws = diff(
            old,
            new,
            Options {
                ignore_whitespace: true,
                ..Options::default()
            },
        );
        assert_eq!(kinds(&ignore_ws), [RowKind::Changed, RowKind::Equal]);
        let ignore_both = diff(
            old,
            new,
            Options {
                ignore_whitespace: true,
                ignore_case: true,
            },
        );
        assert!(ignore_both.is_identical());
        // Equal rows still show each side's original text.
        assert_eq!(
            ignore_both.rows[0].left.as_ref().unwrap().segments[0].text,
            "Hello   World"
        );
    }

    #[test]
    fn unified_output() {
        let patch = unified("a\nb\nc\n", "a\nB\nc\n");
        assert!(
            patch.starts_with("--- original\n+++ modified\n@@ -1,3 +1,3 @@\n"),
            "{patch}"
        );
        assert!(patch.contains("-b\n+B\n"), "{patch}");
        assert!(diff("same", "same", Options::default()).is_identical());
    }
}
