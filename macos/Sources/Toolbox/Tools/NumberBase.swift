import SwiftUI
import ToolboxBindings

@Observable
final class NumberBaseModel {
    static let limits = numberBaseLimits()

    /// The bases the input can be read in. nil reads 0x, 0o and 0b prefixes, otherwise decimal.
    static let inputBases: [(base: UInt32?, label: String)] = [
        (nil, "Auto (0x, 0o, 0b or Decimal)"),
        (10, "Decimal"),
        (16, "Hex"),
        (8, "Octal"),
        (2, "Binary"),
    ]

    var input = "255"
    var inputBase: UInt32?
    var customBase: UInt32 = 36
    var uppercase = false

    var outcome: NumberBaseOutcome {
        numberBaseConvert(input: input, inputBase: inputBase, customBase: customBase, uppercase: uppercase)
    }
}

struct NumberBaseView: View {
    @Bindable var model: NumberBaseModel

    var body: some View {
        let limits = NumberBaseModel.limits
        Form {
            Section {
                LabeledContent("Number") {
                    TextField("Number", text: $model.input, prompt: Text("e.g. 255, 0xff, -42"))
                        .labelsHidden()
                        .font(.body.monospaced())
                        .multilineTextAlignment(.leading)
                        .textFieldStyle(.roundedBorder)
                }
                Picker("Input Base", selection: $model.inputBase) {
                    ForEach(NumberBaseModel.inputBases, id: \.base) { option in
                        Text(option.label).tag(option.base)
                    }
                }
            } footer: {
                FormNote("Spaces, underscores and commas are ignored, and a leading minus sign is allowed. Numbers can be up to 128 bits.")
            }

            Section("Output") {
                LabeledContent("Custom Base") {
                    HStack(spacing: 6) {
                        TextField("Custom Base", value: customBase, format: .number)
                            .labelsHidden()
                            .multilineTextAlignment(.trailing)
                            .monospacedDigit()
                            .frame(width: 44)
                        Stepper("Custom Base", value: customBase, in: Int(limits.minBase)...Int(limits.maxBase))
                            .labelsHidden()
                    }
                }
                Toggle("Uppercase Letters", isOn: $model.uppercase)
            }

            switch model.outcome {
            case .ok(let rows):
                Section("Conversions") {
                    ForEach(rows, id: \.label) { row in
                        ValueRow(row.label, value: row.shown, copy: row.copied)
                    }
                }
            case .invalid(let message):
                Section {
                    Label(message, systemImage: "exclamationmark.triangle.fill")
                        .foregroundStyle(.red)
                        .textSelection(.enabled)
                }
            case .empty:
                EmptyView()
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItem {
                Button("Paste", systemImage: "doc.on.clipboard") {
                    if let text = Pasteboard.string { model.input = text.trimmingCharacters(in: .whitespacesAndNewlines) }
                }
                .help("Replace the number with the clipboard")
            }
        }
    }

    /// The custom base as an Int for the stepper and field, kept within the supported range.
    private var customBase: Binding<Int> {
        let limits = NumberBaseModel.limits
        return Binding {
            Int(model.customBase)
        } set: { newValue in
            model.customBase = UInt32(min(max(newValue, Int(limits.minBase)), Int(limits.maxBase)))
        }
    }
}
