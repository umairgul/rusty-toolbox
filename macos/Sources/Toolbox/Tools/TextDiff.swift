import SwiftUI
import ToolboxBindings

@Observable
final class TextDiffModel {
    /// Rows drawn. Larger diffs are cut off with a note; the unified diff stays complete.
    nonisolated static let maxRows: UInt32 = 2000

    var original = """
        fn greet(name: &str) {
            println!("Hello, {name}!");
        }

        fn main() {
            greet("world");
        }
        """
    var changed = """
        fn greet(name: &str, excited: bool) {
            println!("Hello, {name}!");
        }

        fn main() {
            greet("Toolbox", true);
            greet("world", false);
        }
        """
    var ignoreWhitespace = false
    var ignoreCase = false
    var wrapsLines = true
    private(set) var result: TextDiffResult?

    fileprivate var options: TextDiffOptions {
        TextDiffOptions(ignoreWhitespace: ignoreWhitespace, ignoreCase: ignoreCase)
    }

    func swapSides() {
        (original, changed) = (changed, original)
    }

    fileprivate func compare(_ key: TextDiffView.TaskKey) async {
        let result = await offMainThread {
            textDiffCompare(original: key.original, changed: key.changed, options: key.options, maxRows: Self.maxRows)
        }
        if let result { self.result = result }
    }
}

struct TextDiffView: View {
    @Bindable var model: TextDiffModel

    var body: some View {
        VStack(spacing: 0) {
            VSplitView {
                HSplitView {
                    input("Original", text: $model.original)
                    input("Changed", text: $model.changed)
                }
                .frame(minHeight: 150, idealHeight: 230)
                Pane("Differences") {
                    differences
                } accessories: {
                    CopyButton(title: "Copy Unified Diff", text: model.result?.unified ?? "")
                        .labelStyle(.titleAndIcon)
                        .help("Copy a unified diff (diff -u) of the two texts")
                }
                .frame(minHeight: 180)
            }
            StatusBar { status }
        }
        .task(id: TaskKey(original: model.original, changed: model.changed, options: model.options)) {
            await model.compare(TaskKey(original: model.original, changed: model.changed, options: model.options))
        }
        .toolbar {
            ToolbarItemGroup {
                Toggle("Ignore Whitespace", systemImage: "space", isOn: $model.ignoreWhitespace)
                    .help("Ignore whitespace: treat lines as equal if they differ only in spacing")
                Toggle("Ignore Case", systemImage: "textformat", isOn: $model.ignoreCase)
                    .help("Ignore case: treat lines as equal if they differ only in upper and lower case")
                Button("Swap Sides", systemImage: "arrow.left.arrow.right") { model.swapSides() }
                    .help("Swap the original and changed texts")
                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines in the editors")
            }
        }
    }

    private func input(_ title: String, text: Binding<String>) -> some View {
        Pane(title) {
            CodeEditor(text: text, placeholder: "Paste text", wrapsLines: model.wrapsLines)
        } accessories: {
            Button("Paste", systemImage: "doc.on.clipboard") {
                text.wrappedValue = Pasteboard.string ?? text.wrappedValue
            }
            .help("Replace this text with the clipboard")
            Button("Clear", systemImage: "xmark.circle") { text.wrappedValue = "" }
                .help("Clear this text")
                .disabled(text.wrappedValue.isEmpty)
        }
    }

    @ViewBuilder private var differences: some View {
        if let result = model.result {
            if result.added == 0 && result.removed == 0 {
                ContentUnavailableView(
                    "No Differences",
                    systemImage: "checkmark.circle",
                    description: Text(model.ignoreWhitespace || model.ignoreCase
                        ? "The texts are the same, apart from what's ignored."
                        : "The texts are the same.")
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(result.rows.indices, id: \.self) { index in
                            DiffRowView(row: result.rows[index])
                        }
                    }
                    .padding(.vertical, 4)
                }
                .background(Color(nsColor: .textBackgroundColor))
            }
        } else {
            Color.clear
        }
    }

    @ViewBuilder private var status: some View {
        if let result = model.result {
            if result.added == 0 && result.removed == 0 {
                StatusLabel(ok: true, message: "No differences")
            } else {
                Text("+\(result.added) added")
                    .foregroundStyle(.green)
                    .fontWeight(.semibold)
                Text("−\(result.removed) removed")
                    .foregroundStyle(.red)
                    .fontWeight(.semibold)
                Spacer()
                if result.totalRows > result.rows.count {
                    Text("Showing the first \(result.rows.count) of \(result.totalRows) lines. The unified diff has everything.")
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
        }
    }

    struct TaskKey: Equatable {
        let original: String
        let changed: String
        let options: TextDiffOptions
    }
}

/// One row of the side-by-side diff: the original line on the left, the changed one on the
/// right, each with its line number and changed words highlighted.
private struct DiffRowView: View {
    let row: TextDiffRow

    var body: some View {
        HStack(spacing: 0) {
            side(row.left, tint: .red)
            Divider()
            side(row.right, tint: .green)
        }
        .fixedSize(horizontal: false, vertical: true)
    }

    private func side(_ side: TextDiffSide?, tint: Color) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 0) {
            Text(side.map { "\($0.line)" } ?? "")
                .foregroundStyle(.tertiary)
                .frame(width: 40, alignment: .trailing)
                .padding(.trailing, 8)
            Text(side.map { text($0, tint: tint) } ?? AttributedString(" "))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.body.monospaced())
        .padding(.vertical, 1)
        .padding(.trailing, 6)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(background(side, tint: tint))
    }

    private func text(_ side: TextDiffSide, tint: Color) -> AttributedString {
        var text = AttributedString()
        for segment in side.segments {
            var part = AttributedString(segment.text)
            if segment.changed { part.backgroundColor = tint.opacity(0.35) }
            text += part
        }
        // An empty line still takes a line's height.
        return text.characters.isEmpty ? AttributedString(" ") : text
    }

    private func background(_ side: TextDiffSide?, tint: Color) -> some ShapeStyle {
        switch (side, row.kind) {
        case (nil, _): AnyShapeStyle(.quaternary.opacity(0.5))
        case (_, .equal): AnyShapeStyle(.clear)
        case _: AnyShapeStyle(tint.opacity(0.13))
        }
    }
}
