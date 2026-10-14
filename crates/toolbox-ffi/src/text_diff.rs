use toolbox_core::diff::{self, Diff, Options, Row, RowKind, Segment, Side};

#[derive(Clone, Copy, Default, uniffi::Record)]
pub struct TextDiffOptions {
    /// Treat lines as equal if they differ only in whitespace.
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
}

impl From<TextDiffOptions> for Options {
    fn from(options: TextDiffOptions) -> Self {
        Options {
            ignore_whitespace: options.ignore_whitespace,
            ignore_case: options.ignore_case,
        }
    }
}

#[derive(Clone, Copy, uniffi::Enum)]
pub enum TextDiffRowKind {
    Equal,
    /// The line exists on both sides but differs.
    Changed,
    Removed,
    Added,
}

impl From<RowKind> for TextDiffRowKind {
    fn from(kind: RowKind) -> Self {
        match kind {
            RowKind::Equal => TextDiffRowKind::Equal,
            RowKind::Changed => TextDiffRowKind::Changed,
            RowKind::Removed => TextDiffRowKind::Removed,
            RowKind::Added => TextDiffRowKind::Added,
        }
    }
}

/// A run of text in a line, `changed` if it differs from the other side.
#[derive(uniffi::Record)]
pub struct TextDiffSegment {
    pub text: String,
    pub changed: bool,
}

impl From<Segment> for TextDiffSegment {
    fn from(Segment { text, changed }: Segment) -> Self {
        TextDiffSegment { text, changed }
    }
}

/// One side of a row: its 1-based line number and contents.
#[derive(uniffi::Record)]
pub struct TextDiffSide {
    pub line: u64,
    pub segments: Vec<TextDiffSegment>,
}

impl From<Side> for TextDiffSide {
    fn from(side: Side) -> Self {
        TextDiffSide {
            line: side.line as u64,
            segments: side.segments.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(uniffi::Record)]
pub struct TextDiffRow {
    pub kind: TextDiffRowKind,
    /// `None` when the line exists only on the other side.
    pub left: Option<TextDiffSide>,
    pub right: Option<TextDiffSide>,
}

impl From<Row> for TextDiffRow {
    fn from(row: Row) -> Self {
        TextDiffRow {
            kind: row.kind.into(),
            left: row.left.map(Into::into),
            right: row.right.map(Into::into),
        }
    }
}

#[derive(uniffi::Record)]
pub struct TextDiffResult {
    /// At most `max_rows` rows; `total_rows` says how many there are in all.
    pub rows: Vec<TextDiffRow>,
    pub total_rows: u64,
    /// Lines only on the right, counting changed lines.
    pub added: u64,
    /// Lines only on the left, counting changed lines.
    pub removed: u64,
    /// A unified diff (`diff -u` style) with 3 lines of context, ignoring the options.
    pub unified: String,
}

impl TextDiffResult {
    fn new(diff: Diff, max_rows: usize, unified: String) -> Self {
        TextDiffResult {
            total_rows: diff.rows.len() as u64,
            added: diff.added as u64,
            removed: diff.removed as u64,
            rows: diff
                .rows
                .into_iter()
                .take(max_rows)
                .map(Into::into)
                .collect(),
            unified,
        }
    }
}

/// Compares two texts line by line, with changed words marked, returning at most `max_rows`
/// rows. Slow for long texts: call it off the main thread.
#[uniffi::export]
pub fn text_diff_compare(
    original: &str,
    changed: &str,
    options: TextDiffOptions,
    max_rows: u32,
) -> TextDiffResult {
    TextDiffResult::new(
        diff::diff(original, changed, options.into()),
        max_rows as usize,
        diff::unified(original, changed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_rows_but_keeps_counts() {
        let original = "a\nb\nc\nd";
        let changed = "a\nB\nc\nd\ne";
        let result = text_diff_compare(original, changed, TextDiffOptions::default(), 2);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.total_rows, 5);
        assert_eq!((result.added, result.removed), (2, 1));
        assert!(matches!(result.rows[1].kind, TextDiffRowKind::Changed));
        assert!(result.unified.contains("-b\n+B\n"), "{}", result.unified);
    }
}
