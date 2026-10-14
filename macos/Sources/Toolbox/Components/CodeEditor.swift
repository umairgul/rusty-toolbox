import AppKit
import SwiftUI

/// A plain-text editor for code and data, backed by `NSTextView`.
///
/// Unlike SwiftUI's `TextEditor`, it never rewrites what you type: smart quotes, dashes, text
/// replacement and spelling correction are off, so JSON, SQL and tokens stay intact. It has
/// the system find bar (⌘F), undo, and a monospaced font.
struct CodeEditor: NSViewRepresentable {
    @Binding var text: String
    var isEditable = true
    var wrapsLines = true
    var placeholder: String?

    init(text: Binding<String>, placeholder: String? = nil, wrapsLines: Bool = true) {
        _text = text
        self.placeholder = placeholder
        self.wrapsLines = wrapsLines
    }

    /// A read-only, selectable view of `text`.
    init(readOnly text: String, wrapsLines: Bool = true) {
        _text = .constant(text)
        isEditable = false
        self.wrapsLines = wrapsLines
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        scrollView.borderType = .noBorder
        scrollView.autohidesScrollers = true
        let textView = scrollView.documentView as! NSTextView
        textView.delegate = context.coordinator
        textView.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        textView.textContainerInset = NSSize(width: 6, height: 8)
        textView.isRichText = false
        textView.importsGraphics = false
        textView.allowsUndo = true
        textView.usesFindBar = true
        textView.isIncrementalSearchingEnabled = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.isAutomaticLinkDetectionEnabled = false
        textView.isAutomaticDataDetectionEnabled = false
        textView.isContinuousSpellCheckingEnabled = false
        textView.isGrammarCheckingEnabled = false
        textView.smartInsertDeleteEnabled = false
        textView.string = text
        update(textView, in: scrollView)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        let textView = scrollView.documentView as! NSTextView
        context.coordinator.text = $text
        if textView.string != text {
            replaceAll(in: textView)
        }
        update(textView, in: scrollView)
    }

    private func update(_ textView: NSTextView, in scrollView: NSScrollView) {
        textView.isEditable = isEditable
        textView.isSelectable = true
        textView.placeholderString = placeholder

        guard let container = textView.textContainer else { return }
        if wrapsLines != container.widthTracksTextView {
            container.widthTracksTextView = wrapsLines
            textView.isHorizontallyResizable = !wrapsLines
            scrollView.hasHorizontalScroller = !wrapsLines
            let unlimited = CGFloat.greatestFiniteMagnitude
            if wrapsLines {
                container.containerSize = NSSize(width: scrollView.contentSize.width, height: unlimited)
                textView.frame.size.width = scrollView.contentSize.width
            } else {
                container.containerSize = NSSize(width: unlimited, height: unlimited)
                textView.maxSize = NSSize(width: unlimited, height: unlimited)
            }
        }
    }

    /// Replaces the whole text. In an editable view this is one undoable change, so "Use as
    /// Input" or "Clear" can be undone with ⌘Z.
    private func replaceAll(in textView: NSTextView) {
        let all = NSRange(location: 0, length: (textView.string as NSString).length)
        if isEditable, textView.shouldChangeText(in: all, replacementString: text) {
            textView.textStorage?.replaceCharacters(in: all, with: text)
            textView.didChangeText()
        } else {
            textView.string = text
        }
        textView.setSelectedRange(NSRange(location: 0, length: 0))
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>

        init(text: Binding<String>) {
            self.text = text
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            if text.wrappedValue != textView.string {
                text.wrappedValue = textView.string
            }
        }
    }
}

private extension NSTextView {
    /// NSTextView draws a placeholder when this private property is set, the same way
    /// NSTextField does. It's the system's own rendering, so it matches the font and insets.
    var placeholderString: String? {
        get { (value(forKey: "placeholderAttributedString") as? NSAttributedString)?.string }
        set {
            guard newValue != placeholderString else { return }
            let placeholder = newValue.map {
                NSAttributedString(string: $0, attributes: [
                    .foregroundColor: NSColor.placeholderTextColor,
                    .font: font ?? .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular),
                ])
            }
            setValue(placeholder, forKey: "placeholderAttributedString")
        }
    }
}
