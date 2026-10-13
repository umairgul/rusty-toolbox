//! Markdown helpers: HTML export and reading statistics.

use pulldown_cmark::{Options, Parser, html};

/// Renders GitHub-flavoured Markdown (tables, task lists, strikethrough, footnotes) to HTML.
pub fn to_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(markdown, options));
    out
}

/// A standalone HTML page around rendered Markdown, with light default styling.
pub fn to_html_page(markdown: &str, title: &str) -> String {
    let title = html_escape::encode_text(title);
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\nbody {{ font: 16px/1.6 -apple-system, system-ui, sans-serif; max-width: 760px; margin: 2rem auto; padding: 0 1rem; color: #1f2328; }}\npre, code {{ font-family: ui-monospace, Menlo, monospace; background: #f6f8fa; border-radius: 6px; }}\npre {{ padding: 1rem; overflow-x: auto; }}\ncode {{ padding: .1em .3em; }}\npre code {{ padding: 0; }}\ntable {{ border-collapse: collapse; }}\nth, td {{ border: 1px solid #d0d7de; padding: .4rem .8rem; }}\nblockquote {{ margin: 0; padding-left: 1rem; border-left: 4px solid #d0d7de; color: #59636e; }}\n</style>\n</head>\n<body>\n{}</body>\n</html>\n",
        to_html(markdown)
    )
}

/// (words, minutes to read at 200 words per minute, rounded up).
pub fn reading_stats(markdown: &str) -> (usize, usize) {
    let words = markdown
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count();
    (words, words.div_ceil(200))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_github_flavoured_markdown() {
        let html = to_html(
            "# Title\n\nSome **bold** and ~~gone~~.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n",
        );
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<del>gone</del>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn page_wraps_and_escapes_title() {
        let page = to_html_page("hi", "A <b> & c");
        assert!(page.starts_with("<!doctype html>"));
        assert!(page.contains("<title>A &lt;b&gt; &amp; c</title>"));
        assert!(page.contains("<p>hi</p>"));
    }

    #[test]
    fn counts_words() {
        assert_eq!(reading_stats("# Hello world\n\n- one two -"), (4, 1));
        assert_eq!(reading_stats(&"word ".repeat(401)), (401, 3));
        assert_eq!(reading_stats(""), (0, 0));
    }
}
