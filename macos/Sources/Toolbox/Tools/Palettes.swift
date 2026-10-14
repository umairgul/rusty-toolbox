import SwiftUI
import ToolboxBindings

@Observable
final class PalettesModel {
    /// Alpha is ignored: shades and harmonies are opaque.
    var base = RgbaColor.blue
    var name = "brand"
    var export = PaletteExport.cssVariables

    var exported: String {
        palettesExport(base: base, name: name, export: export)
    }
}

extension PaletteExport: CaseIterable {
    public static let allCases: [PaletteExport] = [.cssVariables, .tailwind, .json]

    var label: String {
        switch self {
        case .cssVariables: "CSS Variables"
        case .tailwind: "Tailwind v4"
        case .json: "JSON"
        }
    }
}

struct PalettesView: View {
    @Bindable var model: PalettesModel

    var body: some View {
        let scale = palettesScale(base: model.base)
        let exported = model.exported
        Form {
            Section {
                LabeledContent("Color") {
                    ColorField(color: $model.base, supportsOpacity: false)
                }
            }

            Section {
                HStack(alignment: .top, spacing: 6) {
                    ForEach(scale.shades, id: \.step) { shade in
                        CopySwatch(
                            color: shade.color,
                            caption: shade.step == scale.nearestStep ? "\(shade.step) (base)" : "\(shade.step)",
                            height: 48
                        )
                        .frame(maxWidth: .infinity)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Shades")
            } footer: {
                FormNote("Your color is closest to step \(scale.nearestStep). Click a swatch to copy it.")
            }

            Section("Harmonies") {
                ForEach(palettesHarmonies(base: model.base), id: \.label) { harmony in
                    LabeledContent(harmony.label) {
                        HStack(alignment: .top, spacing: 8) {
                            ForEach(Array(harmony.colors.enumerated()), id: \.offset) { _, color in
                                CopySwatch(color: color, height: 32)
                                    .frame(width: 64)
                            }
                        }
                        .padding(.vertical, 2)
                    }
                }
            }

            Section {
                Picker("Format", selection: $model.export) {
                    ForEach(PaletteExport.allCases, id: \.self) { Text($0.label) }
                }
                .pickerStyle(.segmented)
                TextField("Name", text: $model.name, prompt: Text("brand"))
                CodeEditor(readOnly: exported, wrapsLines: false)
                    .frame(height: 240)
            } header: {
                HStack {
                    Text("Export")
                    Spacer()
                    CopyButton(text: exported)
                        .buttonStyle(.link)
                        .font(.callout)
                }
            } footer: {
                FormNote("The shade scale, named after Name in lowercase with hyphens, e.g. \"Brand Blue\" becomes brand-blue.")
            }
        }
        .formStyle(.grouped)
    }
}

