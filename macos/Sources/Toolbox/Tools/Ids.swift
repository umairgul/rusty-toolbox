import SwiftUI
import ToolboxBindings

@Observable
final class IdsModel {
    var options: IdsOptions = {
        var options = idsDefaultOptions()
        options.count = 5
        return options
    }() {
        didSet { if options != oldValue { regenerate() } }
    }
    /// Fails when the options are invalid, e.g. a Nano ID alphabet of one character.
    private(set) var ids: Result<[String], any Error> = .success([])

    init() {
        regenerate()
    }

    func regenerate() {
        ids = Result { try idsGenerate(options: options) }
    }

    var text: String {
        (try? ids.get())?.joined(separator: "\n") ?? ""
    }
}

extension IdsKind: CaseIterable {
    public static let allCases: [IdsKind] = [.uuidV4, .uuidV7, .ulid, .nanoId]
}

struct IdsView: View {
    @Bindable var model: IdsModel

    var body: some View {
        Form {
            Section {
                Picker("Type", selection: $model.options.kind) {
                    ForEach(IdsKind.allCases, id: \.self) { kind in
                        Text(idsKindInfo(kind: kind).label)
                    }
                }
                .pickerStyle(.segmented)
            } footer: {
                FormNote(idsKindInfo(kind: model.options.kind).description)
            }

            Section("Options") {
                SliderField("Count", value: $model.options.count, in: 1...idsMaxCount(), logarithmic: true)
                switch model.options.kind {
                case .uuidV4, .uuidV7:
                    Toggle("Hyphens", isOn: $model.options.uuidHyphens)
                    Toggle("Uppercase", isOn: $model.options.uuidUppercase)
                case .ulid:
                    Toggle("Lowercase", isOn: $model.options.ulidLowercase)
                case .nanoId:
                    SliderField("Length", value: $model.options.nanoidLength, in: 1...idsMaxNanoidLength(), logarithmic: true)
                    NanoIdAlphabetField(alphabet: $model.options.nanoidAlphabet)
                }
            }

            Section {
                switch model.ids {
                case .success(let ids):
                    CodeEditor(readOnly: model.text, wrapsLines: false)
                        .frame(height: CGFloat(min(max(ids.count, 1), 16)) * 16 + 18)
                        .clipShape(.rect(cornerRadius: 6))
                case .failure(let error):
                    Label(error.message, systemImage: "exclamationmark.triangle.fill")
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItemGroup {
                Button("Regenerate", systemImage: "arrow.clockwise") { model.regenerate() }
                    .keyboardShortcut("r")
                    .help("Generate new IDs")
                CopyButton(title: model.options.count == 1 ? "Copy" : "Copy All", text: model.text)
            }
        }
    }
}

/// A menu of common alphabets ("Custom" when the text matches none), and the alphabet itself.
private struct NanoIdAlphabetField: View {
    @Binding var alphabet: String
    private let presets = idsNanoidAlphabetPresets()

    var body: some View {
        Picker("Alphabet", selection: preset) {
            ForEach(presets, id: \.alphabet) { preset in
                Text(preset.name).tag(String?.some(preset.alphabet))
            }
            if preset.wrappedValue == nil {
                Divider()
                Text("Custom").tag(String?.none)
            }
        }
        LabeledContent("Characters") {
            TextField("Characters", text: $alphabet, prompt: Text("Characters to use"))
                .labelsHidden()
                .textFieldStyle(.roundedBorder)
                .font(.body.monospaced())
                .multilineTextAlignment(.leading)
        }
    }

    private var preset: Binding<String?> {
        Binding {
            presets.first { $0.alphabet == alphabet }?.alphabet
        } set: {
            if let preset = $0 { alphabet = preset }
        }
    }
}


