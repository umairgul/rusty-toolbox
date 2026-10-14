import SwiftUI
import ToolboxBindings

@Observable
final class JwtModel {
    var token = ""
    var secret = ""
    var secretIsBase64 = false
    var wrapsLines = true

    var isEmpty: Bool { token.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    /// nil while there's no token. Expiry is relative to now, so call it again as time passes.
    func decode() -> Result<JwtDecoded, any Error>? {
        isEmpty ? nil : Result { try jwtDecode(token: token) }
    }

    /// nil until a secret is entered.
    func verify() -> JwtVerification? {
        secret.isEmpty ? nil : try? jwtVerify(token: token, secret: secret, secretIsBase64: secretIsBase64)
    }
}

struct JwtView: View {
    @Bindable var model: JwtModel

    var body: some View {
        // "Expires in 3 minutes" and the claims' relative times change as time passes.
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            let decoded = model.decode()
            VStack(spacing: 0) {
                HSplitView {
                    // A VStack, not a VSplitView: a split view leaves an empty gap where the
                    // details were when the token stops decoding. The token pane keeps its
                    // identity either way, so typing in it is never interrupted.
                    let jwt = try? decoded?.get()
                    VStack(spacing: 0) {
                        Pane("Token") {
                            CodeEditor(text: $model.token, placeholder: "Paste a JWT (eyJ…), with or without \"Bearer \"", wrapsLines: true)
                        } accessories: {
                            Button("Paste", systemImage: "doc.on.clipboard") {
                                model.token = Pasteboard.string ?? model.token
                            }
                            .help("Replace the token with the clipboard")
                            Button("Clear", systemImage: "xmark.circle") { model.token = "" }
                                .help("Clear the token")
                                .disabled(model.token.isEmpty)
                        }
                        .frame(maxHeight: jwt == nil ? .infinity : 180)
                        if let jwt {
                            Divider()
                            JwtDetails(model: model, jwt: jwt)
                        }
                    }
                    .frame(minWidth: 320)
                    decodedParts(decoded)
                        .frame(minWidth: 280, maxWidth: .infinity, maxHeight: .infinity)
                }
                // Without this the split view shrinks to its ideal height when nothing in it
                // is greedy (no token, or one that doesn't decode).
                .frame(maxHeight: .infinity)
                StatusBar { status(decoded) }
            }
        }
        .toolbar {
            ToolbarItem {
                Toggle("Wrap Lines", systemImage: "text.word.spacing", isOn: $model.wrapsLines)
                    .help("Wrap long lines in the header and payload")
            }
        }
    }

    @ViewBuilder private func decodedParts(_ decoded: Result<JwtDecoded, any Error>?) -> some View {
        switch decoded {
        case .success(let jwt):
            VSplitView {
                Pane("Header") {
                    CodeEditor(readOnly: jwt.header, wrapsLines: model.wrapsLines)
                } accessories: {
                    CopyButton(title: "Copy Header", text: jwt.header)
                }
                .frame(minHeight: 100, idealHeight: 160)
                Pane("Payload") {
                    CodeEditor(readOnly: jwt.payload, wrapsLines: model.wrapsLines)
                } accessories: {
                    CopyButton(title: "Copy Payload", text: jwt.payload)
                }
            }
        case .failure(let error):
            ContentUnavailableView {
                Label("Can't Decode Token", systemImage: "exclamationmark.triangle.fill")
                    .symbolRenderingMode(.multicolor)
            } description: {
                Text(error.message)
                    .textSelection(.enabled)
            }
        case nil:
            ContentUnavailableView(
                "No Token",
                systemImage: "checkmark.seal",
                description: Text("Paste a JWT to see its header, payload and expiry. Tokens never leave this computer.")
            )
        }
    }

    @ViewBuilder private func status(_ decoded: Result<JwtDecoded, any Error>?) -> some View {
        switch decoded {
        case .success(let jwt):
            StatusLabel(ok: jwt.status.ok, message: jwt.status.message)
            Spacer()
            Text(jwt.algorithmDescription)
                .foregroundStyle(.secondary)
        case .failure(let error):
            StatusLabel(ok: false, message: error.message)
        case nil:
            EmptyView()
        }
    }
}

/// The time claims and signature verification of a decoded token.
private struct JwtDetails: View {
    @Bindable var model: JwtModel
    let jwt: JwtDecoded

    var body: some View {
        Form {
            if !jwt.claims.isEmpty {
                Section("Claims") {
                    ForEach(jwt.claims, id: \.label) { claim in
                        ValueRow(claim.label, value: claim.shown, copy: claim.copy)
                    }
                }
            }
            Section {
                if jwt.verifiable {
                    SecureField("Secret", text: $model.secret, prompt: Text("Secret"))
                    Toggle("Secret is Base64", isOn: $model.secretIsBase64)
                }
            } header: {
                Text("Verify Signature")
            } footer: {
                if let note = jwt.verifyNote {
                    FormNote(note)
                } else {
                    verification
                }
            }
        }
        .formStyle(.grouped)
    }

    @ViewBuilder private var verification: some View {
        switch model.verify() {
        case .valid(let message):
            StatusLabel(ok: true, message: message)
        case .invalid(let message):
            StatusLabel(ok: false, message: message)
        case .invalidSecret(let message):
            Label(message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
        case .unsupported(let message):
            FormNote(message)
        case nil:
            EmptyView()
        }
    }
}

