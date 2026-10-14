import AppKit
import SwiftUI
import ToolboxBindings

@Observable
final class RegexTesterModel {
    /// Matches listed and highlighted. Counting stops here too.
    nonisolated static let maxMatches: UInt32 = 1000
    /// Rows shown in the match table.
    static let maxRows = 200

    enum Mode: Hashable { case matches, replace }

    var pattern = #"(?<user>[\w.+-]+)@(?<domain>[\w-]+\.[\w.]+)"#
    var flags = RegexFlags(caseInsensitive: false, multiLine: false, dotAll: false, extended: false)
    var text = "Contact alice@example.com or bob.smith+test@mail.example.org.\nInvalid: not-an-email@, @nope.com"
    var replacement = "${user} at ${domain}"
    var mode = Mode.matches
    var showsCheatSheet = false
    private(set) var outcome: Outcome?

    struct Key: Equatable, Sendable {
        let pattern: String
        let flags: RegexFlags
        let text: String
        let replacement: String
    }

    struct Outcome: Sendable {
        enum Matching: Sendable {
            case found(RegexMatches)
            case failed(String)
        }

        enum Replacing: Sendable {
            case replaced(String)
            case failed(String)
        }

        let matching: Matching
        let replacing: Replacing
        let highlights: RegexHighlights

        nonisolated init(_ key: Key) {
            do {
                let found = try regexFind(pattern: key.pattern, flags: key.flags, text: key.text, limit: RegexTesterModel.maxMatches)
                matching = .found(found)
                highlights = RegexHighlights(text: key.text, matches: found.matches)
            } catch {
                matching = .failed(error.message)
                highlights = RegexHighlights(text: key.text, matches: [])
            }
            do {
                replacing = .replaced(try regexReplace(pattern: key.pattern, flags: key.flags, text: key.text, replacement: key.replacement))
            } catch {
                replacing = .failed(error.message)
            }
        }
    }

    fileprivate var key: Key {
        Key(pattern: pattern, flags: flags, text: text, replacement: replacement)
    }

    fileprivate func run(_ key: Key) async {
        if let outcome = await offMainThread({ Outcome(key) }) {
            self.outcome = outcome
        }
    }
}

/// Match and group ranges in UTF-16 offsets, for the text they were found in.
struct RegexHighlights: Equatable, Sendable {
    let text: String
    let matches: [Range<Int>]
    let groups: [Range<Int>]

    nonisolated init(text: String, matches: [RegexMatch]) {
        self.text = text
        self.matches = matches.map(\.span.range)
        self.groups = matches.flatMap { $0.groups.compactMap { $0.span?.range } }
    }
}

private extension RegexSpan {
    nonisolated var range: Range<Int> { Int(start)..<Int(end) }
    var nsRange: NSRange { NSRange(location: Int(start), length: Int(end - start)) }
}

