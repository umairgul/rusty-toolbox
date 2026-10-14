import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class FakeDataModel {
    static let allFields = fakeDataAllFields()
    static let maxRows = Int(fakeDataMaxRows())

    var format = FakeDataFormat.csv
    var rows = 10
    var table = "users"
    var fields = Set(fakeDataDefaultFields())
    var showsOptions = true
    /// Bumped by Regenerate, so the same options produce new records.
    private(set) var generation = 0
    private(set) var outcome: Result<String, any Error>?

    fileprivate var options: FakeDataOptions {
        FakeDataOptions(
            rows: UInt32(clamping: rows),
            fields: Self.allFields.filter(fields.contains),
            format: format,
            table: table
        )
    }

    func regenerate() {
        generation += 1
    }

    fileprivate func generate(_ options: FakeDataOptions) async {
        if let outcome = await offMainThread({ Result { try fakeDataGenerate(options: options) } }) {
            self.outcome = outcome
        }
    }

    /// e.g. "users.csv".
    var suggestedFileName: String {
        "\(table.isEmpty ? "data" : table).\(fakeDataFormatInfo(format: format).extension)"
    }
}

extension FakeDataFormat: CaseIterable {
    public static let allCases: [FakeDataFormat] = [.csv, .json, .sql]
}

struct FakeDataView: View {
    @Bindable var model: FakeDataModel
    @ViewState private var saveError: String?

    var body: some View {
        let output = (try? model.outcome?.get()) ?? ""
        VStack(spacing: 0) {
            Pane("Output") {
                switch model.outcome {
                case .success(let output):
                    CodeEditor(readOnly: output, wrapsLines: false)
                case .failure(let error):
                    ContentUnavailableView("No Data", systemImage: "tablecells", description: Text(error.message))
                case nil:
                    Color.clear
                }
            } accessories: {
                CopyButton(title: "Copy All", text: output)
                Button("Save…", systemImage: "square.and.arrow.down") { save(output) }
                    .help("Save to a file")
                    .disabled(output.isEmpty)
            }
            StatusBar { status }
        }
        .task(id: TaskKey(options: model.options, generation: model.generation)) {
            await model.generate(model.options)
        }
        .inspector(isPresented: $model.showsOptions) {
            FakeDataOptionsForm(model: model)
                .inspectorColumnWidth(min: 260, ideal: 290, max: 400)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Format", selection: $model.format) {
                    ForEach(FakeDataFormat.allCases, id: \.self) { Text(fakeDataFormatInfo(format: $0).label) }
                }
                .pickerStyle(.segmented)
                .help("Output as CSV, a JSON array or SQL INSERT statements")

                Button("Regenerate", systemImage: "arrow.clockwise") { model.regenerate() }
                    .keyboardShortcut("r")
                    .help("Generate new records (⌘R)")

                Button("Options", systemImage: "sidebar.trailing") { model.showsOptions.toggle() }
                    .help(model.showsOptions ? "Hide options" : "Show options")
            }
        }
        .alert("Couldn't Save", isPresented: Binding { saveError != nil } set: { if !$0 { saveError = nil } }) {
        } message: {
            Text(saveError ?? "")
        }
    }

    @ViewBuilder private var status: some View {
        switch model.outcome {
        case .success(let output):
            let rows = model.options.rows
            let columns = model.options.fields.count
            Text("\(rows) \(rows == 1 ? "row" : "rows") · \(columns) \(columns == 1 ? "column" : "columns")")
                .foregroundStyle(.secondary)
                .monospacedDigit()
            Spacer()
            Text(output.utf8.count.byteCount)
                .foregroundStyle(.secondary)
                .monospacedDigit()
        case .failure(let error):
            StatusLabel(ok: false, message: error.message)
        case nil:
            EmptyView()
        }
    }

    private func save(_ output: String) {
        let type = UTType(filenameExtension: fakeDataFormatInfo(format: model.format).extension)
        do {
            try Files.save(Data(output.utf8), suggestedName: model.suggestedFileName, allowedTypes: type.map { [$0] } ?? [])
        } catch {
            saveError = error.message
        }
    }

    private struct TaskKey: Equatable {
        let options: FakeDataOptions
        let generation: Int
    }
}

private struct FakeDataOptionsForm: View {
    @Bindable var model: FakeDataModel

    var body: some View {
        Form {
            Section {
                LabeledContent("Rows") {
                    HStack(spacing: 4) {
                        TextField("Rows", value: $model.rows, format: .number)
                            .labelsHidden()
                            .multilineTextAlignment(.trailing)
                            .monospacedDigit()
                            .frame(width: 70)
                        Stepper("Rows", value: $model.rows, in: 1...FakeDataModel.maxRows)
                            .labelsHidden()
                    }
                }
                if model.format == .sql {
                    TextField("Table", text: $model.table, prompt: Text("users"))
                        .font(.body.monospaced())
                }
            } header: {
                Text("Output")
            } footer: {
                FormNote("Up to \(FakeDataModel.maxRows.formatted()) rows. Emails use example.com domains and phone numbers the 555-01xx range reserved for fiction, so no row points at a real person.")
            }

            Section {
                LazyVGrid(columns: [GridItem(.flexible(), alignment: .leading), GridItem(.flexible(), alignment: .leading)], alignment: .leading, spacing: 6) {
                    ForEach(FakeDataModel.allFields, id: \.self) { field in
                        Toggle(fakeDataFieldColumn(field: field), isOn: binding(for: field))
                            .toggleStyle(.checkbox)
                            .font(.body.monospaced())
                    }
                }
                .padding(.vertical, 4)
            } header: {
                HStack {
                    Text("Fields")
                    Spacer()
                    Button("Defaults") { model.fields = Set(fakeDataDefaultFields()) }
                        .buttonStyle(.link)
                        .font(.callout)
                }
            } footer: {
                FormNote("Emails and usernames are unique within each batch.")
            }
        }
        .formStyle(.grouped)
    }

    private func binding(for field: FakeDataField) -> Binding<Bool> {
        Binding {
            model.fields.contains(field)
        } set: { on in
            if on { model.fields.insert(field) } else { model.fields.remove(field) }
        }
    }
}
