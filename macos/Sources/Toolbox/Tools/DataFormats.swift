import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class DataFormatsModel {
    var input = """
    {
      "name": "Toolbox",
      "version": "0.1.0",
      "tools": ["hashes", "timestamps"],
      "window": { "width": 900, "height": 640 }
    }
    """
    /// nil detects the input format.
    var from: DataFormat?
    var to = DataFormat.yaml
    var options = dataFormatsDefaultOptions()
    var wrapsLines = true
    private(set) var conversion: DataFormatsConversion?

    /// The format the input is read as.
    var source: DataFormat { from ?? conversion?.detected ?? .json }

    var output: String? {
        if case .ok(let output) = conversion?.outcome { output } else { nil }
    }

    fileprivate var request: Request {
        Request(input: input, from: from, to: to, options: options)
    }

    fileprivate func convert(_ request: Request) async {
        let result = await offMainThread {
            dataFormatsConvert(input: request.input, from: request.from, to: request.to, options: request.options)
        }
        if let result { conversion = result }
    }

    /// Converts the output back: it becomes the input, and the formats trade places.
    func swap() {
        guard let output else { return }
        let source = source
        input = output
        from = to
        to = source
    }

    fileprivate struct Request: Equatable {
        let input: String
        let from: DataFormat?
        let to: DataFormat
        let options: DataFormatsOptions
    }
}

extension DataFormat: CaseIterable {
    public static let allCases: [DataFormat] = dataFormatsAll()

    var info: DataFormatInfo { dataFormatInfo(format: self) }
}

struct DataFormatsView: View {
    @Bindable var model: DataFormatsModel
    @ViewState private var saveMessage: SaveMessage?

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Input · \(model.source.info.label)") {
                    CodeEditor(text: $model.input, placeholder: "Paste JSON, YAML, TOML or CSV", wrapsLines: model.wrapsLines)
                } accessories: {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        model.input = Pasteboard.string ?? model.input
                    }
                    .help("Replace the input with the clipboard")
                    Button("Clear", systemImage: "xmark.circle") { model.input = "" }
                        .help("Clear the input")
                        .disabled(model.input.isEmpty)
                }
                Pane("Output · \(model.to.info.label)") {
                    output
                } accessories: {
                    if let output = model.output {
                        Button("Save…", systemImage: "square.and.arrow.down") { save(output) }
                            .help("Save the result to a file")
                        CopyButton(text: output)
                    }
                }
            }
            StatusBar { status }
        }
        .task(id: model.request) {
            await model.convert(model.request)
        }
        .onChange(of: model.request) { saveMessage = nil }
        .toolbar {
            ToolbarItemGroup {
                Picker("From", selection: $model.from) {
                    Text("Auto-Detect (\(model.conversion?.detected.info.label ?? "…"))")
                        .tag(DataFormat?.none)
                    Divider()
                    ForEach(DataFormat.allCases, id: \.self) { format in
                        Text(format.info.label).tag(DataFormat?.some(format))
                    }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .help("Input format")

                Button("Swap", systemImage: "arrow.left.arrow.right") { model.swap() }
                    .disabled(model.output == nil)
                    .help("Swap: convert the output back")

                Picker("To", selection: $model.to) {
                    ForEach(DataFormat.allCases, id: \.self) { format in
                        Text(format.info.label)
                    }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .help("Output format")

                Toggle("Pretty", systemImage: "text.alignleft", isOn: $model.options.pretty)
                    .disabled(model.to != .json)
                    .help("Indent JSON output instead of writing it on one line")

                Toggle("Detect Numbers and Booleans", systemImage: "number", isOn: $model.options.inferCsvTypes)
                    .disabled(model.source != .csv)
                    .help("Read CSV cells like 42, 3.5, true and false as numbers and booleans instead of text")

                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines")
            }
        }
    }

    @ViewBuilder private var output: some View {
        if model.input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            ContentUnavailableView(
                "Nothing to Convert",
                systemImage: "doc.text",
                description: Text("Paste or type JSON, YAML, TOML or CSV.")
            )
        } else {
            switch model.conversion?.outcome {
            case .ok(let output):
                CodeEditor(readOnly: output, wrapsLines: model.wrapsLines)
            case .invalid(let message):
                VStack(alignment: .leading) {
                    Label(message, systemImage: "exclamationmark.triangle.fill")
                        .symbolRenderingMode(.multicolor)
                        .textSelection(.enabled)
                    Spacer()
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
            case nil:
                Color.clear
            }
        }
    }

    @ViewBuilder private var status: some View {
        if let conversion = model.conversion, !model.input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            let from = conversion.from.info.label
            switch conversion.outcome {
            case .ok(let output):
                StatusLabel(ok: true, message: "Converted \(from) to \(model.to.info.label)")
                if let saveMessage {
                    Text(saveMessage.text)
                        .foregroundStyle(saveMessage.failed ? .red : .secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Spacer()
                Text("\(model.input.utf8.count.byteCount) in · \(output.utf8.count.byteCount) out")
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            case .invalid:
                StatusLabel(ok: false, message: "Couldn't convert \(from) to \(model.to.info.label)")
            }
        }
    }

    private func save(_ output: String) {
        let info = model.to.info
        do {
            if let url = try Files.save(
                Data(output.utf8),
                suggestedName: "converted.\(info.extension)",
                allowedTypes: UTType(filenameExtension: info.extension).map { [$0] } ?? []
            ) {
                saveMessage = SaveMessage(text: "Saved to \(ImageFiles.abbreviated(url))", failed: false)
            }
        } catch {
            saveMessage = SaveMessage(text: "Couldn't save: \(error.message)", failed: true)
        }
    }

    private struct SaveMessage {
        let text: String
        let failed: Bool
    }
}
