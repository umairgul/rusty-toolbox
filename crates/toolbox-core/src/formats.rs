//! Converts data between JSON, YAML, TOML and CSV, via a JSON value.

use serde_json::{Map, Number, Value};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Json, Format::Yaml, Format::Toml, Format::Csv];

    pub fn label(self) -> &'static str {
        match self {
            Format::Json => "JSON",
            Format::Yaml => "YAML",
            Format::Toml => "TOML",
            Format::Csv => "CSV",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
            Format::Csv => "csv",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// JSON output: indented instead of on one line.
    pub pretty: bool,
    /// CSV input: read `42`, `3.5`, `true` and `false` as numbers and booleans instead of text.
    pub infer_csv_types: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            pretty: true,
            infer_csv_types: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn error(message: impl fmt::Display) -> Error {
    Error(message.to_string())
}

pub fn convert(input: &str, from: Format, to: Format, options: &Options) -> Result<String, Error> {
    let value = parse(input, from, options)?;
    write(&value, to, options)
}

/// Guesses the format of `input`: JSON if it parses as JSON, TOML if it parses as a TOML
/// table, CSV if its first lines have the same number of commas, otherwise YAML.
pub fn detect(input: &str) -> Format {
    let trimmed = input.trim();
    if serde_json::from_str::<Value>(trimmed).is_ok() {
        return Format::Json;
    }
    if !trimmed.is_empty() && trimmed.parse::<toml::Table>().is_ok() {
        return Format::Toml;
    }
    let commas: Vec<usize> = trimmed
        .lines()
        .take(5)
        .map(|line| line.matches(',').count())
        .collect();
    if commas.len() >= 2 && commas[0] > 0 && commas.iter().all(|&c| c == commas[0]) {
        return Format::Csv;
    }
    Format::Yaml
}

pub fn parse(input: &str, format: Format, options: &Options) -> Result<Value, Error> {
    if input.trim().is_empty() {
        return Err(error("Nothing to convert"));
    }
    match format {
        Format::Json => {
            serde_json::from_str(input).map_err(|e| error(format!("Invalid JSON: {e}")))
        }
        // The YAML parser recurses per nesting level, deeper than a small thread stack allows.
        Format::Yaml => crate::stack::with_large_stack(|| {
            serde_saphyr::from_str(input).map_err(|e| error(format!("Invalid YAML: {e}")))
        }),
        Format::Toml => {
            let table: toml::Table = input
                .parse()
                .map_err(|e| error(format!("Invalid TOML: {e}")))?;
            Ok(toml_to_json(toml::Value::Table(table)))
        }
        Format::Csv => parse_csv(input, options.infer_csv_types),
    }
}

pub fn write(value: &Value, format: Format, options: &Options) -> Result<String, Error> {
    match format {
        Format::Json if options.pretty => serde_json::to_string_pretty(value).map_err(error),
        Format::Json => serde_json::to_string(value).map_err(error),
        Format::Yaml => serde_saphyr::to_string(value).map_err(error),
        Format::Toml => {
            let toml::Value::Table(table) = json_to_toml(value, "")? else {
                return Err(error(
                    "TOML needs a table (an object) at the top level, not an array or a single value",
                ));
            };
            toml::to_string_pretty(&table).map_err(error)
        }
        Format::Csv => write_csv(value),
    }
}

fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(i.into()),
        toml::Value::Float(f) => Number::from_f64(f).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(datetime) => Value::String(datetime.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => Value::Object(
            table
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect(),
        ),
    }
}

/// `path` names the value in error messages, e.g. `servers[0].port`.
fn json_to_toml(value: &Value, path: &str) -> Result<toml::Value, Error> {
    Ok(match value {
        Value::Null => {
            let place = if path.is_empty() {
                "the top level".to_owned()
            } else {
                format!("`{path}`")
            };
            return Err(error(format!(
                "TOML has no null value, found one at {place}. Remove it or use a string"
            )));
        }
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => toml::Value::Integer(i),
            None => toml::Value::Float(n.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(items) => toml::Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, item)| json_to_toml(item, &format!("{path}[{i}]")))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => toml::Value::Table(
            map.iter()
                .map(|(key, item)| {
                    let child = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    Ok((key.clone(), json_to_toml(item, &child)?))
                })
                .collect::<Result<_, Error>>()?,
        ),
    })
}

/// Rows become objects keyed by the header row.
fn parse_csv(input: &str, infer_types: bool) -> Result<Value, Error> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(input.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| error(format!("Invalid CSV: {e}")))?
        .iter()
        .map(str::to_owned)
        .collect();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| error(format!("Invalid CSV: {e}")))?;
        let row: Map<String, Value> = headers
            .iter()
            .zip(record.iter())
            .map(|(header, cell)| {
                (
                    header.clone(),
                    if infer_types {
                        infer(cell)
                    } else {
                        Value::String(cell.to_owned())
                    },
                )
            })
            .collect();
        rows.push(Value::Object(row));
    }
    Ok(Value::Array(rows))
}

fn infer(cell: &str) -> Value {
    match cell {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            // Keep values like "007" or "+1" as text, since they are usually identifiers.
            let looks_numeric = !cell.starts_with('+')
                && !(cell.len() > 1 && cell.starts_with('0') && !cell.starts_with("0."));
            if looks_numeric && let Ok(i) = cell.parse::<i64>() {
                return Value::Number(i.into());
            }
            if looks_numeric
                && cell.contains('.')
                && let Some(n) = cell.parse::<f64>().ok().and_then(Number::from_f64)
            {
                return Value::Number(n);
            }
            Value::String(cell.to_owned())
        }
    }
}