private let cheatSheet: [(token: String, meaning: String)] = [
    (".", "any character except newline"),
    (#"\d  \w  \s"#, "digit, word character, whitespace"),
    (#"\D  \W  \S"#, "not a digit, word character, whitespace"),
    ("[abc]  [^abc]  [a-z]", "one of, none of, range"),
    ("^  $", "start, end (of line with the m flag)"),
    (#"\b"#, "word boundary"),
    ("*  +  ?", "0 or more, 1 or more, 0 or 1"),
    ("{3}  {2,5}  {2,}", "exactly, between, at least"),
    ("*?  +?", "lazy: as few as possible"),
    ("(…)  (?:…)", "capture group, non-capturing group"),
    ("(?<name>…)", "named group"),
    (#"\1"#, "backreference to group 1"),
    ("(?=…)  (?!…)", "lookahead, negative lookahead"),
    ("(?<=…)  (?<!…)", "lookbehind, negative lookbehind"),
    ("a|b", "a or b"),
]

struct RegexTesterView: View {
    @Bindable var model: RegexTesterModel
    @ViewState private var selection: MatchRow.ID?

    var body: some View {
        VStack(spacing: 0) {
            patternBar
            Divider()
            VSplitView {
                Pane("Test String") {
                    RegexTextEditor(
                        text: $model.text,
                        placeholder: "Text to search",
                        highlights: model.outcome?.highlights,
                        reveal: selectedMatch?.span.nsRange
                    )
                } accessories: {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        model.text = Pasteboard.string ?? model.text
                    }
                    .help("Replace the test string with the clipboard")
                    Button("Clear", systemImage: "xmark.circle") { model.text = "" }
                        .help("Clear the test string")
                        .disabled(model.text.isEmpty)
                }
                .frame(minHeight: 120, idealHeight: 220)
                switch model.mode {
                case .matches: matchesPane
                case .replace: replacePane
                }
            }
            StatusBar { status }
        }
        .task(id: model.key) {
            await model.run(model.key)
        }
        .onChange(of: model.text) { selection = nil }
        .onChange(of: model.pattern) { selection = nil }
        .onChange(of: model.flags) { selection = nil }
        .inspector(isPresented: $model.showsCheatSheet) {
            CheatSheet()
                .inspectorColumnWidth(min: 240, ideal: 280, max: 380)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Mode", selection: $model.mode) {
                    Text("Matches").tag(RegexTesterModel.Mode.matches)
                    Text("Replace").tag(RegexTesterModel.Mode.replace)
                }
                .pickerStyle(.segmented)
                .help("List matches and groups, or replace every match")

                Button("Cheat Sheet", systemImage: "book") { model.showsCheatSheet.toggle() }
                    .help(model.showsCheatSheet ? "Hide the regex cheat sheet" : "Show the regex cheat sheet")
            }
        }
    }

    // MARK: Pattern

    private var patternBar: some View {
        Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 8, verticalSpacing: 8) {
            GridRow {
                Text("Pattern")
                    .foregroundStyle(.secondary)
                    .gridColumnAlignment(.trailing)
                HStack(spacing: 6) {
                    Text("/").foregroundStyle(.secondary)
                    TextField("Pattern", text: $model.pattern, prompt: Text("Regular expression"))
                        .labelsHidden()
                    Text("/").foregroundStyle(.secondary)
                    flagToggles
                }
                .font(.body.monospaced())
            }
            if model.mode == .replace {
                GridRow {
                    Text("Replace")
                        .foregroundStyle(.secondary)
                    VStack(alignment: .leading, spacing: 4) {
                        TextField("Replace With", text: $model.replacement, prompt: Text("Replacement"))
                            .labelsHidden()
                            .font(.body.monospaced())
                        Text("$1 or ${name} insert groups, $$ a literal $")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .textFieldStyle(.roundedBorder)
        .autocorrectionDisabled()
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
    }

    private var flagToggles: some View {
        HStack(spacing: 2) {
            Toggle("i", isOn: $model.flags.caseInsensitive)
                .help("i: case-insensitive")
            Toggle("m", isOn: $model.flags.multiLine)
                .help("m: multi-line, ^ and $ match at line breaks")
            Toggle("s", isOn: $model.flags.dotAll)
                .help("s: dot matches newlines")
            Toggle("x", isOn: $model.flags.extended)
                .help("x: extended, ignore whitespace and # comments in the pattern")
        }
        .toggleStyle(.button)
        .fixedSize()
    }

    // MARK: Results

    private var rows: [MatchRow] {
        guard case .found(let found) = model.outcome?.matching else { return [] }
        return found.matches.prefix(RegexTesterModel.maxRows).enumerated().map { MatchRow(id: $0.offset, match: $0.element) }
    }

    private var selectedMatch: RegexMatch? {
        guard let selection else { return nil }
        let rows = rows
        return rows.indices.contains(selection) ? rows[selection].match : nil
    }

    private var matchesPane: some View {
        Pane("Matches") {
            switch model.outcome?.matching {
            case .failed(let message):
                ErrorMessage(message: message)
            case .found(let found) where model.pattern.isEmpty || found.matches.isEmpty:
                if model.pattern.isEmpty {
                    ContentUnavailableView("No Pattern", systemImage: "asterisk", description: Text("Enter a pattern to start matching."))
                } else {
                    ContentUnavailableView("No Matches", systemImage: "magnifyingglass", description: Text("The pattern doesn't match the test string."))
                }
            case .found:
                matchesTable
            case nil:
                Color.clear
            }
        }
        .frame(minHeight: 140)
    }

    private var matchesTable: some View {
        Table(rows, selection: $selection) {
            TableColumn("#") { row in
                Text("\(row.id + 1)").monospacedDigit()
            }
            .width(min: 24, ideal: 32, max: 56)
            TableColumn("Position") { row in
                Text("\(row.match.span.start)–\(row.match.span.end)")
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
            .width(min: 60, ideal: 80, max: 120)
            TableColumn("Match") { row in
                Text(row.match.span.quoted)
                    .font(.body.monospaced())
                    .help(row.match.span.text)
            }
            .width(min: 100, ideal: 220)
            TableColumn("Groups") { row in
                let groups = row.groups
                Text(groups)
                    .font(.body.monospaced())
                    .help(groups)
            }
            .width(min: 100, ideal: 320)
        }
        .contextMenu(forSelectionType: MatchRow.ID.self) { ids in
            if let id = ids.first, let row = rows.first(where: { $0.id == id }) {
                Button("Copy Match") { Pasteboard.copy(row.match.span.text) }
                if !row.match.groups.isEmpty {
                    Button("Copy Groups") { Pasteboard.copy(row.groups) }
                }
            }
        }
    }

    private var replacePane: some View {
        Pane("Result") {
            switch model.outcome?.replacing {
            case .replaced(let output):
                CodeEditor(readOnly: output)
            case .failed(let message):
                ErrorMessage(message: message)
            case nil:
                Color.clear
            }
        } accessories: {
            if case .replaced(let output) = model.outcome?.replacing {
                Button("Use as Test String", systemImage: "arrow.uturn.left") { model.text = output }
                    .help("Replace the test string with this result")
                CopyButton(title: "Copy Result", text: output)
            }
        }
        .frame(minHeight: 140)
    }

    @ViewBuilder private var status: some View {
        switch model.outcome?.matching {
        case .failed(let message):
            StatusLabel(ok: false, message: message)
                .lineLimit(1)
                .help(message)
        case .found where model.pattern.isEmpty:
            Text("Enter a pattern to start matching.")
                .foregroundStyle(.secondary)
        case .found(let found):
            let count = found.matches.count
            Text("\(count)\(found.truncated ? "+" : "") \(count == 1 ? "match" : "matches")")
                .fontWeight(.semibold)
                .monospacedDigit()
            Spacer()
            if model.mode == .matches && count > RegexTesterModel.maxRows {
                Text("Showing the first \(RegexTesterModel.maxRows) matches.")
                    .foregroundStyle(.secondary)
            }
        case nil:
            EmptyView()
        }
    }
}

private struct MatchRow: Identifiable {
    let id: Int
    let match: RegexMatch

    /// e.g. `user: "alice"   domain: "example.com"`.
    var groups: String {
        match.groups.map { "\($0.label): \($0.span?.quoted ?? "—")" }.joined(separator: "   ")
    }
}

private struct ErrorMessage: View {
    let message: String

    var body: some View {
        VStack(alignment: .leading) {
            Label(message, systemImage: "exclamationmark.triangle.fill")
                .symbolRenderingMode(.multicolor)
                .textSelection(.enabled)
            Spacer()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
    }
}

private struct CheatSheet: View {
    var body: some View {
        List {
            Section("Cheat Sheet") {
                ForEach(cheatSheet, id: \.token) { entry in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(entry.token)
                            .font(.body.monospaced())
                            .textSelection(.enabled)
                        Text(entry.meaning)
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 2)
                }
            }
        }
    }
}

// MARK: - Highlighting editor

/// A plain-text editor like `CodeEditor` that highlights regex matches (alternating background
/// colors) and underlines capture groups.
///
/// Highlights are TextKit 1 temporary attributes, so they never touch the text storage: typing,
/// undo and the typing attributes stay as they are. They're applied only when `highlights` was
/// computed for the text on screen; while a newer result is pending, the old highlights move
/// along with edits.
private struct RegexTextEditor: NSViewRepresentable {
    @Binding var text: String
    var placeholder: String?
    var highlights: RegexHighlights?
    /// A range to scroll to and flash, e.g. the match selected in the table.
    var reveal: NSRange?

    private static let font = NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)

    /// The match colors alternate so neighbouring matches stay distinct.
    private static let matchColors = [
        NSColor(name: nil) { $0.isDark ? NSColor(srgbRed: 110 / 255, green: 88 / 255, blue: 20 / 255, alpha: 1) : NSColor(srgbRed: 1, green: 226 / 255, blue: 120 / 255, alpha: 1) },
        NSColor(name: nil) { $0.isDark ? NSColor(srgbRed: 30 / 255, green: 72 / 255, blue: 120 / 255, alpha: 1) : NSColor(srgbRed: 186 / 255, green: 222 / 255, blue: 1, alpha: 1) },
    ]

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.borderType = .noBorder
        scrollView.hasVerticalScroller = true
        scrollView.autohidesScrollers = true

        // TextKit 1, for temporary attributes.
        let textView = NSTextView(usingTextLayoutManager: false)
        textView.minSize = .zero
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: .greatestFiniteMagnitude)
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainer?.widthTracksTextView = true
        textView.textContainer?.containerSize = NSSize(width: scrollView.contentSize.width, height: .greatestFiniteMagnitude)
        scrollView.documentView = textView

        textView.delegate = context.coordinator
        textView.font = Self.font
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
        textView.regexPlaceholder = placeholder.map {
            NSAttributedString(string: $0, attributes: [.foregroundColor: NSColor.placeholderTextColor, .font: Self.font])
        }
        update(textView, context: context)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        let textView = scrollView.documentView as! NSTextView
        context.coordinator.text = $text
        if textView.string != text {
            // Paste, Clear or Use as Test String: one undoable change.
            let all = NSRange(location: 0, length: (textView.string as NSString).length)
            if textView.shouldChangeText(in: all, replacementString: text) {
                textView.textStorage?.replaceCharacters(in: all, with: text)
                textView.didChangeText()
            }
            textView.setSelectedRange(NSRange(location: 0, length: 0))
        }
        update(textView, context: context)
    }

    private func update(_ textView: NSTextView, context: Context) {
        let coordinator = context.coordinator
        if let highlights, highlights != coordinator.applied, highlights.text == textView.string {
            apply(highlights, to: textView)
            coordinator.applied = highlights
        }
        if reveal != coordinator.revealed {
            coordinator.revealed = reveal
            let length = (textView.string as NSString).length
            if let reveal, NSMaxRange(reveal) <= length {
                textView.scrollRangeToVisible(reveal)
                if reveal.length > 0 { textView.showFindIndicator(for: reveal) }
            }
        }
    }

    private func apply(_ highlights: RegexHighlights, to textView: NSTextView) {
        guard let layoutManager = textView.layoutManager else { return }
        let length = (textView.string as NSString).length
        let all = NSRange(location: 0, length: length)
        for key in [NSAttributedString.Key.backgroundColor, .underlineStyle, .underlineColor] {
            layoutManager.removeTemporaryAttribute(key, forCharacterRange: all)
        }
        for (index, range) in highlights.matches.enumerated() where !range.isEmpty && range.upperBound <= length {
            layoutManager.addTemporaryAttribute(
                .backgroundColor,
                value: Self.matchColors[index % 2],
                forCharacterRange: NSRange(range)
            )
        }
        for range in highlights.groups where !range.isEmpty && range.upperBound <= length {
            layoutManager.addTemporaryAttributes(
                [.underlineStyle: NSUnderlineStyle.thick.rawValue, .underlineColor: NSColor.systemOrange],
                forCharacterRange: NSRange(range)
            )
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>
        var applied: RegexHighlights?
        var revealed: NSRange?

        init(text: Binding<String>) {
            self.text = text
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            // Edits shift the highlights, so the next result must be applied even if it's the
            // same as the last one (e.g. after typing and undoing).
            applied = nil
            if text.wrappedValue != textView.string {
                text.wrappedValue = textView.string
            }
        }
    }
}

private extension NSAppearance {
    var isDark: Bool {
        bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
    }
}

private extension NSTextView {
    /// The same private placeholder `CodeEditor` uses.
    var regexPlaceholder: NSAttributedString? {
        get { value(forKey: "placeholderAttributedString") as? NSAttributedString }
        set { setValue(newValue, forKey: "placeholderAttributedString") }
    }
}
