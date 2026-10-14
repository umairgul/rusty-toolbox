import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class QrModel {
    enum Mode: Hashable { case text, wifi }

    /// Pixels per module for saved and copied images.
    nonisolated static let exportScale: UInt32 = 16

    var mode = Mode.text
    var text = "https://example.com"
    var ssid = ""
    var password = ""
    var security = QrWifiSecurity.wpa
    var hidden = false
    var errorCorrection = QrErrorCorrection.medium

    /// The text encoded in the QR code. Empty in Wi-Fi mode until there's a network name.
    var payload: String {
        switch mode {
        case .text: text
        case .wifi where ssid.isEmpty: ""
        case .wifi: qrWifiPayload(ssid: ssid, password: password, security: security, hidden: hidden)
        }
    }
}

extension QrErrorCorrection: CaseIterable {
    public static let allCases: [QrErrorCorrection] = [.low, .medium, .quartile, .high]
}

extension QrWifiSecurity: CaseIterable {
    public static let allCases: [QrWifiSecurity] = [.wpa, .wep, .open]
}

struct QrView: View {
    @Bindable var model: QrModel
    @ViewState private var saveError: String?

    var body: some View {
        let payload = model.payload
        let code = Result { try qrEncode(data: payload, errorCorrection: model.errorCorrection) }
        Form {
            Section {
                input
                Picker("Error Correction", selection: $model.errorCorrection) {
                    ForEach(QrErrorCorrection.allCases, id: \.self) { Text(qrErrorCorrectionLabel(level: $0)) }
                }
            } footer: {
                if model.mode == .wifi {
                    FormNote("Phones join the network when they scan this code.")
                }
            }

            Section {
                preview(code, payload: payload)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
            }
        }
        .formStyle(.grouped)
        .toolbar {
            ToolbarItemGroup {
                Picker("Content", selection: $model.mode) {
                    Text("Text / URL").tag(QrModel.Mode.text)
                    Text("Wi-Fi").tag(QrModel.Mode.wifi)
                }
                .pickerStyle(.segmented)
                .help("Encode text or a link, or Wi-Fi network details")

                CopyImageButton(code: try? code.get())

                Menu("Save", systemImage: "square.and.arrow.down") {
                    Button("Save PNG…") { save(code, as: .png) }
                    Button("Save SVG…") { save(code, as: .svg) }
                }
                .disabled((try? code.get()) == nil)
                .help("Save the QR code as a PNG or SVG image")
            }
        }
        .alert("Couldn't Save", isPresented: Binding { saveError != nil } set: { if !$0 { saveError = nil } }) {
        } message: {
            Text(saveError ?? "")
        }
    }

    @ViewBuilder private var input: some View {
        switch model.mode {
        case .text:
            CodeEditor(text: $model.text, placeholder: "Text or URL")
                .frame(height: 76)
        case .wifi:
            TextField("Network Name", text: $model.ssid, prompt: Text("SSID"))
            Picker("Security", selection: $model.security) {
                ForEach(QrWifiSecurity.allCases, id: \.self) { Text(qrWifiSecurityLabel(security: $0)) }
            }
            if model.security != .open {
                SecureField("Password", text: $model.password)
            }
            Toggle("Hidden network", isOn: $model.hidden)
        }
    }

    @ViewBuilder private func preview(_ code: Result<QrCode, any Error>, payload: String) -> some View {
        switch code {
        case .success(let code):
            VStack(spacing: 8) {
                if let image = code.cgImage(scale: 1) {
                    Image(decorative: image, scale: 1)
                        .resizable()
                        .interpolation(.none)
                        .frame(width: 260, height: 260)
                        .clipShape(.rect(cornerRadius: 4))
                        .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(.separator))
                        .contextMenu {
                            Button("Copy Image") { code.copyImage() }
                            Button("Save PNG…") { save(.success(code), as: .png) }
                            Button("Save SVG…") { save(.success(code), as: .svg) }
                        }
                }
                Text("\(code.modules()) × \(code.modules()) modules")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
        case .failure where payload.isEmpty && model.mode == .wifi:
            ContentUnavailableView(
                "No Network Name",
                systemImage: "wifi",
                description: Text("Enter the network name (SSID) to make a Wi-Fi QR code.")
            )
        case .failure(let error) where payload.isEmpty:
            ContentUnavailableView(error.message, systemImage: "qrcode")
        case .failure(let error):
            Label(error.message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
                .textSelection(.enabled)
        }
    }

    private func save(_ code: Result<QrCode, any Error>, as type: UTType) {
        guard let code = try? code.get() else { return }
        let data: Data? = type == .svg ? Data(code.svg().utf8) : code.png()
        guard let data else {
            saveError = "Couldn't create the PNG."
            return
        }
        do {
            try Files.save(data, suggestedName: "qr-code.\(type == .svg ? "svg" : "png")", allowedTypes: [type])
        } catch {
            saveError = error.message
        }
    }
}

/// Copies the QR code as an image, then briefly shows a checkmark.
private struct CopyImageButton: View {
    let code: QrCode?
    @ViewState private var copied = false

    var body: some View {
        Button {
            code?.copyImage()
            copied = true
        } label: {
            Label("Copy Image", systemImage: copied ? "checkmark" : "doc.on.doc")
                .contentTransition(.symbolEffect(.replace))
        }
        .disabled(code == nil)
        .help("Copy the QR code to the clipboard as an image")
        .task(id: copied) {
            guard copied else { return }
            try? await Task.sleep(for: .seconds(1.5))
            copied = false
        }
    }
}

private extension QrCode {
    /// One pixel per module at scale 1, including the quiet zone. Scale it up with
    /// `.interpolation(.none)` so the edges stay crisp.
    func cgImage(scale: UInt32) -> CGImage? {
        let size = Int(pixelSize(scale: scale))
        guard size > 0, let provider = CGDataProvider(data: pixels(scale: scale) as CFData) else { return nil }
        return CGImage(
            width: size,
            height: size,
            bitsPerComponent: 8,
            bitsPerPixel: 8,
            bytesPerRow: size,
            space: CGColorSpaceCreateDeviceGray(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
            provider: provider,
            decode: nil,
            shouldInterpolate: false,
            intent: .defaultIntent
        )
    }

    /// A PNG at the export scale.
    func png() -> Data? {
        cgImage(scale: QrModel.exportScale).flatMap {
            NSBitmapImageRep(cgImage: $0).representation(using: .png, properties: [:])
        }
    }

    /// Puts the code on the clipboard as PNG and TIFF, so any app can paste it.
    func copyImage() {
        guard let image = cgImage(scale: QrModel.exportScale) else { return }
        let rep = NSBitmapImageRep(cgImage: image)
        let item = NSPasteboardItem()
        if let png = rep.representation(using: .png, properties: [:]) { item.setData(png, forType: .png) }
        if let tiff = rep.tiffRepresentation { item.setData(tiff, forType: .tiff) }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([item])
    }
}
