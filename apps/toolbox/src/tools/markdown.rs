use super::{Category, Tool, copy_button, save_file};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use toolbox_core::markdown;

const SAMPLE: &str = "# Toolbox notes

Markdown is rendered **live** as you type, with *emphasis*, `inline code` and [links](https://commonmark.org).

## Lists

- Generators
- Encoders & converters
- Formatters & testers
  - JSON, regex, diff, SQL and Markdown

1. Write
2. Preview
3. Export as HTML

- [x] Task lists
- [ ] Even more tools

## Code

```rust
fn main() {
    println!(\"Hello from Toolbox!\");
}
```

> Blockquotes work too.

| Tool | Category |
|------|----------|
| Hashes | Converters |
| Regex Tester | Formatters |
";

pub struct MarkdownTool {
    text: String,
    cache: CommonMarkCache,
    status: Option<String>,
}

impl Default for MarkdownTool {
    fn default() -> Self {
        Self {
            text: SAMPLE.into(),
            cache: CommonMarkCache::default(),
            status: None,
        }
    }
}

impl Tool for MarkdownTool {
    fn name(&self) -> &'static str {
        "Markdown Preview"
    }

    fn description(&self) -> &'static str {
        "Write Markdown with a live preview, then copy or save it as HTML."
    }

    fn category(&self) -> Category {
        Category::Formatters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            copy_button(
                ui,
                "copy_markdown_html",
                "Copy HTML",
                &markdown::to_html(&self.text),
            );
            if ui.button("Save HTML page…").clicked() {
                let page = markdown::to_html_page(&self.text, "Document");
                self.status = save_file("document.html", "HTML page", "html", page.as_bytes());
            }
            let (words, minutes) = markdown::reading_stats(&self.text);
            ui.weak(format!("{words} words · {minutes} min read"));
        });
        if let Some(status) = &self.status {
            ui.weak(status.as_str());
        }
        ui.add_space(8.0);

        let height = (ui.available_height() - 8.0).max(320.0);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("markdown_editor")
                .max_height(height)
                .show(&mut columns[0], |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(24)
                            .hint_text("Write Markdown"),
                    );
                });
            egui::ScrollArea::vertical()
                .id_salt("markdown_preview")
                .max_height(height)
                .show(&mut columns[1], |ui| {
                    CommonMarkViewer::new().show(ui, &mut self.cache, &self.text);
                });
        });
    }
}
