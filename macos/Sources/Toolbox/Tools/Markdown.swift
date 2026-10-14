import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers
import WebKit

@Observable
final class MarkdownModel {
    static let sample = """
        # Toolbox notes

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
            println!("Hello from Toolbox!");
        }
        ```

        > Blockquotes work too.

        | Tool | Category |
        |------|----------|
        | Hashes | Converters |
        | Regex Tester | Formatters |

        """

    var text = sample
    var wrapsLines = true
    private(set) var rendered: Rendered?

    struct Rendered: Sendable {
        let html: String
        let stats: MarkdownStats
    }

    fileprivate func render(_ text: String) async {
        let rendered = await offMainThread {
            Rendered(html: markdownToHtml(markdown: text), stats: markdownReadingStats(markdown: text))
        }
        if let rendered { self.rendered = rendered }
    }

    /// Asks where to save, then writes a standalone HTML page. Returns a message for the status
    /// bar, or nil if cancelled.
    fileprivate func saveHTMLPage() -> SaveResult? {
        let page = markdownToHtmlPage(markdown: text, title: "Document")
        do {
            guard let url = try Files.save(Data(page.utf8), suggestedName: "document.html", allowedTypes: [.html]) else {
                return nil
            }
            return SaveResult(ok: true, message: "Saved \(url.lastPathComponent)")
        } catch {
            return SaveResult(ok: false, message: error.message)
        }
    }

    struct SaveResult {
        let ok: Bool
        let message: String
    }
}

struct MarkdownView: View {
    @Bindable var model: MarkdownModel
    @ViewState private var saveResult: MarkdownModel.SaveResult?

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Markdown") {
                    CodeEditor(text: $model.text, placeholder: "Write Markdown", wrapsLines: model.wrapsLines)
                } accessories: {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        model.text = Pasteboard.string ?? model.text
                    }
                    .help("Replace the Markdown with the clipboard")
                    Button("Clear", systemImage: "xmark.circle") { model.text = "" }
                        .help("Clear the Markdown")
                        .disabled(model.text.isEmpty)
                }
                Pane("Preview") {
                    if let rendered = model.rendered {
                        MarkdownPreview(html: rendered.html)
                    } else {
                        Color.clear
                    }
                }
            }
            StatusBar { status }
        }
        .task(id: model.text) {
            await model.render(model.text)
        }
        .onChange(of: model.text) { saveResult = nil }
        .toolbar {
            ToolbarItemGroup {
                CopyButton(title: "Copy HTML", text: model.rendered?.html ?? "")
                    .help("Copy the rendered HTML")
                Button("Save HTML Page…", systemImage: "square.and.arrow.down") {
                    saveResult = model.saveHTMLPage() ?? saveResult
                }
                .help("Save a standalone HTML page")
                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines in the editor")
            }
        }
    }

    @ViewBuilder private var status: some View {
        if let stats = model.rendered?.stats {
            Text("\(stats.words) \(stats.words == 1 ? "word" : "words") · \(stats.minutes) min read")
                .foregroundStyle(.secondary)
                .monospacedDigit()
        }
        Spacer()
        if let saveResult {
            StatusLabel(ok: saveResult.ok, message: saveResult.message)
                .lineLimit(1)
        }
    }
}

/// Shows rendered Markdown in a web view. The page is loaded once; later updates replace the
/// body, so the scroll position survives typing. Links open in the default browser.
private struct MarkdownPreview: NSViewRepresentable {
    let html: String

    private static let page = """
        <!doctype html>
        <html>
        <head>
        <meta charset="utf-8">
        <style>
        :root { color-scheme: light dark; }
        body { font: 14px/1.55 -apple-system, system-ui, sans-serif; margin: 0; padding: 12px 18px 24px; overflow-wrap: break-word; }
        h1, h2 { padding-bottom: .25em; border-bottom: 1px solid rgba(128, 128, 128, .3); }
        h1 { font-size: 1.8em; } h2 { font-size: 1.4em; }
        a { color: LinkText; color: -apple-system-blue; }
        pre, code { font-family: ui-monospace, SF Mono, Menlo, monospace; font-size: 12.5px; background: rgba(128, 128, 128, .14); border-radius: 5px; }
        pre { padding: 10px 12px; overflow-x: auto; line-height: 1.45; }
        code { padding: .1em .35em; }
        pre code { padding: 0; background: none; }
        blockquote { margin: 0; padding: 0 1em; border-left: 4px solid rgba(128, 128, 128, .35); color: rgba(128, 128, 128, 1); }
        table { border-collapse: collapse; }
        th, td { border: 1px solid rgba(128, 128, 128, .35); padding: 5px 12px; }
        th { background: rgba(128, 128, 128, .1); }
        img { max-width: 100%; }
        hr { border: 0; border-top: 1px solid rgba(128, 128, 128, .35); }
        li:has(> input[type=checkbox]) { list-style: none; }
        li > input[type=checkbox] { margin: 0 .4em 0 -1.3em; }
        </style>
        </head>
        <body><main id="content"></main></body>
        </html>
        """

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        // The preview is the user's own text, but raw HTML in it shouldn't run scripts.
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = context.coordinator
        webView.allowsMagnification = true
        context.coordinator.html = html
        webView.loadHTMLString(Self.page, baseURL: nil)
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.update(html, in: webView)
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        var html = ""
        private var isLoaded = false
        private var shown: String?

        func update(_ html: String, in webView: WKWebView) {
            self.html = html
            guard isLoaded, html != shown else { return }
            shown = html
            // Runs in the app's own content world, so it works with page scripts disabled.
            webView.callAsyncJavaScript(
                "document.getElementById('content').innerHTML = html",
                arguments: ["html": html],
                in: nil,
                in: .defaultClient
            )
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            isLoaded = true
            shown = nil
            update(html, in: webView)
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping @MainActor (WKNavigationActionPolicy) -> Void
        ) {
            guard let url = navigationAction.request.url else { return decisionHandler(.cancel) }
            // The page itself, and in-page anchors such as footnotes.
            if url.scheme == "about" {
                let inPage = navigationAction.navigationType == .linkActivated && url.fragment != nil
                return decisionHandler(!isLoaded || inPage ? .allow : .cancel)
            }
            if navigationAction.navigationType == .linkActivated {
                NSWorkspace.shared.open(url)
            }
            decisionHandler(.cancel)
        }
    }
}