/// An array of objects becomes rows under a header of every key, in first-seen order.
/// A single object becomes one row. Nested values are written as JSON.
fn write_csv(value: &Value) -> Result<String, Error> {
    let rows: Vec<&Map<String, Value>> = match value {
        Value::Object(map) => vec![map],
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_object().ok_or_else(|| {
                    error("CSV needs a list of objects, e.g. [{\"a\": 1}, {\"a\": 2}]")
                })
            })
            .collect::<Result<_, _>>()?,
        _ => {
            return Err(error(
                "CSV needs a list of objects, e.g. [{\"a\": 1}, {\"a\": 2}]",
            ));
        }
    };
    let mut columns: Vec<&str> = Vec::new();
    for row in &rows {
        for key in row.keys() {
            if !columns.contains(&key.as_str()) {
                columns.push(key);
            }
        }
    }
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&columns).map_err(error)?;
    for row in rows {
        let cells = columns.iter().map(|column| match row.get(*column) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        });
        writer.write_record(cells).map_err(error)?;
    }
    String::from_utf8(writer.into_inner().map_err(error)?).map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn options() -> Options {
        Options::default()
    }

    #[test]
    fn deeply_nested_yaml_is_an_error_not_a_crash() {
        // Swift runs work on threads with 512 KB stacks.
        let parse_small_stack = |input: String| {
            std::thread::Builder::new()
                .stack_size(512 * 1024)
                .spawn(move || parse(&input, Format::Yaml, &Options::default()).is_ok())
                .unwrap()
                .join()
                .unwrap()
        };
        for depth in [300, 500, 5_000] {
            assert!(!parse_small_stack("{\"a\":".repeat(depth)));
            assert!(!parse_small_stack("[".repeat(depth) + &"]".repeat(depth)));
        }
        assert!(parse_small_stack("[".repeat(10) + &"]".repeat(10)));
    }

    #[test]
    fn json_yaml_round_trip() {
        let json = r#"{"name":"Toolbox","tags":["rust","egui"],"version":1,"beta":false,"ratio":0.5,"owner":null}"#;
        let yaml = convert(json, Format::Json, Format::Yaml, &options()).unwrap();
        // Keys keep their input order rather than being sorted alphabetically.
        assert!(yaml.starts_with("name: Toolbox\ntags:"), "{yaml}");
        let back = convert(
            &yaml,
            Format::Yaml,
            Format::Json,
            &Options {
                pretty: false,
                ..options()
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&back).unwrap(),
            serde_json::from_str::<Value>(json).unwrap()
        );
    }

    #[test]
    fn toml_conversions() {
        let toml = "title = \"Demo\"\nreleased = 2024-05-01T10:00:00Z\n\n[server]\nport = 8080\nhosts = [\"a\", \"b\"]\n";
        let value = parse(toml, Format::Toml, &options()).unwrap();
        assert_eq!(
            value,
            json!({"title": "Demo", "released": "2024-05-01T10:00:00Z", "server": {"port": 8080, "hosts": ["a", "b"]}})
        );
        let back = write(&value, Format::Toml, &options()).unwrap();
        assert_eq!(parse(&back, Format::Toml, &options()).unwrap(), value);

        let with_null =
            write(&json!({"server": {"name": null}}), Format::Toml, &options()).unwrap_err();
        assert!(with_null.0.contains("`server.name`"), "{with_null}");
        assert!(
            write(&json!([1, 2]), Format::Toml, &options())
                .unwrap_err()
                .0
                .contains("top level")
        );
    }

    #[test]
    fn csv_conversions() {
        let csv = "id,name,score,active,zip\n1,\"Doe, Jane\",9.5,true,02134\n2,Sam,,false,10001\n";
        let value = parse(csv, Format::Csv, &options()).unwrap();
        assert_eq!(
            value,
            json!([
                {"id": 1, "name": "Doe, Jane", "score": 9.5, "active": true, "zip": "02134"},
                {"id": 2, "name": "Sam", "score": "", "active": false, "zip": 10001}
            ])
        );
        let text_only = parse(
            csv,
            Format::Csv,
            &Options {
                infer_csv_types: false,
                ..options()
            },
        )
        .unwrap();
        assert_eq!(text_only[0]["id"], "1");

        let out = write(
            &json!([{"a": 1, "b": {"c": true}}, {"a": null, "d": "x,y"}]),
            Format::Csv,
            &options(),
        )
        .unwrap();
        assert_eq!(out, "a,b,d\n1,\"{\"\"c\"\":true}\",\n,,\"x,y\"\n");
        assert!(write(&json!([1, 2]), Format::Csv, &options()).is_err());
    }

    #[test]
    fn detects_formats() {
        assert_eq!(detect(r#"{"a": 1}"#), Format::Json);
        assert_eq!(detect("a = 1\n[b]\nc = 2"), Format::Toml);
        assert_eq!(detect("a,b\n1,2\n3,4"), Format::Csv);
        assert_eq!(detect("a: 1\nb:\n  - x"), Format::Yaml);
    }

    #[test]
    fn reports_parse_errors() {
        assert!(
            convert("{bad", Format::Json, Format::Yaml, &options())
                .unwrap_err()
                .0
                .starts_with("Invalid JSON")
        );
        assert!(convert("  ", Format::Json, Format::Yaml, &options()).is_err());
    }
}
