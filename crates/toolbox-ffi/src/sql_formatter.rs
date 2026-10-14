use toolbox_core::sql::{self, Dialect, Indent, KeywordCase, Options};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum SqlIndent {
    TwoSpaces,
    FourSpaces,
    Tab,
}

impl From<SqlIndent> for Indent {
    fn from(indent: SqlIndent) -> Self {
        match indent {
            SqlIndent::TwoSpaces => Indent::TwoSpaces,
            SqlIndent::FourSpaces => Indent::FourSpaces,
            SqlIndent::Tab => Indent::Tab,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum SqlKeywordCase {
    Upper,
    Lower,
    Preserve,
}

impl From<SqlKeywordCase> for KeywordCase {
    fn from(case: SqlKeywordCase) -> Self {
        match case {
            SqlKeywordCase::Upper => KeywordCase::Upper,
            SqlKeywordCase::Lower => KeywordCase::Lower,
            SqlKeywordCase::Preserve => KeywordCase::Preserve,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum SqlDialect {
    Generic,
    PostgreSql,
    SqlServer,
}

impl From<SqlDialect> for Dialect {
    fn from(dialect: SqlDialect) -> Self {
        match dialect {
            SqlDialect::Generic => Dialect::Generic,
            SqlDialect::PostgreSql => Dialect::PostgreSql,
            SqlDialect::SqlServer => Dialect::SqlServer,
        }
    }
}

#[derive(Clone, Copy, uniffi::Record)]
pub struct SqlFormatOptions {
    pub indent: SqlIndent,
    pub keyword_case: SqlKeywordCase,
    pub dialect: SqlDialect,
    /// Keep each query on one line.
    pub compact: bool,
}

impl From<SqlFormatOptions> for Options {
    fn from(options: SqlFormatOptions) -> Self {
        Options {
            indent: options.indent.into(),
            keyword_case: options.keyword_case.into(),
            dialect: options.dialect.into(),
            compact: options.compact,
        }
    }
}

/// e.g. "2 spaces".
#[uniffi::export]
pub fn sql_indent_label(indent: SqlIndent) -> String {
    Indent::from(indent).label().into()
}

/// e.g. "UPPERCASE keywords".
#[uniffi::export]
pub fn sql_keyword_case_label(keyword_case: SqlKeywordCase) -> String {
    KeywordCase::from(keyword_case).label().into()
}

/// e.g. "Generic (MySQL, SQLite, …)".
#[uniffi::export]
pub fn sql_dialect_label(dialect: SqlDialect) -> String {
    Dialect::from(dialect).label().into()
}

/// Formats one or more queries. Never fails: text it can't parse is passed through.
#[uniffi::export]
pub fn sql_format(input: &str, options: SqlFormatOptions) -> String {
    sql::format(input, &options.into())
}
