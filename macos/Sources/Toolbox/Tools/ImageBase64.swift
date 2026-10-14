import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class ImageBase64Model {
    /// Above this, a data URI is usually worse than a separate file.
    static let largeImage = 32 * 1024
    /// Characters of a data URI shown before cutting it short.
    static let shownCharacters = 96

    enum Mode: Hashable { case toBase64, toImage }

    struct Encoded {
        let url: URL
        /// For display only; the data URI comes from the Rust core.
        let preview: NSImage?
        let result: ImageBase64Encoded
    }

    struct Decoded {
        let result: ImageBase64Decoded
        /// For display only.
        let preview: NSImage?
    }

    var mode = Mode.toBase64

    private(set) var encodedURL: URL?
    private(set) var encoded: Result<Encoded, any Error>?
    private(set) var isEncoding = false

    var input = ""
    /// nil while the input is empty.
    private(set) var decoded: Result<Decoded, any Error>?
    /// Where the decoded image was last saved, or why saving failed.
    private(set) var saved: Result<URL, any Error>?

    func encode(_ url: URL) {
        mode = .toBase64
        encodedURL = url
        isEncoding = true
        Task {
            let result = await offMainThread { Result { try imageBase64EncodeFile(path: url.path) } }
            guard encodedURL == url, let result else { return }
            encoded = result.map { Encoded(url: url, preview: NSImage(contentsOf: url), result: $0) }
            isEncoding = false
        }
    }

    fileprivate func decode(_ input: String) async {
        saved = nil
        guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            decoded = nil
            return
        }
        if let result = await offMainThread({ Result { try imageBase64Decode(text: input) } }) {
            decoded = result.map { Decoded(result: $0, preview: NSImage(data: $0.bytes)) }
        }
    }

    func saveImage() {
        guard case .success(let decoded) = decoded else { return }
        let ext = decoded.result.extension
        do {
            if let url = try Files.save(
                decoded.result.bytes,
                suggestedName: "image.\(ext)",
                allowedTypes: [UTType(filenameExtension: ext)].compactMap { $0 }
            ) {
                saved = .success(url)
            }
        } catch {
            saved = .failure(error)
        }
    }
}

extension ImageBase64Model: OpensFiles {
    func open(_ urls: [URL]) {
        if let url = urls.first(where: ImageFiles.accepts) { encode(url) }
    }
}

extension ImageBase64Model {
    /// The start of a long value, with its length, so a huge data URI doesn't flood the view.
    static func shortened(_ text: String) -> String {
        guard text.count > shownCharacters else { return text }
        return "\(text.prefix(shownCharacters))… (\(text.count.formatted()) characters)"
    }
}

struct ImageBase64View: View {
    @Bindable var model: ImageBase64Model
    @ViewState private var importing = false

    var body: some View {
        Group {
            switch model.mode {
            case .toBase64: encoder
            case .toImage: ImageBase64Decoder(model: model)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let url = urls.first(where: ImageFiles.accepts) else { return false }
            model.encode(url)
            return true
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: ImageFiles.contentTypes) { result in
            if let url = try? result.get() { model.encode(url) }
        }
        .task(id: model.input) {
            await model.decode(model.input)
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Mode", selection: $model.mode) {
                    Text("Image to Base64").tag(ImageBase64Model.Mode.toBase64)
                    Text("Base64 to Image").tag(ImageBase64Model.Mode.toImage)
                }
                .pickerStyle(.segmented)
                .help("Turn an image into a data URI, or a data URI back into an image")

                switch model.mode {
                case .toBase64:
                    Button("Open Image", systemImage: "photo.badge.plus") { importing = true }
                        .help("Choose an image to turn into a data URI")
                case .toImage:
                    Button("Save Image", systemImage: "square.and.arrow.down") { model.saveImage() }
                        .help("Save the decoded image to a file")
                        .disabled(!model.hasDecodedImage)
                }
            }
        }
    }

    @ViewBuilder private var encoder: some View {
        switch model.encoded {
        case nil where model.isEncoding:
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        case nil:
            ImageDropZone(title: "Drop an Image Here", message: "Choose an image to get it as a data URI for HTML, CSS or Markdown.") {
                importing = true
            }
        case .failure(let error):
            ContentUnavailableView {
                Label("Couldn't Read This Image", systemImage: "exclamationmark.triangle.fill")
            } description: {
                Text(error.message)
            } actions: {
                Button("Choose Another Image…") { importing = true }
            }
        case .success(let encoded):
            ImageBase64Encoder(encoded: encoded)
        }
    }
}

