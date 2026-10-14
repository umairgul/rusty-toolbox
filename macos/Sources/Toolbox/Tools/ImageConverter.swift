import SwiftUI
import ToolboxBindings

@Observable
final class ImageConverterModel {
    enum ResizeMode: CaseIterable {
        case original, fit, scale, exact

        var label: String {
            switch self {
            case .original: "Keep Original Size"
            case .fit: "Fit Within"
            case .scale: "Scale by Percentage"
            case .exact: "Exact Size (May Stretch)"
            }
        }
    }

    enum Status {
        case pending
        case converting
        case converted(ConvertedImage)
        case failed(String)
    }

    struct Item: Identifiable {
        var id: URL { url }
        let url: URL
        let probe: Result<ImageProbe, any Error>
        var status = Status.pending

        var dimensions: String {
            switch probe {
            case .success(let probe):
                if let width = probe.width, let height = probe.height { "\(width) × \(height)" }
                else if probe.mime == "image/heic" { "HEIC" }
                else if probe.mime == "image/svg+xml" { "SVG (vector)" }
                else { "Unknown size" }
            case .failure: "—"
            }
        }

        var size: String {
            (try? probe.get()).map { Int($0.fileSize).byteCount } ?? "—"
        }
    }

    var items: [Item] = []
    var format = ImageOutputFormat.jpeg
    var quality = 85.0
    var background = RgbaColor.white
    var resizeMode = ResizeMode.original
    var width = 1920
    var height = 1080
    var percent = 50
    var allowUpscale = false
    /// nil saves next to each original.
    var outputFolder: URL?
    var showsOptions = true

    private(set) var progress: (done: Int, total: Int)?
    private var conversion: Task<Void, Never>?

    var isConverting: Bool { progress != nil }

    func add(_ urls: [URL]) {
        for url in urls where ImageFiles.accepts(url) && !items.contains(where: { $0.url == url }) {
            items.append(Item(url: url, probe: Result { try imageProbe(path: url.path) }))
        }
    }

    func remove(_ ids: Set<Item.ID>) {
        items.removeAll { ids.contains($0.id) }
    }

    func clear() {
        items.removeAll()
    }

    private var options: ImageConvertOptions {
        let resize: ImageResize = switch resizeMode {
        case .original: .original
        case .fit: .fit(width: UInt32(clamping: width), height: UInt32(clamping: height))
        case .scale: .scale(percent: UInt32(clamping: percent))
        case .exact: .exact(width: UInt32(clamping: width), height: UInt32(clamping: height))
        }
        let byte = { (value: Double) in UInt8((value * 255).rounded()) }
        return ImageConvertOptions(
            format: format,
            quality: UInt8(quality.rounded()),
            resize: resize,
            allowUpscale: allowUpscale,
            background: Data([byte(background.r), byte(background.g), byte(background.b)])
        )
    }

    func convert() {
        guard !isConverting else { return }
        let options = options
        let folder = outputFolder
        progress = (0, items.count)
        for index in items.indices { items[index].status = .pending }
        conversion = Task {
            for index in items.indices {
                guard !Task.isCancelled else { break }
                let input = items[index].url
                let outputFolder = (folder ?? input.deletingLastPathComponent()).path
                items[index].status = .converting
                let result = await offMainThread {
                    Result { try imageConvertFile(input: input.path, folder: outputFolder, options: options) }
                }
                switch result {
                case .success(let converted): items[index].status = .converted(converted)
                case .failure(let error): items[index].status = .failed(error.message)
                case nil: items[index].status = .pending
                }
                progress?.done += 1
            }
            progress = nil
            conversion = nil
        }
    }

    func cancel() {
        conversion?.cancel()
    }

    var convertedURLs: [URL] {
        items.compactMap {
            if case .converted(let converted) = $0.status { URL(filePath: converted.outputPath) } else { nil }
        }
    }

