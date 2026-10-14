import SwiftUI
import ToolboxBindings

@Observable
final class PasswordModel {
    enum Mode: Hashable { case password, passphrase }

    var mode = Mode.password
    var passwordOptions = passwordDefaultOptions() {
        didSet { if passwordOptions != oldValue { generatePassword() } }
    }
    var passphraseOptions = passphraseDefaultOptions() {
        didSet { if passphraseOptions != oldValue { generatePassphrase() } }
    }
    /// Fails when no character set is selected.
    private(set) var password: Result<String, any Error> = .success("")
    private(set) var passphrase = ""

    init() {
        generatePassword()
        generatePassphrase()
    }

    func regenerate() {
        switch mode {
        case .password: generatePassword()
        case .passphrase: generatePassphrase()
        }
    }

    /// The password or passphrase on screen, or "" when the options are invalid.
    var secret: String {
        switch mode {
        case .password: (try? password.get()) ?? ""
        case .passphrase: passphrase
        }
    }

    var strength: PasswordStrengthInfo {
        switch mode {
        case .password: passwordStrength(options: passwordOptions)
        case .passphrase: passphraseStrength(options: passphraseOptions)
        }
    }

    private func generatePassword() {
        password = Result { try passwordGenerate(options: passwordOptions) }
    }

    private func generatePassphrase() {
        passphrase = passphraseGenerate(options: passphraseOptions)
    }
}

extension PassphraseSeparator: CaseIterable {
    public static let allCases: [PassphraseSeparator] = [.hyphen, .space, .period, .underscore, .noSeparator]
}

struct PasswordView: View {
    @Bindable var model: PasswordModel

    var body: some View {
        Form {
            Section {
                result
                StrengthRow(strength: model.strength)
            }
            switch model.mode {
            case .password: passwordOptions
            case .passphrase: passphraseOptions
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItemGroup {
                Picker("Mode", selection: $model.mode) {
                    Text("Password").tag(PasswordModel.Mode.password)
                    Text("Passphrase").tag(PasswordModel.Mode.passphrase)
                }
                .pickerStyle(.segmented)
                .help("Random characters, or random words that are easier to remember and type")

                Button("Regenerate", systemImage: "arrow.clockwise") { model.regenerate() }
                    .keyboardShortcut("r")
                    .help(model.mode == .password ? "Generate a new password" : "Generate a new passphrase")

                CopyButton(text: model.secret)
            }
        }
    }

    @ViewBuilder private var result: some View {
        switch (model.mode, model.password) {
        case (.password, .failure(let error)):
            Label(error.message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
                .padding(.vertical, 6)
        default:
            HStack(alignment: .firstTextBaseline) {
                Text(model.secret)
                    .font(.system(.title2, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .fixedSize(horizontal: false, vertical: true)
                CopyButton(text: model.secret)
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
            }
            .padding(.vertical, 6)
        }
    }

    private var passwordOptions: some View {
        Section("Options") {
            SliderField("Length", value: $model.passwordOptions.length, in: passwordMinLength()...passwordMaxLength())
            Toggle("Lowercase (a–z)", isOn: $model.passwordOptions.lowercase)
            Toggle("Uppercase (A–Z)", isOn: $model.passwordOptions.uppercase)
            Toggle("Digits (0–9)", isOn: $model.passwordOptions.digits)
            Toggle("Symbols (!@#$…)", isOn: $model.passwordOptions.symbols)
            Toggle("Exclude look-alikes (0 O 1 l I |)", isOn: $model.passwordOptions.excludeAmbiguous)
        }
    }

    private var passphraseOptions: some View {
        Section {
            SliderField("Words", value: $model.passphraseOptions.words, in: passphraseMinWords()...passphraseMaxWords())
            Picker("Separator", selection: $model.passphraseOptions.separator) {
                ForEach(PassphraseSeparator.allCases, id: \.self) { separator in
                    Text(passphraseSeparatorLabel(separator: separator))
                }
            }
            Toggle("Capitalize words", isOn: $model.passphraseOptions.capitalize)
            Toggle("Add a number", isOn: $model.passphraseOptions.includeNumber)
        } header: {
            Text("Options")
        } footer: {
            FormNote("Words from the EFF long word list (\(passphraseWordListSize().formatted()) words).")
        }
    }
}

/// The strength rating and bits of entropy over a meter that's full at 128 bits, tinted by the
/// rating.
private struct StrengthRow: View {
    let strength: PasswordStrengthInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            LabeledContent("Strength") {
                Text("\(strength.label) · \(Int(strength.bits.rounded())) bits of entropy")
                    .monospacedDigit()
            }
            ProgressView(value: min(strength.bits / 128, 1))
                .tint(strength.strength.color)
                .accessibilityLabel("Strength")
        }
        .padding(.vertical, 2)
    }
}

private extension PasswordStrength {
    var color: Color {
        switch self {
        case .weak: .red
        case .fair: .orange
        case .strong, .veryStrong: .green
        }
    }
}


