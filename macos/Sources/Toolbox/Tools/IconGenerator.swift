import AppKit
import SwiftUI
import ToolboxBindings

@Observable
final class IconGeneratorModel {
    static let recommendedSize = Int(iconRecommendedSize())
    /// e.g. "1024×1024", without digit grouping.
    static let recommendedDimensions = "\(recommendedSize)×\(recommendedSize)"

    struct Source {
        let url: URL
        /// For display only; icons are made from `image`, decoded by the Rust core.
        let preview: NSImage?
        let image: IconSource
        let width: Int
        let height: Int

        var isSquare: Bool { width == height }
        var isSmall: Bool { min(width, height) < IconGeneratorModel.recommendedSize }
    }

    struct Generated {
        let folder: URL
        var files: [String]
        var cancelled = false

        var hasFavicons: Bool { files.contains("favicon.ico") }
    }

    private(set) var url: URL?
    private(set) var source: Result<Source, any Error>?
    private(set) var isLoading = false
    var fit = IconFit.pad
    var sets = Set(IconSetKind.allCases)
    var appName = "My App"
    private(set) var progress: (done: Int, total: Int)?
    private(set) var result: Result<Generated, any Error>?
    private var generation: Task<Void, Never>?

    var isGenerating: Bool { progress != nil }

    var canGenerate: Bool {
        if case .success = source { !sets.isEmpty && !isGenerating } else { false }
    }

    func open(_ url: URL) {
        guard !isGenerating else { return }
        self.url = url
        result = nil
        isLoading = true
        Task {
            let loaded = await offMainThread { Result { try IconSource.open(path: url.path) } }
            guard self.url == url, let loaded else { return }
            source = loaded.map { image in
                Source(
                    url: url,
                    preview: NSImage(contentsOf: url),
                    image: image,
                    width: Int(image.width()),
                    height: Int(image.height())
                )
            }
            isLoading = false
        }
    }

    func toggle(_ set: IconSetKind, on: Bool) {
        if on { sets.insert(set) } else { sets.remove(set) }
    }

    /// Asks for a folder, then writes the chosen sets there one at a time.
    func chooseFolderAndGenerate() {
        guard canGenerate, let folder = ImageFiles.chooseFolder(prompt: "Generate Icons") else { return }
        generate(into: folder)
    }

    func generate(into folder: URL) {
        guard canGenerate, case .success(let source) = source else { return }
        let image = source.image
        let fit = fit
        let name = appName
        let chosen = IconSetKind.allCases.filter(sets.contains)
        result = nil
        progress = (0, chosen.count)
        generation = Task {
            defer {
                progress = nil
                generation = nil
            }
            var generated = Generated(folder: folder, files: [])
            for set in chosen {
                guard !Task.isCancelled else { break }
                let written = await offMainThread {
                    Result { try image.generate(folder: folder.path, fit: fit, sets: [set], appName: name) }
                }
                switch written {
                case .success(let files):
                    generated.files += files
                case .failure(let error):
                    result = .failure(error)
                    return
                case nil:
                    break
                }
                progress?.done += 1
            }
            generated.cancelled = Task.isCancelled
            result = .success(generated)
        }
    }

    func cancel() {
        generation?.cancel()
    }
}

extension IconGeneratorModel: OpensFiles {
    func open(_ urls: [URL]) {
        if let url = urls.first(where: ImageFiles.accepts) { open(url) }
    }
}

extension IconFit: CaseIterable {
    public static let allCases: [IconFit] = [.pad, .crop]
}

extension IconSetKind: CaseIterable {
    public static let allCases: [IconSetKind] = [.favicons, .macOs, .windows, .pngSizes]
}

struct IconGeneratorView: View {
    @Bindable var model: IconGeneratorModel
    @ViewState private var importing = false

    var body: some View {
        Group {
            switch model.source {
            case nil where model.isLoading:
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            case nil:
                ImageDropZone(
                    title: "Drop an Image Here",
                    message: "Choose a square image of at least \(IconGeneratorModel.recommendedDimensions), or an SVG, for the sharpest icons."
                ) { importing = true }
            case .failure(let error):
                ContentUnavailableView {
                    Label("Couldn't Open This Image", systemImage: "exclamationmark.triangle.fill")
                } description: {
                    Text(error.message)
                } actions: {
                    Button("Choose Another Image…") { importing = true }
                }
            case .success(let source):
                IconGeneratorForm(model: model, source: source)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard !model.isGenerating, let url = urls.first(where: ImageFiles.accepts) else { return false }
            model.open(url)
            return true
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: ImageFiles.contentTypes) { result in
            if let url = try? result.get() { model.open(url) }
        }
        .toolbar {
            ToolbarItemGroup {
                Button("Open Image", systemImage: "photo.badge.plus") { importing = true }
                    .help("Choose the image to make icons from")
                    .disabled(model.isGenerating)
                Button("Generate", systemImage: "wand.and.stars") { model.chooseFolderAndGenerate() }
                    .keyboardShortcut("r")
                    .help("Choose a folder and generate the icons there (⌘R)")
                    .disabled(!model.canGenerate)
                if case .success(let generated) = model.result, !generated.files.isEmpty {
                    Button("Show in Finder", systemImage: "folder") { generated.reveal() }
                        .help("Show the generated icons in Finder")
                }
            }
        }
    }
}

private extension IconGeneratorModel.Generated {
    func reveal() {
        ImageFiles.reveal(files.map { folder.appending(path: $0) })
    }
}

private struct IconGeneratorForm: View {
    @Bindable var model: IconGeneratorModel
    let source: IconGeneratorModel.Source