    var summary: String {
        let converted = convertedURLs.count
        let failed = items.filter { if case .failed = $0.status { true } else { false } }.count
        let count = items.count == 1 ? "1 image" : "\(items.count) images"
        if converted + failed == 0 { return count }
        return failed == 0 ? "Converted \(converted) of \(count)" : "Converted \(converted) of \(count), \(failed) failed"
    }
}

extension ImageConverterModel: OpensFiles {
    func open(_ urls: [URL]) {
        add(urls)
    }
}

extension ImageOutputFormat: CaseIterable {
    public static let allCases: [ImageOutputFormat] = [.png, .jpeg, .webp, .avif, .gif, .bmp, .tiff, .ico]
}

struct ImageConverterView: View {
    @Bindable var model: ImageConverterModel
    @ViewState private var importing = false
    @ViewState private var selection = Set<ImageConverterModel.Item.ID>()

    var body: some View {
        Group {
            if model.items.isEmpty {
                ImageDropZone { importing = true }
            } else {
                VStack(spacing: 0) {
                    table
                    StatusBar { status }
                }
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            model.add(urls)
            return true
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: ImageFiles.contentTypes, allowsMultipleSelection: true) { result in
            model.add((try? result.get()) ?? [])
        }
        .inspector(isPresented: $model.showsOptions) {
            ImageConverterOptions(model: model)
                .inspectorColumnWidth(min: 260, ideal: 300, max: 400)
                .disabled(model.isConverting)
        }
        .toolbar {
            ToolbarItemGroup {
                Button("Add Images", systemImage: "plus") { importing = true }
                    .help("Add images to convert")
                    .disabled(model.isConverting)
                Button("Convert", systemImage: "arrow.triangle.2.circlepath") { model.convert() }
                    .help("Convert every image in the list")
                    .disabled(model.items.isEmpty || model.isConverting)
                Button("Options", systemImage: "sidebar.trailing") { model.showsOptions.toggle() }
                    .help(model.showsOptions ? "Hide options" : "Show options")
            }
        }
    }

    private var table: some View {
        Table(model.items, selection: $selection) {
            TableColumn("Name") { item in
                Label {
                    Text(item.url.lastPathComponent).truncationMode(.middle)
                } icon: {
                    Image(nsImage: ImageFiles.icon(for: item.url))
                }
                .help(ImageFiles.abbreviated(item.url))
            }
            .width(min: 140, ideal: 200)
            TableColumn("Dimensions") { item in
                Text(item.dimensions).foregroundStyle(.secondary).monospacedDigit()
            }
            .width(min: 70, ideal: 90)
            TableColumn("Size") { item in
                Text(item.size).foregroundStyle(.secondary).monospacedDigit()
            }
            .width(min: 50, ideal: 70)
            TableColumn("Result") { item in
                ResultCell(item: item)
            }
            .width(min: 120, ideal: 200)
        }
        .contextMenu(forSelectionType: ImageConverterModel.Item.ID.self) { ids in
            Button("Show Original in Finder") { ImageFiles.reveal(Array(ids)) }
            let outputs = model.items.filter { ids.contains($0.id) }.compactMap(\.outputURL)
            if !outputs.isEmpty {
                Button("Show Converted in Finder") { ImageFiles.reveal(outputs) }
            }
            Divider()
            Button("Remove from List") { model.remove(ids) }
                .disabled(model.isConverting)
        } primaryAction: { ids in
            let items = model.items.filter { ids.contains($0.id) }
            ImageFiles.reveal(items.map { $0.outputURL ?? $0.url })
        }
        .onDeleteCommand {
            if !model.isConverting { model.remove(selection) }
        }
    }

    @ViewBuilder private var status: some View {
        if let progress = model.progress {
            ProgressView(value: Double(progress.done), total: Double(max(progress.total, 1)))
                .frame(width: 160)
            Text("Converting \(progress.done + 1) of \(progress.total)…")
                .foregroundStyle(.secondary)
                .monospacedDigit()
            Spacer()
            Button("Cancel") { model.cancel() }
                .controlSize(.small)
        } else {
            Text(model.summary).foregroundStyle(.secondary)
            Spacer()
            if !model.convertedURLs.isEmpty {
                Button("Show in Finder") { ImageFiles.reveal(model.convertedURLs) }
                    .controlSize(.small)
            }
            Button("Clear List") { model.clear() }
                .controlSize(.small)
        }
    }
}

private extension ImageConverterModel.Item {
    var outputURL: URL? {
        if case .converted(let converted) = status { URL(filePath: converted.outputPath) } else { nil }
    }
}

private struct ResultCell: View {
    let item: ImageConverterModel.Item

