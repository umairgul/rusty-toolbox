import AppKit
import SwiftUI
import ToolboxBindings

extension RgbaColor {
    static let blue = RgbaColor(r: 0x3b / 255, g: 0x82 / 255, b: 0xf6 / 255, a: 1)
    static let white = RgbaColor(r: 1, g: 1, b: 1, a: 1)
    static let black = RgbaColor(r: 0, g: 0, b: 0, a: 1)

    var swiftUI: Color {
        Color(.sRGB, red: r, green: g, blue: b, opacity: a)
    }

    /// Converts a system color (e.g. from the color panel or the eyedropper) to sRGB, rounded
    /// to 8 bits per channel so its hex code matches what was picked.
    init?(_ color: NSColor) {
        guard let srgb = color.usingColorSpace(.sRGB) else { return nil }
        self = colorRoundedTo8bit(color: RgbaColor(
            r: srgb.redComponent, g: srgb.greenComponent, b: srgb.blueComponent, a: srgb.alphaComponent
        ))
    }

    var hex: String { colorHex(color: self) }

    func withAlpha(_ a: Double) -> RgbaColor {
        RgbaColor(r: r, g: g, b: b, a: a)
    }
}

/// A color as a native color well, a text field that takes any CSS color, and an eyedropper.
struct ColorField: View {
    @Binding var color: RgbaColor
    var supportsOpacity = true
    /// Called after the eyedropper picks a color, e.g. to add it to a history.
    var onSample: (RgbaColor) -> Void = { _ in }

    @ViewState private var text = ""
    @ViewState private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                ColorPicker("Color", selection: well, supportsOpacity: supportsOpacity)
                    .labelsHidden()
                TextField("Color", text: $text, prompt: Text("#3b82f6, rgb(…), hsl(…), tomato"))
                    .labelsHidden()
                    .font(.body.monospaced())
                    .multilineTextAlignment(.leading)
                    .textFieldStyle(.roundedBorder)
                    .onChange(of: text) { parseText() }
                EyedropperButton { picked in
                    set(picked)
                    onSample(color)
                }
            }
            if let error {
                Text(error)
                    .font(.callout)
                    .foregroundStyle(.red)
            }
        }
        .onChange(of: color, initial: true) { syncText() }
    }

    private var well: Binding<Color> {
        Binding {
            color.swiftUI
        } set: { newValue in
            if let picked = RgbaColor(NSColor(newValue)) { set(picked) }
        }
    }

    private func set(_ newColor: RgbaColor) {
        color = supportsOpacity ? newColor : newColor.withAlpha(1)
    }

    private func parseText() {
        do {
            let parsed = try colorParse(input: text)
            error = nil
            set(parsed)
        } catch {
            self.error = error.message
        }
    }

    /// Shows the hex code of a color set from elsewhere, but leaves the text alone while it
    /// already describes this color (e.g. "tomato" or "hsl(…)" as typed).
    private func syncText() {
        let typed = try? colorParse(input: text)
        if typed.map({ supportsOpacity ? $0 : $0.withAlpha(1) }) != color {
            text = color.hex
            error = nil
        }
    }
}

/// The system eyedropper: a magnifying loupe that samples any pixel on screen. It needs no
/// screen-recording permission.
struct EyedropperButton: View {
    let onPick: (RgbaColor) -> Void

    var body: some View {
        Button("Pick from Screen", systemImage: "eyedropper") {
            NSColorSampler().show { picked in
                // AppKit calls this on the main thread; nil means the user cancelled.
                MainActor.assumeIsolated {
                    if let picked, let color = RgbaColor(picked) {
                        onPick(color)
                    }
                }
            }
        }
        .labelStyle(.iconOnly)
        .help("Pick a color from anywhere on screen")
    }
}

/// A color swatch, with a checkerboard behind translucent colors.
struct Swatch: View {
    let color: RgbaColor
    var cornerRadius: CGFloat = 6

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
        ZStack {
            if color.a < 1 { Checkerboard() }
            color.swiftUI
        }
        .clipShape(shape)
        .overlay(shape.strokeBorder(.separator))
    }
}

private struct Checkerboard: View {
    var body: some View {
        Canvas { context, size in
            let square: CGFloat = 6
            context.fill(Path(CGRect(origin: .zero, size: size)), with: .color(.white))
            for row in 0..<Int((size.height / square).rounded(.up)) {
                for column in 0..<Int((size.width / square).rounded(.up)) where (row + column).isMultiple(of: 2) {
                    let rect = CGRect(x: CGFloat(column) * square, y: CGFloat(row) * square, width: square, height: square)
                    context.fill(Path(rect), with: .color(Color(white: 0.8)))
                }
            }
        }
    }
}

/// A swatch with an optional caption and its hex code below. Clicking copies the hex code and
/// briefly shows "Copied!" in its place.
struct CopySwatch: View {
    let color: RgbaColor
    var caption = ""
    let height: CGFloat
    @ViewState private var copied: String?

    var body: some View {
        let hex = color.hex
        Button {
            Pasteboard.copy(hex)
            copied = hex
        } label: {
            VStack(alignment: .leading, spacing: 3) {
                Swatch(color: color, cornerRadius: 4)
                    .frame(height: height)
                if !caption.isEmpty {
                    Text(caption)
                        .font(.caption.weight(.semibold))
                }
                Text(copied == hex ? "Copied!" : hex)
                    .font(.caption.monospaced())
                    .foregroundStyle(copied == hex ? AnyShapeStyle(.green) : AnyShapeStyle(.secondary))
            }
            .lineLimit(1)
            .minimumScaleFactor(0.7)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .help("Click to copy \(hex)")
        .task(id: copied) {
            guard copied != nil else { return }
            try? await Task.sleep(for: .seconds(1.5))
            copied = nil
        }
    }
}
