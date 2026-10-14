import SwiftUI
import ToolboxBindings

@Observable
final class ColorPickerModel {
    static let maxRecent = 16

    var color = RgbaColor.blue
    private(set) var recent: [RgbaColor] = []

    func remember(_ color: RgbaColor) {
        recent.removeAll { $0.hex == color.hex }
        recent.insert(color, at: 0)
        recent = Array(recent.prefix(Self.maxRecent))
    }

    func clearRecent() {
        recent.removeAll()
    }
}

struct ColorPickerView: View {
    @Bindable var model: ColorPickerModel

    var body: some View {
        Form {
            Section {
                HStack(spacing: 16) {
                    Swatch(color: model.color, cornerRadius: 10)
                        .frame(width: 112, height: 72)
                    VStack(alignment: .leading, spacing: 2) {
                        let name = colorNearestName(color: model.color)
                        Text(name.exact ? name.name : "Close to \(name.name)")
                            .font(.title2.weight(.semibold))
                        Text(name.exact ? "CSS named color" : "Nearest CSS named color")
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 4)
                LabeledContent("Color") {
                    ColorField(color: $model.color) { model.remember($0) }
                }
            }

            Section("Formats") {
                ForEach(colorFormats(color: model.color), id: \.label) { format in
                    ValueRow(format.label, value: format.value)
                }
            }

            if !model.recent.isEmpty {
                Section {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 32, maximum: 32), spacing: 8)], alignment: .leading, spacing: 8) {
                        ForEach(model.recent, id: \.hex) { color in
                            Button { model.color = color } label: {
                                Swatch(color: color).frame(width: 32, height: 32)
                            }
                            .buttonStyle(.plain)
                            .help(color.hex)
                        }
                    }
                    .padding(.vertical, 4)
                } header: {
                    HStack {
                        Text("Recent")
                        Spacer()
                        Button("Clear") { model.clearRecent() }
                            .buttonStyle(.link)
                            .font(.callout)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItem {
                Button("Save to Recent", systemImage: "plus.square.on.square") {
                    model.remember(model.color)
                }
                .help("Add this color to Recent")
            }
        }
    }
}
