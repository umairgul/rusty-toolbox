import SwiftUI
import ToolboxBindings

@Observable
final class LoremModel {
    var options = loremDefaultOptions() {
        didSet {
            // Switching to a unit with a lower limit brings the count down to it. Setting a
            // property in its own didSet doesn't call didSet again.
            options.count = min(options.count, loremMaxCount(unit: options.unit))
            if options != oldValue { regenerate() }
        }
    }
    private(set) var text = ""

    init() {
        regenerate()
    }

    func regenerate() {
        text = loremGenerate(options: options)
    }

    var stats: String {
        let words = text.split(whereSeparator: \.isWhitespace).count
        return "\(words.formatted()) words · \(text.unicodeScalars.count.formatted()) characters"
    }
}

extension LoremUnit: CaseIterable {
    public static let allCases: [LoremUnit] = [.paragraphs, .sentences, .words]
}

struct LoremView: View {
    @Bindable var model: LoremModel

    var body: some View {
        let maxCount = loremMaxCount(unit: model.options.unit)
        Form {
            Section {
                Picker("Generate", selection: $model.options.unit) {
                    ForEach(LoremUnit.allCases, id: \.self) { unit in
                        Text(loremUnitLabel(unit: unit))
                    }
                }
                .pickerStyle(.segmented)
                SliderField("Count", value: $model.options.count, in: 1...maxCount, logarithmic: maxCount > 100)
                Toggle("Start with “Lorem ipsum dolor sit amet…”", isOn: $model.options.classicStart)
                Toggle("Wrap paragraphs in <p> tags", isOn: $model.options.html)
            }

            Section {
                CodeEditor(readOnly: model.text)
                    .frame(height: 360)
                    .clipShape(.rect(cornerRadius: 6))
            } footer: {
                Text(model.stats)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItemGroup {
                Button("Regenerate", systemImage: "arrow.clockwise") { model.regenerate() }
                    .keyboardShortcut("r")
                    .help("Generate new text")
                CopyButton(text: model.text)
            }
        }
    }
}

