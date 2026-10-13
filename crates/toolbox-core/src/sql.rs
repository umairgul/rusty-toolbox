//! SQL formatter, built on the `sqlformat` crate.

use sqlformat::{Dialect as SqlDialect, FormatOptions, Indent as SqlIndent, QueryParams};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordCase {
    Upper,
    Lower,
    Preserve,
}

impl KeywordCase {
    pub const ALL: [KeywordCase; 3] = [
        KeywordCase::Upper,
        KeywordCase::Lower,
        KeywordCase::Preserve,
    ];

    pub fn label(self) -> &'static str {
        match self {
            KeywordCase::Upper => "UPPERCASE keywords",
            KeywordCase::Lower => "lowercase keywords",
            KeywordCase::Preserve => "Keep keyword case",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Generic,
    PostgreSql,
    SqlServer,
}

impl Dialect {
    pub const ALL: [Dialect; 3] = [Dialect::Generic, Dialect::PostgreSql, Dialect::SqlServer];

    pub fn label(self) -> &'static str {
        match self {
            Dialect::Generic => "Generic (MySQL, SQLite, …)",
            Dialect::PostgreSql => "PostgreSQL",
            Dialect::SqlServer => "SQL Server",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub indent: Indent,
    pub keyword_case: KeywordCase,
    pub dialect: Dialect,
    /// Keep each query on one line.
    pub compact: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            indent: Indent::TwoSpaces,
            keyword_case: KeywordCase::Upper,
            dialect: Dialect::Generic,
            compact: false,
        }
    }
}

pub fn format(sql: &str, options: &Options) -> String {
    let format_options = FormatOptions {
        indent: match options.indent {
            Indent::TwoSpaces => SqlIndent::Spaces(2),
            Indent::FourSpaces => SqlIndent::Spaces(4),
            Indent::Tab => SqlIndent::Tabs,
        },
        uppercase: match options.keyword_case {
            KeywordCase::Upper => Some(true),
            KeywordCase::Lower => Some(false),
            KeywordCase::Preserve => None,
        },
        dialect: match options.dialect {
            Dialect::Generic => SqlDialect::Generic,
            Dialect::PostgreSql => SqlDialect::PostgreSql,
            Dialect::SqlServer => SqlDialect::SQLServer,
        },
        inline: options.compact,
        // The number of line breaks between queries: 2 leaves one blank line.
        lines_between_queries: if options.compact { 1 } else { 2 },
        ..FormatOptions::default()
    };
    sqlformat::format(sql.trim(), &QueryParams::None, &format_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY: &str = "select u.id, u.name, count(o.id) as orders from users u left join orders o on o.user_id = u.id where u.active = 1 group by u.id, u.name order by orders desc limit 10";

    #[test]
    fn formats_clauses_on_their_own_lines() {
        let formatted = format(QUERY, &Options::default());
        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[0], "SELECT");
        for clause in [
            "FROM",
            "LEFT JOIN",
            "WHERE",
            "GROUP BY",
            "ORDER BY",
            "LIMIT",
        ] {
            assert!(
                lines
                    .iter()
                    .any(|line| line.trim_start().starts_with(clause)),
                "{clause} missing in\n{formatted}"
            );
        }
        assert!(lines.contains(&"  u.id,"), "{formatted}");
    }

    #[test]
    fn applies_options() {
        let lower = format(
            "SELECT a FROM t",
            &Options {
                keyword_case: KeywordCase::Lower,
                ..Options::default()
            },
        );
        assert!(lower.starts_with("select"), "{lower}");
        let kept = format(
            "Select a From t",
            &Options {
                keyword_case: KeywordCase::Preserve,
                ..Options::default()
            },
        );
        assert!(kept.starts_with("Select"), "{kept}");
        let four = format(
            QUERY,
            &Options {
                indent: Indent::FourSpaces,
                ..Options::default()
            },
        );
        assert!(four.lines().any(|line| line == "    u.id,"), "{four}");
        let compact = format(
            QUERY,
            &Options {
                compact: true,
                ..Options::default()
            },
        );
        assert_eq!(compact.lines().count(), 1, "{compact}");
    }

    #[test]
    fn separates_multiple_queries() {
        let formatted = format("select 1; select 2;", &Options::default());
        assert!(formatted.contains(";\n\nSELECT"), "{formatted}");
    }
}
