import SwiftUI
import ToolboxBindings

@Observable
final class ContrastModel {
    var foreground = RgbaColor(r: 0x93 / 255, g: 0xc5 / 255, b: 0xfd / 255, a: 1)
    /// Always opaque.
    var background = RgbaColor.white

    func swap() {
        let swapped = contrastSwap(foreground: foreground, background: background)
        foreground = swapped.foreground
        background = swapped.background
    }
}

struct ContrastView: View {
    @Bindable var model: ContrastModel

    var body: some View {
        let report = contrastCheck(foreground: model.foreground, background: model.background)
        Form {
            Section {
                LabeledContent("Text") {
                    ColorField(color: $model.foreground)
                }
                LabeledContent("Background") {
                    ColorField(color: $model.background, supportsOpacity: false)
                }
            }

            Section("Preview") {
                ContrastPreview(foreground: model.foreground, background: model.background)
                    .padding(.vertical, 4)
            }

            Section("Contrast") {
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Text(String(format: "%.2f : 1", report.ratio))
                        .font(.system(size: 28, weight: .bold))
                        .monospacedDigit()
                        .textSelection(.enabled)
                    Text(report.verdict)
                        .font(.title3)
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 2)
                ForEach(report.levels, id: \.label) { level in
                    LabeledContent(level.label) {
                        HStack(spacing: 16) {
                            Text("\(level.requiredRatio.formatted()):1")
                                .foregroundStyle(.secondary)
                                .monospacedDigit()
                            StatusLabel(ok: level.passes, message: level.passes ? "Pass" : "Fail")
                                .frame(width: 56, alignment: .leading)
                        }
                    }
                }
            }

            if !report.suggestions.isEmpty {
                Section {
                    ForEach(report.suggestions, id: \.level) { suggestion in
                        SuggestionRow(suggestion: suggestion) { model.foreground = $0 }
                    }
                } header: {
                    Text("Suggestions")
                } footer: {
                    FormNote("The text color with the same hue and the smallest change in lightness that passes.")
                }
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItem {
                Button("Swap Colors", systemImage: "arrow.up.arrow.down") { model.swap() }
                    .help("Swap the text and background colors")
            }
        }
    }
}

/// Sample text and a button outline in the chosen colors.
private struct ContrastPreview: View {
    let foreground: RgbaColor
    let background: RgbaColor

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 6, style: .continuous)
        VStack(alignment: .leading, spacing: 6) {
            Text("Large text, 24 px")
                .font(.system(size: 24))
            Text("Normal body text at 16 px. The quick brown fox jumps over the lazy dog.")
                .font(.system(size: 16))
            Text("Small print at 12 px")
                .font(.system(size: 12))
            Text("Button")
                .font(.system(size: 14))
                .frame(width: 120, height: 30)
                .overlay(RoundedRectangle(cornerRadius: 6, style: .continuous).strokeBorder(lineWidth: 2))
                .padding(.top, 6)
        }
        .foregroundStyle(foreground.swiftUI)
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(background.withAlpha(1).swiftUI, in: shape)
        .overlay(shape.strokeBorder(.separator))
    }
}

private struct SuggestionRow: View {
    let suggestion: ContrastSuggestion
    let use: (RgbaColor) -> Void

    var body: some View {
        if let color = suggestion.color {
            HStack(spacing: 10) {
                Swatch(color: color, cornerRadius: 4)
                    .frame(width: 28, height: 20)
                Text("\(Text(color.hex).monospaced()) passes \(suggestion.level) (\(String(format: "%.2f", suggestion.ratio)):1), keeping the same hue.")
                    .textSelection(.enabled)
                Spacer()
                Button("Use Color") { use(color) }
                    .help("Use \(color.hex) as the text color")
            }
        } else {
            Text("No shade of this hue reaches \(suggestion.target.formatted()):1 on this background.")
                .foregroundStyle(.secondary)
        }
    }
}