    var body: some View {
        Form {
            Section {
                sourceRow
                if !source.isSquare {
                    Picker("Not Square", selection: $model.fit) {
                        ForEach(IconFit.allCases, id: \.self) { Text(iconFitLabel(fit: $0)) }
                    }
                    .pickerStyle(.radioGroup)
                }
            }

            Section("Make") {
                ForEach(IconSetKind.allCases, id: \.self) { set in
                    let info = iconSetInfo(set: set)
                    Toggle(isOn: Binding { model.sets.contains(set) } set: { model.toggle(set, on: $0) }) {
                        Text(info.label)
                        Text(info.description)
                    }
                }
            }
            .disabled(model.isGenerating)

            if model.sets.contains(.favicons) {
                Section {
                    TextField("App Name", text: $model.appName, prompt: Text("App"))
                        .disabled(model.isGenerating)
                } footer: {
                    FormNote("Used for the name in site.webmanifest.")
                }
            }

            Section {
                generateRow
                if let result = model.result { resultRows(result) }
            }

            if case .success(let generated) = model.result, generated.hasFavicons {
                Section {
                    CodeEditor(readOnly: iconFaviconHtml(), wrapsLines: false)
                        .frame(height: 96)
                } header: {
                    HStack {
                        Text("Add to Your Page's <head>")
                        Spacer()
                        CopyButton(text: iconFaviconHtml())
                            .buttonStyle(.borderless)
                            .font(.callout)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    private var sourceRow: some View {
        HStack(alignment: .top, spacing: 16) {
            IconPreview(image: source.preview, fit: source.isSquare ? .pad : model.fit)
            VStack(alignment: .leading, spacing: 4) {
                Text(source.url.lastPathComponent)
                    .font(.title3.weight(.semibold))
                    .help(ImageFiles.abbreviated(source.url))
                Text("\(String(source.width)) × \(String(source.height))")
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                if source.isSmall {
                    Label("Smaller than \(IconGeneratorModel.recommendedDimensions): the largest icons will be upscaled and look soft.", systemImage: "exclamationmark.triangle.fill")
                        .font(.callout)
                        .foregroundStyle(.orange)
                        .padding(.top, 4)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder private var generateRow: some View {
        if let progress = model.progress {
            HStack {
                ProgressView(value: Double(progress.done), total: Double(max(progress.total, 1)))
                    .frame(width: 160)
                Text("Generating \(min(progress.done + 1, progress.total)) of \(progress.total) sets…")
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                Spacer()
                Button("Cancel") { model.cancel() }
            }
        } else {
            HStack {
                Button("Generate…") { model.chooseFolderAndGenerate() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!model.canGenerate)
                if model.sets.isEmpty {
                    Text("Choose at least one set to make.")
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    @ViewBuilder private func resultRows(_ result: Result<IconGeneratorModel.Generated, any Error>) -> some View {
        switch result {
        case .success(let generated):
            HStack {
                let count = generated.files.count == 1 ? "1 file" : "\(generated.files.count) files"
                StatusLabel(
                    ok: !generated.cancelled,
                    message: "\(generated.cancelled ? "Cancelled after saving" : "Saved") \(count) to \(ImageFiles.abbreviated(generated.folder))"
                )
                .help(generated.folder.path)
                Spacer()
                if !generated.files.isEmpty {
                    Button("Show in Finder") { generated.reveal() }
                        .controlSize(.small)
                }
            }
            if !generated.files.isEmpty {
                Text(generated.files.joined(separator: ", "))
                    .font(.callout.monospaced())
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        case .failure(let error):
            Label(error.message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
        }
    }
}

/// The source as a square icon, padded or cropped as it will be generated.
private struct IconPreview: View {
    let image: NSImage?
    let fit: IconFit

    var body: some View {
        ZStack {
            if let image {
                Image(nsImage: image)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: fit == .pad ? .fit : .fill)
                    .frame(width: 112, height: 112)
                    .clipped()
            } else {
                Image(systemName: "photo").font(.largeTitle).foregroundStyle(.tertiary)
            }
        }
        .frame(width: 112, height: 112)
        .padding(8)
        .background(.quaternary.opacity(0.6), in: .rect(cornerRadius: 8))
        .help(fit == .pad ? "Padded with transparency to a square" : "Cropped to a centered square")
    }
}