    var body: some View {
        switch (item.probe, item.status) {
        case (.failure(let error), _):
            Label(error.message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
                .help(error.message)
        case (_, .pending):
            Text("")
        case (_, .converting):
            ProgressView().controlSize(.small)
        case (_, .converted(let converted)):
            let change = (Double(converted.outputSize) / Double(max(converted.inputSize, 1)) - 1) * 100
            HStack(spacing: 4) {
                Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                Text(URL(filePath: converted.outputPath).lastPathComponent)
                    .truncationMode(.middle)
                Text("\(Int(converted.outputSize).byteCount) (\(change > 0 ? "+" : "")\(change, specifier: "%.0f")%)")
                    .foregroundStyle(change > 0 ? .orange : .secondary)
                    .monospacedDigit()
            }
            .help("\(Int(converted.inputSize).byteCount) → \(Int(converted.outputSize).byteCount)")
        case (_, .failed(let message)):
            Label(message, systemImage: "xmark.octagon.fill")
                .foregroundStyle(.red)
                .help(message)
        }
    }
}

private struct ImageConverterOptions: View {
    @Bindable var model: ImageConverterModel

    var body: some View {
        let info = imageFormatInfo(format: model.format)
        Form {
            Section {
                Picker("Format", selection: $model.format) {
                    ForEach(ImageOutputFormat.allCases, id: \.self) { format in
                        Text(imageFormatInfo(format: format).label)
                    }
                }
                if info.hasQuality {
                    LabeledContent("Quality") {
                        HStack {
                            Slider(value: $model.quality, in: 1...100)
                            Text("\(Int(model.quality.rounded()))")
                                .monospacedDigit()
                                .frame(width: 28, alignment: .trailing)
                        }
                    }
                }
                if !info.hasAlpha {
                    LabeledContent("Background") {
                        ColorPicker("Background", selection: backgroundColor, supportsOpacity: false)
                            .labelsHidden()
                    }
                }
            } header: {
                Text("Output")
            } footer: {
                if let note = info.note { FormNote(note) }
            }

            Section("Size") {
                Picker("Resize", selection: $model.resizeMode) {
                    ForEach(ImageConverterModel.ResizeMode.allCases, id: \.self) { Text($0.label) }
                }
                switch model.resizeMode {
                case .original:
                    EmptyView()
                case .fit, .exact:
                    TextField("Width", value: $model.width, format: .number)
                    TextField("Height", value: $model.height, format: .number)
                case .scale:
                    TextField("Percent", value: $model.percent, format: .number)
                }
                if model.resizeMode == .fit || model.resizeMode == .scale {
                    Toggle("Allow making images larger", isOn: $model.allowUpscale)
                }
            }

            Section {
                LabeledContent("Save To") {
                    Text(model.outputFolder.map(ImageFiles.abbreviated) ?? "Next to originals")
                        .truncationMode(.head)
                        .lineLimit(1)
                }
                HStack {
                    Button("Choose Folder…") {
                        if let folder = ImageFiles.chooseFolder() { model.outputFolder = folder }
                    }
                    if model.outputFolder != nil {
                        Button("Use Original Folders") { model.outputFolder = nil }
                    }
                }
            } footer: {
                FormNote("Converting removes EXIF and other metadata, and keeps the color profile. Originals are never overwritten.")
            }
        }
        .formStyle(.grouped)
    }

    private var backgroundColor: Binding<Color> {
        Binding {
            model.background.swiftUI
        } set: { newValue in
            if let color = RgbaColor(NSColor(newValue)) { model.background = color }
        }
    }
}