private extension ImageBase64Model {
    var hasDecodedImage: Bool {
        if case .success = decoded { true } else { false }
    }
}

private struct ImageBase64Encoder: View {
    let encoded: ImageBase64Model.Encoded

    var body: some View {
        let result = encoded.result
        Form {
            Section {
                HStack(alignment: .top, spacing: 16) {
                    if let preview = encoded.preview {
                        Image(nsImage: preview)
                            .resizable()
                            .interpolation(.high)
                            .aspectRatio(contentMode: .fit)
                            .frame(maxWidth: 96, maxHeight: 96)
                    }
                    VStack(alignment: .leading, spacing: 4) {
                        Text(encoded.url.lastPathComponent)
                            .font(.title3.weight(.semibold))
                            .help(ImageFiles.abbreviated(encoded.url))
                        Text("\(Int(result.fileSize).byteCount) file, \(result.uri.utf8.count.byteCount) as Base64")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    Spacer(minLength: 0)
                }
                .padding(.vertical, 4)
            } footer: {
                if result.fileSize > ImageBase64Model.largeImage {
                    FormNote("Data URIs are a third larger than the file and can't be cached separately; they suit small icons best.")
                }
            }

            Section {
                SnippetRow(label: "Data URI", value: result.uri)
                ForEach(result.snippets, id: \.label) { snippet in
                    SnippetRow(label: snippet.label, value: snippet.value)
                }
            } footer: {
                FormNote("Long values are shortened here; Copy copies them in full.")
            }
        }
        .formStyle(.grouped)
    }
}

private struct ImageBase64Decoder: View {
    @Bindable var model: ImageBase64Model

    var body: some View {
        VStack(spacing: 0) {
            HSplitView {
                Pane("Data URI") {
                    CodeEditor(
                        text: $model.input,
                        placeholder: "Paste a data URI (data:image/png;base64,…), CSS url(…), <img src=…> or plain Base64"
                    )
                } accessories: {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        model.input = Pasteboard.string ?? model.input
                    }
                    .help("Replace the input with the clipboard")
                    Button("Clear", systemImage: "xmark.circle") { model.input = "" }
                        .help("Clear the input")
                        .disabled(model.input.isEmpty)
                }
                Pane("Image") {
                    output
                } accessories: {
                    if model.hasDecodedImage {
                        Button("Save Image", systemImage: "square.and.arrow.down") { model.saveImage() }
                            .help("Save the decoded image to a file")
                    }
                }
            }
            StatusBar { status }
        }
    }

    @ViewBuilder private var output: some View {
        switch model.decoded {
        case nil:
            ContentUnavailableView("No Image", systemImage: "photo", description: Text("Paste a data URI or Base64 to see the image."))
        case .failure(let error):
            ContentUnavailableView {
                Label("Not an Image", systemImage: "exclamationmark.triangle.fill")
            } description: {
                Text(error.message)
            }
        case .success(let decoded):
            if let preview = decoded.preview {
                Image(nsImage: preview)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: preview.size.width, maxHeight: preview.size.height)
                    .padding()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ContentUnavailableView(
                    "No Preview",
                    systemImage: "photo",
                    description: Text("This \(decoded.result.mime) image can't be shown here, but you can still save it.")
                )
            }
        }
    }

    @ViewBuilder private var status: some View {
        switch model.decoded {
        case .success(let decoded):
            let result = decoded.result
            let size = if let width = result.width, let height = result.height { " · \(width)×\(height)" } else { "" }
            StatusLabel(ok: true, message: "\(result.mime)\(size) · \(result.bytes.count.byteCount)")
                .monospacedDigit()
            Spacer()
            switch model.saved {
            case .success(let url):
                Text("Saved to \(ImageFiles.abbreviated(url))")
                    .foregroundStyle(.secondary)
                    .truncationMode(.head)
                    .lineLimit(1)
                Button("Show in Finder") { ImageFiles.reveal([url]) }
                    .controlSize(.small)
            case .failure(let error):
                Label("Couldn't save: \(error.message)", systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                    .lineLimit(1)
            case nil:
                EmptyView()
            }
        case .failure(let error):
            StatusLabel(ok: false, message: error.message)
        case nil:
            Text("Paste a data URI or Base64").foregroundStyle(.secondary)
        }
    }
}

/// A label with a copy button, and the start of a long value under it, so data URIs read
/// left to right instead of wrapping against the trailing edge.
private struct SnippetRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                Spacer()
                CopyButton(text: value)
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
            }
            Text(ImageBase64Model.shortened(value))
                .font(.callout.monospaced())
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 2)
    }
}
