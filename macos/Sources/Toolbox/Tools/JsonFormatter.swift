import SwiftUI
import ToolboxBindings

@Observable
final class JsonFormatterModel {
    enum Mode: Hashable { case format, minify }

    var input = #"{"name":"Toolbox","version":"0.1.0","price":19.90,"tags":["rust","swiftui"],"window":{"width":900,"height":640},"beta":false,"owner":null}"#
    var mode = Mode.format
    var indent = JsonIndent.twoSpaces
    var sortKeys = false
    var wrapsLines = true
    private(set) var outcome: JsonOutcome?

    fileprivate var request: JsonMode {
        switch mode {
        case .format: .format(indent: indent, sortKeys: sortKeys)
        case .minify: .minify
        }
    }

    fileprivate func process(_ input: String, _ request: JsonMode) async {
        if let outcome = await offMainThread({ jsonProcess(input: input, mode: request) }) {
            self.outcome = outcome
        }
    }
}

extension JsonIndent: CaseIterable {
    public static let allCases: [JsonIndent] = [.twoSpaces, .fourSpaces, .tab]

    var label: String {
        switch self {
        case .twoSpaces: "2 Spaces"
        case .fourSpaces: "4 Spaces"
        case .tab: "Tabs"
        }
    }
}

struct JsonFormatterView: View {
    @Bindable var model: JsonFormatterModel

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Input") {
                    CodeEditor(text: $model.input, placeholder: "Paste JSON", wrapsLines: model.wrapsLines)
                } accessories: {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        model.input = Pasteboard.string ?? model.input
                    }
                    .help("Replace the input with the clipboard")
                    Button("Clear", systemImage: "xmark.circle") { model.input = "" }
                        .help("Clear the input")
                        .disabled(model.input.isEmpty)
                }
                Pane("Output") {
                    output
                } accessories: {
                    if case .ok(let output) = model.outcome {
                        Button("Use as Input", systemImage: "arrow.uturn.left") { model.input = output }
                            .help("Replace the input with this result")
                        CopyButton(text: output)
                    }
                }
            }
            StatusBar { status }
        }
        .task(id: TaskKey(input: model.input, request: model.request)) {
            await model.process(model.input, model.request)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Mode", selection: $model.mode) {
                    Text("Format").tag(JsonFormatterModel.Mode.format)
                    Text("Minify").tag(JsonFormatterModel.Mode.minify)
                }
                .pickerStyle(.segmented)
                .help("Format with indentation, or minify onto one line")

                Picker("Indent", selection: $model.indent) {
                    ForEach(JsonIndent.allCases, id: \.self) { Text($0.label) }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .disabled(model.mode == .minify)
                .help("Indentation for formatted output")

                Toggle("Sort Keys", systemImage: "arrow.up.arrow.down", isOn: $model.sortKeys)
                    .disabled(model.mode == .minify)
                    .help("Sort object keys. This parses the JSON, so very large integers and float formatting may change.")

                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines")
            }
        }
    }

    @ViewBuilder private var output: some View {
        switch model.outcome {
        case .ok(let output):
            CodeEditor(readOnly: output, wrapsLines: model.wrapsLines)
        case .invalid(let error):
            JsonErrorView(error: error)
        case nil:
            Color.clear
        }
    }

    @ViewBuilder private var status: some View {
        switch model.outcome {
        case .ok(let output):
            StatusLabel(ok: true, message: "Valid JSON")
            Spacer()
            Text("\(model.input.utf8.count.byteCount) in · \(output.utf8.count.byteCount) out")
                .foregroundStyle(.secondary)
                .monospacedDigit()
        case .invalid(let error) where error.line > 0:
            StatusLabel(ok: false, message: "Line \(error.line), column \(error.column)")
        case .invalid:
            StatusLabel(ok: false, message: "No JSON")
        case nil:
            EmptyView()
        }
    }

    private struct TaskKey: Equatable {
        let input: String
        let request: JsonMode
    }
}

private struct JsonErrorView: View {
    let error: JsonSyntaxError

    var body: some View {
        if error.line == 0 {
            ContentUnavailableView(error.message, systemImage: "curlybraces")
        } else {
            VStack(alignment: .leading, spacing: 12) {
                Label(error.message, systemImage: "exclamationmark.triangle.fill")
                    .symbolRenderingMode(.multicolor)
                    .textSelection(.enabled)
                if let context = error.context {
                    Text(context)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: 6))
                }
                Spacer()
            }
            .padding()
        }
    }
}
