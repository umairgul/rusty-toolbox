//! Fake data for seeding test databases, exported as CSV, JSON or SQL `INSERT` statements.
//!
//! Emails use the reserved example.com/.org/.net domains and phone numbers use the 555-01xx
//! range reserved for fiction, so generated rows never point at real people. Emails and
//! usernames are unique within one batch, so they satisfy unique constraints.

use crate::data::{CITIES, JOB_TITLES, STREET_NAMES, STREET_SUFFIXES};
use crate::names;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::fmt;

pub const MAX_ROWS: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Id,
    FirstName,
    LastName,
    FullName,
    Email,
    Username,
    Phone,
    Company,
    JobTitle,
    Street,
    City,
    Postcode,
    Country,
    BirthDate,
}

impl Field {
    pub const ALL: [Field; 14] = [
        Field::Id,
        Field::FirstName,
        Field::LastName,
        Field::FullName,
        Field::Email,
        Field::Username,
        Field::Phone,
        Field::Company,
        Field::JobTitle,
        Field::Street,
        Field::City,
        Field::Postcode,
        Field::Country,
        Field::BirthDate,
    ];

    /// Column name used in every output format.
    pub fn column(self) -> &'static str {
        match self {
            Field::Id => "id",
            Field::FirstName => "first_name",
            Field::LastName => "last_name",
            Field::FullName => "full_name",
            Field::Email => "email",
            Field::Username => "username",
            Field::Phone => "phone",
            Field::Company => "company",
            Field::JobTitle => "job_title",
            Field::Street => "street",
            Field::City => "city",
            Field::Postcode => "postcode",
            Field::Country => "country",
            Field::BirthDate => "birth_date",
        }
    }

    pub fn default_fields() -> Vec<Field> {
        vec![
            Field::Id,
            Field::FirstName,
            Field::LastName,
            Field::Email,
            Field::Phone,
            Field::City,
            Field::Country,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Csv,
    Json,
    Sql,
}

impl Format {
    pub const ALL: [Format; 3] = [Format::Csv, Format::Json, Format::Sql];

    pub fn label(self) -> &'static str {
        match self {
            Format::Csv => "CSV",
            Format::Json => "JSON",
            Format::Sql => "SQL",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Json => "json",
            Format::Sql => "sql",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub rows: usize,
    /// Columns in output order.
    pub fields: Vec<Field>,
    pub format: Format,
    /// Table name for SQL output.
    pub table: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rows: 10,
            fields: Field::default_fields(),
            format: Format::Csv,
            table: "users".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NoFields,
    InvalidRows,
    InvalidTableName,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoFields => write!(f, "Select at least one field"),
            Error::InvalidRows => write!(f, "Rows must be between 1 and {MAX_ROWS}"),
            Error::InvalidTableName => {
                write!(
                    f,
                    "Table name must be letters, digits and underscores, optionally as schema.table"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

enum Value {
    Number(usize),
    Text(String),
}

pub fn generate(options: &Options) -> Result<String, Error> {
    generate_with(options, &mut rand::rng())
}

pub fn generate_with(options: &Options, rng: &mut impl Rng) -> Result<String, Error> {
    if options.fields.is_empty() {
        return Err(Error::NoFields);
    }
    if !(1..=MAX_ROWS).contains(&options.rows) {
        return Err(Error::InvalidRows);
    }
    if options.format == Format::Sql && !is_valid_table_name(&options.table) {
        return Err(Error::InvalidTableName);
    }

    let mut unique = Unique::default();
    let rows: Vec<Vec<Value>> = (1..=options.rows)
        .map(|id| row(id, &options.fields, &mut unique, rng))
        .collect();
    let columns: Vec<&str> = options.fields.iter().map(|f| f.column()).collect();

    Ok(match options.format {
        Format::Csv => to_csv(&columns, &rows),
        Format::Json => to_json(&columns, &rows),
        Format::Sql => to_sql(&options.table, &columns, &rows),
    })
}

/// Values already used in this batch, for columns that are usually unique.
#[derive(Default)]
struct Unique {
    emails: HashSet<String>,
    usernames: HashSet<String>,
}

fn row(id: usize, fields: &[Field], unique: &mut Unique, rng: &mut impl Rng) -> Vec<Value> {
    let (first, last) = names::person(rng);
    let (city, country) = *CITIES.choose(rng).expect("city list is not empty");

    fields
        .iter()
        .map(|field| match field {
            Field::Id => Value::Number(id),
            Field::FirstName => Value::Text(first.to_owned()),
            Field::LastName => Value::Text(last.to_owned()),
            Field::FullName => Value::Text(format!("{first} {last}")),
            Field::Email => {
                let domain = ["example.com", "example.org", "example.net"]
                    .choose(rng)
                    .copied()
                    .unwrap_or_default();
                let local = format!("{}.{}", names::slug(first), names::slug(last));
                Value::Text(make_unique(
                    &mut unique.emails,
                    format!("{local}@{domain}"),
                    |n| format!("{local}{n}@{domain}"),
                ))
            }
            Field::Username => {
                let base = names::username(first, last, rng);
                Value::Text(make_unique(&mut unique.usernames, base.clone(), |n| {
                    format!("{base}{n}")
                }))
            }
            Field::Phone => Value::Text(format!(
                "+1 ({}) 555-01{:02}",
                rng.random_range(201..=989),
                rng.random_range(0..100)
            )),
            Field::Company => Value::Text(names::company(rng)),
            Field::JobTitle => Value::Text(
                JOB_TITLES
                    .choose(rng)
                    .copied()
                    .unwrap_or_default()
                    .to_owned(),
            ),
            Field::Street => Value::Text(format!(
                "{} {} {}",
                rng.random_range(1..=9999),
                STREET_NAMES.choose(rng).copied().unwrap_or_default(),
                STREET_SUFFIXES.choose(rng).copied().unwrap_or_default()
            )),
            Field::City => Value::Text(city.to_owned()),
            Field::Postcode => Value::Text(rng.random_range(10000..=99999).to_string()),
            Field::Country => Value::Text(country.to_owned()),
            Field::BirthDate => Value::Text(format!(
                "{}-{:02}-{:02}",
                rng.random_range(1950..=2006),
                rng.random_range(1..=12),
                rng.random_range(1..=28)
            )),
        })
        .collect()
}

/// Returns `first_choice` if unused, otherwise `numbered(2)`, `numbered(3)`, … until one is free.
fn make_unique(
    used: &mut HashSet<String>,
    first_choice: String,
    numbered: impl Fn(usize) -> String,
) -> String {
    let mut candidate = first_choice;
    let mut n = 2;
    while used.contains(&candidate) {
        candidate = numbered(n);
        n += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn is_valid_table_name(name: &str) -> bool {
    let is_identifier = |part: &str| {
        part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() <= 2 && parts.iter().all(|part| is_identifier(part))
}

fn to_csv(columns: &[&str], rows: &[Vec<Value>]) -> String {
    let escape = |text: &str| {
        if text.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    };
    let mut out = columns.join(",");
    for row in rows {
        out.push('\n');
        let cells: Vec<String> = row
            .iter()
            .map(|value| match value {
                Value::Number(n) => n.to_string(),
                Value::Text(text) => escape(text),
            })
            .collect();
        out.push_str(&cells.join(","));
    }
    out.push('\n');
    out
}

/// A JSON array with one object per line.
fn to_json(columns: &[&str], rows: &[Vec<Value>]) -> String {
    let objects: Vec<String> = rows
        .iter()
        .map(|row| {
            let members: Vec<String> = columns
                .iter()
                .zip(row)
                .map(|(column, value)| {
                    let value = match value {
                        Value::Number(n) => n.to_string(),
                        Value::Text(text) => {
                            serde_json::to_string(text).expect("strings always serialize")
                        }
                    };
                    format!("\"{column}\": {value}")
                })
                .collect();
            format!("  {{{}}}", members.join(", "))
        })
        .collect();
    format!("[\n{}\n]\n", objects.join(",\n"))
}

fn to_sql(table: &str, columns: &[&str], rows: &[Vec<Value>]) -> String {
    let tuples: Vec<String> = rows
        .iter()
        .map(|row| {
            let values: Vec<String> = row
                .iter()
                .map(|value| match value {
                    Value::Number(n) => n.to_string(),
                    Value::Text(text) => format!("'{}'", text.replace('\'', "''")),
                })
                .collect();
            format!("  ({})", values.join(", "))
        })
        .collect();
    format!(
        "INSERT INTO {table} ({}) VALUES\n{};\n",
        columns.join(", "),
        tuples.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn generate(options: &Options) -> Result<String, Error> {
        generate_with(options, &mut StdRng::seed_from_u64(21))
    }

    #[test]
    fn csv_has_header_and_rows() {
        let options = Options {
            rows: 25,
            ..Default::default()
        };
        let csv = generate(&options).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "id,first_name,last_name,email,phone,city,country");
        assert_eq!(lines.len(), 26);
        assert!(lines[1].starts_with("1,"));
    }

    #[test]
    fn csv_quotes_special_characters() {
        let rows = vec![vec![Value::Text("a, \"b\"".into())]];
        assert_eq!(to_csv(&["name"], &rows), "name\n\"a, \"\"b\"\"\"\n");
    }

    #[test]
    fn json_is_valid_and_typed() {
        let options = Options {
            rows: 50,
            fields: Field::ALL.to_vec(),
            format: Format::Json,
            ..Default::default()
        };
        let json = generate(&options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let rows = parsed.as_array().unwrap();
        assert_eq!(rows.len(), 50);
        assert_eq!(rows[0]["id"], 1);
        assert!(rows[0]["email"].as_str().unwrap().contains("@example."));
    }

    #[test]
    fn sql_escapes_quotes() {
        let rows = vec![vec![Value::Number(1), Value::Text("O'Brien".into())]];
        assert_eq!(
            to_sql("users", &["id", "last_name"], &rows),
            "INSERT INTO users (id, last_name) VALUES\n  (1, 'O''Brien');\n"
        );
    }

    #[test]
    fn emails_and_usernames_are_unique() {
        let options = Options {
            rows: 3000,
            fields: vec![Field::Email, Field::Username],
            ..Default::default()
        };
        let csv = generate(&options).unwrap();
        let mut emails = HashSet::new();
        let mut usernames = HashSet::new();
        for line in csv.lines().skip(1) {
            let (email, username) = line.split_once(',').unwrap();
            assert!(emails.insert(email.to_owned()), "duplicate {email}");
            assert!(
                usernames.insert(username.to_owned()),
                "duplicate {username}"
            );
        }
    }

    #[test]
    fn rejects_invalid_options() {
        assert_eq!(
            generate(&Options {
                fields: vec![],
                ..Default::default()
            }),
            Err(Error::NoFields)
        );
        assert_eq!(
            generate(&Options {
                rows: 0,
                ..Default::default()
            }),
            Err(Error::InvalidRows)
        );
        let bad_table = Options {
            format: Format::Sql,
            table: "users; DROP TABLE x".into(),
            ..Default::default()
        };
        assert_eq!(generate(&bad_table), Err(Error::InvalidTableName));
        assert!(
            generate(&Options {
                format: Format::Sql,
                table: "public.users".into(),
                ..Default::default()
            })
            .is_ok()
        );
    }
}
