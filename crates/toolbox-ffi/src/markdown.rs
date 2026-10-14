use toolbox_core::markdown;

#[derive(uniffi::Record)]
pub struct MarkdownStats {
    pub words: u64,
    /// Minutes to read at 200 words per minute, rounded up.
    pub minutes: u64,
}

/// Renders GitHub-flavoured Markdown (tables, task lists, strikethrough, footnotes) to an HTML
/// fragment.
#[uniffi::export]
pub fn markdown_to_html(markdown: &str) -> String {
    markdown::to_html(markdown)
}

/// A standalone HTML page around the rendered Markdown, with light default styling.
#[uniffi::export]
pub fn markdown_to_html_page(markdown: &str, title: &str) -> String {
    markdown::to_html_page(markdown, title)
}

#[uniffi::export]
pub fn markdown_reading_stats(markdown: &str) -> MarkdownStats {
    let (words, minutes) = markdown::reading_stats(markdown);
    MarkdownStats {
        words: words as u64,
        minutes: minutes as u64,
    }
}
