import SwiftUI
import ToolboxBindings

@Observable
final class SqlFormatterModel {
    var input = "select u.id, u.name, count(o.id) as orders from users u left join orders o on o.user_id = u.id where u.active = 1 and o.created_at >= '2024-01-01' group by u.id, u.name having count(o.id) > 5 order by orders desc limit 10;"
    var keywordCase = SqlKeywordCase.upper
    var indent = SqlIndent.twoSpaces
    var dialect = SqlDialect.generic
    var compact = false
    var wrapsLines = true
    private(set) var output: String?

    fileprivate var options: SqlFormatOptions {
        SqlFormatOptions(indent: indent, keywordCase: keywordCase, dialect: dialect, compact: compact)
    }

    fileprivate func format(_ input: String, _ options: SqlFormatOptions) async {
        if let output = await offMainThread({ sqlFormat(input: input, options: options) }) {
            self.output = output
        }
    }
}

extension SqlKeywordCase: CaseIterable {
    public static let allCases: [SqlKeywordCase] = [.upper, .lower, .preserve]
}

extension SqlIndent: CaseIterable {
    public static let allCases: [SqlIndent] = [.twoSpaces, .fourSpaces, .tab]
}

extension SqlDialect: CaseIterable {
    public static let allCases: [SqlDialect] = [.generic, .postgreSql, .sqlServer]
}

struct SqlFormatterView: View {
    @Bindable var model: SqlFormatterModel

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Input") {
                    CodeEditor(text: $model.input, placeholder: "Paste SQL", wrapsLines: model.wrapsLines)
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
                    if let output = model.output, !output.isEmpty {
                        Button("Use as Input", systemImage: "arrow.uturn.left") { model.input = output }
                            .help("Replace the input with this result")
                        CopyButton(text: output)
                    }
                }
            }
            StatusBar { status }
        }
        .task(id: TaskKey(input: model.input, options: model.options)) {
            await model.format(model.input, model.options)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Keywords", selection: $model.keywordCase) {
                    ForEach(SqlKeywordCase.allCases, id: \.self) { Text(sqlKeywordCaseLabel(keywordCase: $0)) }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .help("Case of SQL keywords such as SELECT and FROM")

                Picker("Indent", selection: $model.indent) {
                    ForEach(SqlIndent.allCases, id: \.self) { Text(sqlIndentLabel(indent: $0)) }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .disabled(model.compact)
                .help("Indentation for formatted output")

                Picker("Dialect", selection: $model.dialect) {
                    ForEach(SqlDialect.allCases, id: \.self) { Text(sqlDialectLabel(dialect: $0)) }
                }
                .pickerStyle(.menu)
                .fixedSize()
                .help("SQL dialect, for its placeholders, operators and quoting")

                Toggle("Compact", systemImage: "arrow.right.to.line", isOn: $model.compact)
                    .help("Compact: put each query on one line")

                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines")
            }
        }
    }

    @ViewBuilder private var output: some View {
        if let output = model.output {
            if output.isEmpty {
                ContentUnavailableView("No SQL", systemImage: "cylinder.split.1x2", description: Text("Type or paste a query to format it."))
            } else {
                CodeEditor(readOnly: output, wrapsLines: model.wrapsLines)
            }
        } else {
            Color.clear
        }
    }

    @ViewBuilder private var status: some View {
        if let output = model.output, !output.isEmpty {
            let lines = output.split(separator: "\n", omittingEmptySubsequences: false).count
            Text(lines == 1 ? "1 line" : "\(lines) lines")
                .foregroundStyle(.secondary)
                .monospacedDigit()
            Spacer()
            Text("\(model.input.utf8.count.byteCount) in · \(output.utf8.count.byteCount) out")
                .foregroundStyle(.secondary)
                .monospacedDigit()
        }
    }

    private struct TaskKey: Equatable {
        let input: String
        let options: SqlFormatOptions
    }
}
