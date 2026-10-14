use toolbox_core::case::Case;

/// One case style applied to the input, e.g. label "snake_case", value "parse_http_response".
#[derive(uniffi::Record)]
pub struct TextCaseValue {
    pub label: String,
    pub value: String,
}

/// `input` in every case style, in display order. Each line is converted separately.
#[uniffi::export]
pub fn text_case_convert_all(input: &str) -> Vec<TextCaseValue> {
    Case::ALL
        .iter()
        .map(|case| TextCaseValue {
            label: case.label().into(),
            value: case.convert(input),
        })
        .collect()
}
