//! Text case conversion between identifier styles (camelCase, snake_case, …) and prose cases.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Lower,
    Upper,
    Title,
    Sentence,
    Camel,
    Pascal,
    Snake,
    ScreamingSnake,
    Kebab,
    Train,
    Dot,
    Path,
    Flat,
}

impl Case {
    pub const ALL: [Case; 13] = [
        Case::Lower,
        Case::Upper,
        Case::Title,
        Case::Sentence,
        Case::Camel,
        Case::Pascal,
        Case::Snake,
        Case::ScreamingSnake,
        Case::Kebab,
        Case::Train,
        Case::Dot,
        Case::Path,
        Case::Flat,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Case::Lower => "lowercase",
            Case::Upper => "UPPERCASE",
            Case::Title => "Title Case",
            Case::Sentence => "Sentence case",
            Case::Camel => "camelCase",
            Case::Pascal => "PascalCase",
            Case::Snake => "snake_case",
            Case::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            Case::Kebab => "kebab-case",
            Case::Train => "Train-Case",
            Case::Dot => "dot.case",
            Case::Path => "path/case",
            Case::Flat => "flatcase",
        }
    }

    /// Converts each line of `text` separately. Lowercase and uppercase keep the text as is
    /// apart from letter case; the other styles rebuild it from its words.
    pub fn convert(self, text: &str) -> String {
        text.lines()
            .map(|line| match self {
                Case::Lower => line.to_lowercase(),
                Case::Upper => line.to_uppercase(),
                _ => self.join_words(&split_words(line)),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Joins words in this style, e.g. `["http", "Server"]` → `httpServer` for camelCase.
    pub fn join_words<S: AsRef<str>>(self, words: &[S]) -> String {
        let lower: Vec<String> = words.iter().map(|w| w.as_ref().to_lowercase()).collect();
        let capitalized = || lower.iter().map(|w| capitalize(w)).collect::<Vec<_>>();
        match self {
            Case::Lower => lower.join(" "),
            Case::Upper => lower.join(" ").to_uppercase(),
            Case::Title => capitalized().join(" "),
            Case::Sentence => capitalize(&lower.join(" ")),
            Case::Camel => lower
                .iter()
                .enumerate()
                .map(|(i, w)| if i == 0 { w.clone() } else { capitalize(w) })
                .collect(),
            Case::Pascal => capitalized().concat(),
            Case::Snake => lower.join("_"),
            Case::ScreamingSnake => lower.join("_").to_uppercase(),
            Case::Kebab => lower.join("-"),
            Case::Train => capitalized().join("-"),
            Case::Dot => lower.join("."),
            Case::Path => lower.join("/"),
            Case::Flat => lower.concat(),
        }
    }
}

/// Splits text into words at non-alphanumeric characters and at case changes:
/// `"parseHTTPResponse_v2"` → `["parse", "HTTP", "Response", "v2"]`.
pub fn split_words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        if let Some(&previous) = i.checked_sub(1).and_then(|p| chars.get(p))
            && !current.is_empty()
            && c.is_uppercase()
        {
            let next_is_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            // "fooBar" splits before B; "HTTPServer" splits before the S that starts "Server".
            if previous.is_lowercase()
                || previous.is_ascii_digit()
                || (previous.is_uppercase() && next_is_lower)
            {
                words.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_identifiers_and_prose() {
        assert_eq!(
            split_words("parseHTTPResponse_v2"),
            ["parse", "HTTP", "Response", "v2"]
        );
        assert_eq!(split_words("XMLHttpRequest"), ["XML", "Http", "Request"]);
        assert_eq!(
            split_words("user-id  & email.address"),
            ["user", "id", "email", "address"]
        );
        assert_eq!(split_words("utf8Encoder"), ["utf8", "Encoder"]);
        assert_eq!(split_words("ÜberCool"), ["Über", "Cool"]);
        assert!(split_words(" -_ ").is_empty());
    }

    #[test]
    fn converts_to_every_style() {
        let text = "user accountID";
        let expected = [
            (Case::Lower, "user accountid"),
            (Case::Upper, "USER ACCOUNTID"),
            (Case::Title, "User Account Id"),
            (Case::Sentence, "User account id"),
            (Case::Camel, "userAccountId"),
            (Case::Pascal, "UserAccountId"),
            (Case::Snake, "user_account_id"),
            (Case::ScreamingSnake, "USER_ACCOUNT_ID"),
            (Case::Kebab, "user-account-id"),
            (Case::Train, "User-Account-Id"),
            (Case::Dot, "user.account.id"),
            (Case::Path, "user/account/id"),
            (Case::Flat, "useraccountid"),
        ];
        for (case, output) in expected {
            assert_eq!(case.convert(text), output, "{case:?}");
        }
    }

    #[test]
    fn converts_each_line() {
        assert_eq!(
            Case::Snake.convert("firstName\nlastName"),
            "first_name\nlast_name"
        );
    }
}
