import SwiftUI
import ToolboxBindings

@Observable
final class NamesModel {
    var kind = NamesKind.project {
        didSet { if kind != oldValue { regenerate() } }
    }
    /// The case style for project names.
    var style = NamesCase.kebab {
        didSet { if style != oldValue { regenerate() } }
    }
    var count: UInt32 = 10 {
        didSet { if count != oldValue { regenerate() } }
    }
    private(set) var names: [String] = []

    init() {
        regenerate()
    }

    func regenerate() {
        names = namesGenerate(kind: kind, style: style, count: count)
    }
}

extension NamesKind: CaseIterable {
    public static let allCases: [NamesKind] = [.project, .username, .person, .company]
}

extension NamesCase: CaseIterable {
    public static let allCases: [NamesCase] = [.kebab, .snake, .pascal, .camel, .title, .flat]
}

struct NamesView: View {
    @Bindable var model: NamesModel

    var body: some View {
        Form {
            Section {
                Picker("Kind", selection: $model.kind) {
                    ForEach(NamesKind.allCases, id: \.self) { kind in
                        Text(namesKindLabel(kind: kind))
                    }
                }
                .pickerStyle(.segmented)
                if model.kind == .project {
                    Picker("Case", selection: $model.style) {
                        ForEach(NamesCase.allCases, id: \.self) { style in
                            Text(namesCaseLabel(style: style))
                        }
                    }
                }
                SliderField("Count", value: $model.count, in: 1...namesMaxCount())
            }

            Section {
                ForEach(Array(model.names.enumerated()), id: \.offset) { _, name in
                    HStack(alignment: .firstTextBaseline) {
                        Text(name)
                            .font(.body.monospaced())
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        CopyButton(text: name)
                            .labelStyle(.iconOnly)
                            .buttonStyle(.borderless)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItemGroup {
                Button("Regenerate", systemImage: "arrow.clockwise") { model.regenerate() }
                    .keyboardShortcut("r")
                    .help("Generate new names")
                CopyButton(title: "Copy All", text: model.names.joined(separator: "\n"))
            }
        }
    }
}

