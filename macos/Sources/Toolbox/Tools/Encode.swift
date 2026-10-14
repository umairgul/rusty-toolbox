import SwiftUI
import ToolboxBindings

@Observable
final class EncodeModel {
    var input = ""
    var codec = EncodeCodec.base64
    var direction = EncodeDirection.encode
    var options = encodeDefaultOptions()
    var wrapsLines = true
    private(set) var outcome: EncodeOutcome?

    var output: String? {
        if case .ok(let output) = outcome { output } else { nil }
    }

    /// Uses the result as input and reverses the direction.
    func swap() {
        guard let output else { return }
        input = output
        direction = direction == .encode ? .decode : .encode
    }

    fileprivate var request: Request {
        Request(input: input, codec: codec, direction: direction, options: options)
    }

    fileprivate func process(_ request: Request) async {
        if let outcome = await offMainThread({
            encodeProcess(codec: request.codec, direction: request.direction, input: request.input, options: request.options)
        }) {
            self.outcome = outcome
        }
    }

    fileprivate struct Request: Equatable {
        let input: String
        let codec: EncodeCodec
        let direction: EncodeDirection
        let options: EncodeOptions
    }
}

extension EncodeCodec: CaseIterable {
    public static let allCases: [EncodeCodec] = [.base64, .url, .html, .hex]

    var label: String { encodeCodecLabel(codec: self) }
}

struct EncodeView: View {
    @Bindable var model: EncodeModel

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Input") {
                    CodeEditor(
                        text: $model.input,
                        placeholder: model.direction == .encode ? "Text to encode" : "Text to decode",
                        wrapsLines: model.wrapsLines
                    )
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
                    switch model.outcome {
                    case .ok(let output):
                        CopyButton(title: "Copy Result", text: output)
                    case .notText(_, let hex):
                        CopyButton(title: "Copy Hex", text: hex)
                    case .invalid, nil:
                        EmptyView()
                    }
                }
            }
            StatusBar { status }
        }
        .task(id: model.request) {
            await model.process(model.request)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Codec", selection: $model.codec) {
                    ForEach(EncodeCodec.allCases, id: \.self) { Text($0.label) }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .help("What to encode to or decode from")

                Picker("Direction", selection: $model.direction) {
                    Text("Encode").tag(EncodeDirection.encode)
                    Text("Decode").tag(EncodeDirection.decode)
                }
                .pickerStyle(.segmented)
                .help("Encode text, or decode it back")

                Button("Swap", systemImage: "arrow.left.arrow.right") { model.swap() }
                    .disabled(model.output == nil)
                    .help("Use the result as input and reverse the direction")
            }
            ToolbarItemGroup { options }
            ToolbarItem {
                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines")
            }
        }
    }

    /// The options for the selected codec. Hex has none.
    @ViewBuilder private var options: some View {
        switch model.codec {
        case .base64:
            Toggle("URL-Safe", systemImage: "link", isOn: $model.options.base64UrlSafe)
                .help(model.direction == .encode
                    ? "Use the URL-safe alphabet (- and _ instead of + and /)"
                    : "Decoding accepts either alphabet, with or without padding")
            Toggle("Padding", systemImage: "equal.square", isOn: $model.options.base64Padding)
                .disabled(model.direction == .decode)
                .help(model.direction == .encode
                    ? "End with = padding"
                    : "Decoding accepts either alphabet, with or without padding")
        case .url:
            Toggle("Spaces as +", systemImage: "plus.square", isOn: $model.options.urlPlusForSpace)
                .help("Encode spaces as + and decode + as a space, as HTML forms do")
        case .html:
            Toggle("Encode Non-ASCII", systemImage: "globe", isOn: $model.options.htmlEncodeNonAscii)
                .disabled(model.direction == .decode)
                .help("Encode all non-ASCII characters as numeric entities")
        case .hex:
            EmptyView()
        }
    }

    @ViewBuilder private var output: some View {
        switch model.outcome {
        case .ok(let output):
            CodeEditor(readOnly: output, wrapsLines: model.wrapsLines)
        case .notText(let message, let hex):
            VStack(alignment: .leading, spacing: 0) {
                VStack(alignment: .leading, spacing: 4) {
                    ErrorLabel(message: message)
                    Text("As hex:")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                .padding(10)
                Divider()
                CodeEditor(readOnly: hex, wrapsLines: model.wrapsLines)
            }
        case .invalid(let message):
            VStack(alignment: .leading) {
                ErrorLabel(message: message)
                Spacer()
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
        case nil:
            Color.clear
        }
    }

    @ViewBuilder private var status: some View {
        switch model.outcome {
        case .ok(let output):
            if model.codec == .base64 && model.direction == .decode {
                Text("Decoding accepts either alphabet, with or without padding.")
                    .foregroundStyle(.secondary)
            }
            Spacer()
            // Counted as Unicode scalars, like the Rust core counts characters.
            Text("\(model.input.unicodeScalars.count) characters in, \(output.unicodeScalars.count) out")
                .foregroundStyle(.secondary)
                .monospacedDigit()
        case .notText(let message, _), .invalid(let message):
            StatusLabel(ok: false, message: message)
        case nil:
            EmptyView()
        }
    }
}

private struct ErrorLabel: View {
    let message: String

    var body: some View {
        Label(message, systemImage: "exclamationmark.triangle.fill")
            .symbolRenderingMode(.multicolor)
            .textSelection(.enabled)
    }
}
