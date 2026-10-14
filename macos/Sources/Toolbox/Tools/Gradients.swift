import SwiftUI
import ToolboxBindings

@Observable
final class GradientsModel {
    static let stopLimits = 2...6
    static let stepLimits = 2...16

    struct Stop: Identifiable {
        let id = UUID()
        var color: RgbaColor
    }

    var stops = [
        Stop(color: RgbaColor(r: 0xff / 255, g: 0x5f / 255, b: 0x6d / 255, a: 1)),
        Stop(color: RgbaColor(r: 0x36 / 255, g: 0x5b / 255, b: 0xff / 255, a: 1)),
    ]
    var space = GradientSpace.oklab
    /// Degrees, as in CSS: 0 points up, 90 to the right.
    var angle = 90.0
    var steps = 7

    var colors: [RgbaColor] { stops.map(\.color) }
    var canAddStop: Bool { stops.count < Self.stopLimits.upperBound }
    var canRemoveStop: Bool { stops.count > Self.stopLimits.lowerBound }

    func addStop() {
        guard canAddStop else { return }
        stops.append(Stop(color: stops.last?.color ?? .white))
    }

    func removeStop(_ id: Stop.ID) {
        guard canRemoveStop else { return }
        stops.removeAll { $0.id == id }
    }
}

extension GradientSpace: CaseIterable {
    public static let allCases: [GradientSpace] = [.oklab, .oklch, .srgb, .linearSrgb]
}

struct GradientsView: View {
    /// Colors sampled for the preview; the view blends between them, so close samples keep it
    /// true to the chosen color space.
    private static let previewSamples: UInt32 = 96

    @Bindable var model: GradientsModel

    var body: some View {
        let colors = model.colors
        let css = gradientsCss(stops: colors, angle: model.angle, space: model.space)
        Form {
            Section("Stops") {
                ForEach($model.stops) { $stop in
                    LabeledContent("Stop \((model.stops.firstIndex { $0.id == stop.id } ?? 0) + 1)") {
                        HStack(alignment: .top, spacing: 8) {
                            ColorField(color: $stop.color)
                            if model.canRemoveStop {
                                Button("Remove Stop", systemImage: "minus.circle") {
                                    model.removeStop(stop.id)
                                }
                                .labelStyle(.iconOnly)
                                .buttonStyle(.borderless)
                                .padding(.top, 3)
                                .help("Remove this stop")
                            }
                        }
                    }
                }
                if model.canAddStop {
                    Button("Add Stop", systemImage: "plus") { model.addStop() }
                        .help("Add a stop at the end, starting from the last color")
                }
            }

            Section {
                Picker("Blend In", selection: $model.space) {
                    ForEach(GradientSpace.allCases, id: \.self) { Text(gradientsSpaceLabel(space: $0)) }
                }
                LabeledContent("Angle") {
                    HStack {
                        Slider(value: angle, in: 0...360)
                        Text("\(Int(model.angle.rounded()))°")
                            .monospacedDigit()
                            .frame(width: 40, alignment: .trailing)
                    }
                }
                GradientPreview(
                    colors: gradientsSteps(stops: colors, count: Self.previewSamples, space: model.space),
                    angle: model.angle
                )
                .frame(height: 120)
                .padding(.vertical, 4)
            } header: {
                Text("Gradient")
            } footer: {
                FormNote("The preview is blended in the chosen color space and follows the angle, as the CSS does.")
            }

            Section("CSS") {
                ValueRow("CSS (modern browsers)", value: css.modern)
                ValueRow("CSS (every browser)", value: css.fallback)
            }

            Section {
                Stepper(value: $model.steps, in: GradientsModel.stepLimits) {
                    LabeledContent("Steps") {
                        Text("\(model.steps)").monospacedDigit()
                    }
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 64, maximum: 96), spacing: 8)], alignment: .leading, spacing: 10) {
                    let steps = gradientsSteps(stops: colors, count: UInt32(model.steps), space: model.space)
                    ForEach(Array(steps.enumerated()), id: \.offset) { index, color in
                        CopySwatch(color: color, caption: "\(index + 1)", height: 40)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Steps")
            } footer: {
                FormNote("Evenly spaced colors from the first stop to the last. Click a swatch to copy it.")
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItem {
                Button("Add Stop", systemImage: "plus") { model.addStop() }
                    .disabled(!model.canAddStop)
                    .help("Add a color stop")
            }
        }
    }

    /// Whole degrees, without the tick marks a stepped slider draws.
    private var angle: Binding<Double> {
        Binding { model.angle } set: { model.angle = $0.rounded() }
    }
}

/// The gradient drawn from colors sampled by the core, at a CSS angle. Translucent colors show
/// a checkerboard behind them.
private struct GradientPreview: View {
    let colors: [RgbaColor]
    let angle: Double

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 8, style: .continuous)
        let gradient = Gradient(stops: colors.enumerated().map { index, color in
            Gradient.Stop(color: color.swiftUI, location: Double(index) / Double(max(colors.count - 1, 1)))
        })
        ZStack {
            if colors.contains(where: { $0.a < 1 }) {
                Swatch(color: RgbaColor.white.withAlpha(0), cornerRadius: 8)
            }
            Canvas { context, size in
                let (start, end) = Self.line(angle: angle, size: size)
                context.fill(
                    Path(CGRect(origin: .zero, size: size)),
                    with: .linearGradient(gradient, startPoint: start, endPoint: end)
                )
            }
        }
        .clipShape(shape)
        .overlay(shape.strokeBorder(.separator))
    }

    /// The gradient line CSS uses for `angle`: through the center, long enough that the
    /// corners get the first and last colors.
    static func line(angle: Double, size: CGSize) -> (CGPoint, CGPoint) {
        let radians = angle * .pi / 180
        let (dx, dy) = (sin(radians), -cos(radians))
        let half = (abs(size.width * dx) + abs(size.height * dy)) / 2
        let center = CGPoint(x: size.width / 2, y: size.height / 2)
        return (
            CGPoint(x: center.x - dx * half, y: center.y - dy * half),
            CGPoint(x: center.x + dx * half, y: center.y + dy * half)
        )
    }
}

