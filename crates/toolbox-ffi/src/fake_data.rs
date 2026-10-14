use crate::ToolError;
use toolbox_core::fake::{self, Field, Format, MAX_ROWS};

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum FakeDataField {
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

impl From<FakeDataField> for Field {
    fn from(field: FakeDataField) -> Self {
        match field {
            FakeDataField::Id => Field::Id,
            FakeDataField::FirstName => Field::FirstName,
            FakeDataField::LastName => Field::LastName,
            FakeDataField::FullName => Field::FullName,
            FakeDataField::Email => Field::Email,
            FakeDataField::Username => Field::Username,
            FakeDataField::Phone => Field::Phone,
            FakeDataField::Company => Field::Company,
            FakeDataField::JobTitle => Field::JobTitle,
            FakeDataField::Street => Field::Street,
            FakeDataField::City => Field::City,
            FakeDataField::Postcode => Field::Postcode,
            FakeDataField::Country => Field::Country,
            FakeDataField::BirthDate => Field::BirthDate,
        }
    }
}

impl From<Field> for FakeDataField {
    fn from(field: Field) -> Self {
        match field {
            Field::Id => FakeDataField::Id,
            Field::FirstName => FakeDataField::FirstName,
            Field::LastName => FakeDataField::LastName,
            Field::FullName => FakeDataField::FullName,
            Field::Email => FakeDataField::Email,
            Field::Username => FakeDataField::Username,
            Field::Phone => FakeDataField::Phone,
            Field::Company => FakeDataField::Company,
            Field::JobTitle => FakeDataField::JobTitle,
            Field::Street => FakeDataField::Street,
            Field::City => FakeDataField::City,
            Field::Postcode => FakeDataField::Postcode,
            Field::Country => FakeDataField::Country,
            Field::BirthDate => FakeDataField::BirthDate,
        }
    }
}

/// Every field, in the order columns are written.
#[uniffi::export]
pub fn fake_data_all_fields() -> Vec<FakeDataField> {
    Field::ALL.into_iter().map(Into::into).collect()
}

/// The fields selected when the tool opens.
#[uniffi::export]
pub fn fake_data_default_fields() -> Vec<FakeDataField> {
    Field::default_fields()
        .into_iter()
        .map(Into::into)
        .collect()
}

/// The column name used in every output format, e.g. "first_name".
#[uniffi::export]
pub fn fake_data_field_column(field: FakeDataField) -> String {
    Field::from(field).column().into()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum FakeDataFormat {
    Csv,
    Json,
    Sql,
}

impl From<FakeDataFormat> for Format {
    fn from(format: FakeDataFormat) -> Self {
        match format {
            FakeDataFormat::Csv => Format::Csv,
            FakeDataFormat::Json => Format::Json,
            FakeDataFormat::Sql => Format::Sql,
        }
    }
}

#[derive(uniffi::Record)]
pub struct FakeDataFormatInfo {
    /// e.g. "CSV".
    pub label: String,
    /// e.g. "csv", without the dot.
    pub extension: String,
}

#[uniffi::export]
pub fn fake_data_format_info(format: FakeDataFormat) -> FakeDataFormatInfo {
    let format = Format::from(format);
    FakeDataFormatInfo {
        label: format.label().into(),
        extension: format.extension().into(),
    }
}

/// The most rows one batch can have.
#[uniffi::export]
pub fn fake_data_max_rows() -> u32 {
    MAX_ROWS as u32
}

#[derive(Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct FakeDataOptions {
    pub rows: u32,
    /// Columns are always written in [`fake_data_all_fields`] order; duplicates are ignored.
    pub fields: Vec<FakeDataField>,
    pub format: FakeDataFormat,
    /// Table name for SQL output.
    pub table: String,
}

impl From<FakeDataOptions> for fake::Options {
    fn from(options: FakeDataOptions) -> Self {
        let chosen: Vec<Field> = options.fields.into_iter().map(Into::into).collect();
        fake::Options {
            rows: options.rows as usize,
            fields: Field::ALL
                .into_iter()
                .filter(|field| chosen.contains(field))
                .collect(),
            format: options.format.into(),
            table: options.table,
        }
    }
}

/// Generates a batch of random records. Up to a few megabytes for the largest batches: call it
/// off the main thread.
#[uniffi::export]
pub fn fake_data_generate(options: FakeDataOptions) -> Result<String, ToolError> {
    fake::generate(&options.into()).map_err(ToolError::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_written_in_canonical_order_without_duplicates() {
        let options = FakeDataOptions {
            rows: 2,
            fields: vec![
                FakeDataField::Email,
                FakeDataField::Id,
                FakeDataField::Email,
            ],
            format: FakeDataFormat::Csv,
            table: "users".into(),
        };
        let csv = fake_data_generate(options).unwrap();
        assert_eq!(csv.lines().next(), Some("id,email"));
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn reports_core_errors() {
        let options = FakeDataOptions {
            rows: 1,
            fields: vec![],
            format: FakeDataFormat::Json,
            table: "users".into(),
        };
        assert!(fake_data_generate(options).is_err());
    }
}
