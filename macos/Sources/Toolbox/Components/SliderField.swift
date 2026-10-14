import SwiftUI

/// A whole-number setting as a slider plus a number field, clamped to `range`. With
/// `logarithmic`, the slider spends as much travel on 1–10 as on 100–1000.
struct SliderField: View {
    let title: String
    @Binding var value: UInt32
    let range: ClosedRange<UInt32>
    var logarithmic = false

    init(_ title: String, value: Binding<UInt32>, in range: ClosedRange<UInt32>, logarithmic: Bool = false) {
        self.title = title
        _value = value
        self.range = range
        self.logarithmic = logarithmic
    }

    var body: some View {
        LabeledContent(title) {
            HStack {
                Slider(value: position, in: 0...1)
                TextField(title, value: clamped, format: .number.grouping(.never))
                    .labelsHidden()
                    .textFieldStyle(.roundedBorder)
                    .monospacedDigit()
                    .multilineTextAlignment(.trailing)
                    .frame(width: 56)
            }
        }
    }

    private var clamped: Binding<UInt32> {
        Binding { value } set: { value = min(max($0, range.lowerBound), range.upperBound) }
    }

    /// The slider's position, 0–1.
    private var position: Binding<Double> {
        let low = Double(max(range.lowerBound, logarithmic ? 1 : 0))
        let high = Double(range.upperBound)
        return Binding {
            let value = Double(value)
            if logarithmic { return log(value / low) / log(high / low) }
            return (value - low) / (high - low)
        } set: { position in
            let mapped = logarithmic ? low * pow(high / low, position) : low + position * (high - low)
            clamped.wrappedValue = UInt32(mapped.rounded())
        }
    }
}
