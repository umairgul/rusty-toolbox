import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class HashesModel {
    enum Mode: Hashable { case text, file }

    enum FileStatus {
        case hashing(HashFileJob)
        case hashed(HashDigests)
        case failed(String)
    }

    struct HashedFile {
        let url: URL
        let size: Int
        var status: FileStatus
    }

    var mode = Mode.text
    var text = ""
    var uppercase = false
    var expected = ""
    private(set) var textDigests = hashesText(text: "")
    private(set) var file: HashedFile?

    /// The digests for the current mode, or nil while a file is hashing or none is chosen.
    var digests: HashDigests? {
        switch mode {
        case .text: return textDigests
        case .file:
            if case .hashed(let digests) = file?.status { return digests }
            return nil
        }
    }

    fileprivate func hashText(_ text: String) async {
        if let digests = await offMainThread({ hashesText(text: text) }) {
            textDigests = digests
        }
    }

    /// Hashes `url` on a background thread, replacing (and cancelling) any file before it.
    func hashFile(_ url: URL) {
        cancelFile()
        mode = .file
        let job = HashFileJob()
        let size = (try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0
        file = HashedFile(url: url, size: size, status: .hashing(job))
        let path = url.path
        Task {
            let result = await Task.detached(priority: .userInitiated) {
                Result { try job.run(path: path) }
            }.value
            // A newer file or Cancel replaced this job meanwhile.
            guard case .hashing(let current) = file?.status, current === job else { return }
            switch result {
            case .success(let digests?): file?.status = .hashed(digests)
            case .success(nil): file = nil
            case .failure(let error): file?.status = .failed(error.message)
            }
        }
    }

    func cancelFile() {
        if case .hashing(let job) = file?.status {
            job.cancel()
            file = nil
        }
    }
}

extension HashesModel: OpensFiles {
    func open(_ urls: [URL]) {
        if let url = urls.first { hashFile(url) }
    }
}

struct HashesView: View {
    @Bindable var model: HashesModel
    @ViewState private var dropTargeted = false

    var body: some View {
        Form {
            switch model.mode {
            case .text:
                Section("Text") {
                    CodeEditor(text: $model.text, placeholder: "Text to hash (UTF-8)")
                        .frame(height: 96)
                }
            case .file:
                Section("File") {
                    FileRow(model: model, choose: chooseFile)
                }
            }

            if let digests = model.digests {
                Section {
                    ForEach(digests.entries(), id: \.algorithm) { entry in
                        DigestRow(
                            label: entry.insecure ? "\(entry.label) (legacy)" : entry.label,
                            digest: model.uppercase ? entry.digest.uppercased() : entry.digest
                        )
                    }
                } header: {
                    Text("Hashes")
                } footer: {
                    FormNote("MD5 and SHA-1 are fine for checksums but broken for security uses.")
                }

                Section("Verify a Checksum") {
                    TextField("Expected Hash", text: $model.expected, prompt: Text("Paste the expected hash"))
                        .labelsHidden()
                        .multilineTextAlignment(.leading)
                        .font(.body.monospaced())
                    if !model.expected.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        if let algorithm = digests.findMatch(expected: model.expected) {
                            StatusLabel(ok: true, message: "Matches the \(hashAlgorithmLabel(algorithm: algorithm)) hash")
                        } else {
                            StatusLabel(ok: false, message: "Doesn't match any hash above")
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
        .task(id: model.text) {
            await model.hashText(model.text)
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let url = urls.first(where: \.isFileURL) else { return false }
            model.hashFile(url)
            return true
        } isTargeted: {
            dropTargeted = $0
        }
        .overlay {
            if dropTargeted {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(Color.accentColor, lineWidth: 3)
                    .padding(4)
                    .allowsHitTesting(false)
            }
        }
        .toolbar {
            ToolbarItemGroup {
                Picker("Input", selection: $model.mode) {
                    Text("Text").tag(HashesModel.Mode.text)
                    Text("File").tag(HashesModel.Mode.file)
                }
                .pickerStyle(.segmented)
                .help("Hash text you type, or a file")

                Button("Choose File", systemImage: "doc.badge.plus", action: chooseFile)
                    .help("Choose a file to hash, or drop one onto the window")

                Toggle("Uppercase", systemImage: "characters.uppercase", isOn: $model.uppercase)
                    .help("Show hashes in uppercase")
            }
        }
    }

    private func chooseFile() {
        if let url = Files.open(allowedTypes: [.item]).first {
            model.hashFile(url)
        }
    }
}

/// A digest with its algorithm and a copy button. Digests longer than 64 characters (SHA-384,
/// SHA-512) are shown on two lines split at 64, so they never wrap mid-word with a hyphen; the
/// copy button copies the digest without the line break.
private struct DigestRow: View {
    static let lineLength = 64

    let label: String
    let digest: String

    var body: some View {
        LabeledContent {
            HStack(alignment: .firstTextBaseline) {
                Text(lines)
                    .font(.body.monospaced())
                    .multilineTextAlignment(.leading)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
                CopyButton(text: digest)
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
            }
        } label: {
            Text(label)
        }
    }

    private var lines: String {
        guard digest.count > Self.lineLength else { return digest }
        let split = digest.index(digest.startIndex, offsetBy: Self.lineLength)
        return digest[..<split] + "\n" + digest[split...]
    }
}

/// The chosen file: a drop target while there's none, progress while hashing, then its name.
private struct FileRow: View {
    let model: HashesModel
    let choose: () -> Void

    var body: some View {
        if let file = model.file {
            HStack(spacing: 10) {
                Image(nsImage: NSWorkspace.shared.icon(forFile: file.url.path))
                    .resizable()
                    .frame(width: 32, height: 32)
                VStack(alignment: .leading, spacing: 4) {
                    Text(file.url.lastPathComponent)
                        .fontWeight(.semibold)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .help(ImageFiles.abbreviated(file.url))
                    details(file)
                }
                Spacer(minLength: 8)
                actions(file)
            }
            .padding(.vertical, 2)
        } else {
            VStack(spacing: 8) {
                Image(systemName: "doc.badge.arrow.up")
                    .font(.largeTitle)
                    .foregroundStyle(.secondary)
                Text("Drop a File Here")
                    .font(.headline)
                Text("Any file, of any size. It's read in chunks and never leaves this computer.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("Choose File…", action: choose)
                    .padding(.top, 4)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
            .background {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(.tertiary, style: StrokeStyle(lineWidth: 1.5, dash: [6, 4]))
            }
            .padding(.vertical, 4)
        }
    }

    @ViewBuilder private func details(_ file: HashesModel.HashedFile) -> some View {
        switch file.status {
        case .hashing(let job):
            TimelineView(.periodic(from: .now, by: 0.1)) { _ in
                let done = Int(job.bytesRead())
                HStack(spacing: 8) {
                    ProgressView(value: Double(min(done, file.size)), total: Double(max(file.size, 1)))
                        .frame(maxWidth: 240)
                    Text("\(done.byteCount) of \(file.size.byteCount)")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
            }
        case .hashed:
            Text(file.size.byteCount)
                .font(.callout)
                .foregroundStyle(.secondary)
                .monospacedDigit()
        case .failed(let message):
            Label(message, systemImage: "exclamationmark.triangle.fill")
                .font(.callout)
                .foregroundStyle(.red)
                .textSelection(.enabled)
        }
    }

    @ViewBuilder private func actions(_ file: HashesModel.HashedFile) -> some View {
        if case .hashing = file.status {
            Button("Cancel") { model.cancelFile() }
        } else {
            Button("Show in Finder", systemImage: "folder") { ImageFiles.reveal([file.url]) }
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
                .help("Show the file in Finder")
            Button("Choose File…", action: choose)
        }
    }
}
