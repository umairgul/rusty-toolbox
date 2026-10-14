import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

@Observable
final class ImageMetadataModel {
    struct Opened {
        let url: URL
        /// For display only; the report comes from the Rust core.
        let preview: NSImage?
        let report: ImageMetadataReport
    }

    private(set) var url: URL?
    private(set) var opened: Result<Opened, any Error>?
    private(set) var saved: Result<ImageMetadataSaved, any Error>?
    private(set) var isLoading = false
    private(set) var isSaving = false
    /// Groups the user has folded away. "Other" starts folded, as it's rarely interesting.
    var collapsedGroups: Set<String> = ["Other"]

    func open(_ url: URL) {
        self.url = url
        saved = nil
        isLoading = true
        Task {
            let result = await offMainThread { Result { try imageMetadataRead(path: url.path) } }
            guard self.url == url, let result else { return }
            opened = result.map { Opened(url: url, preview: NSImage(contentsOf: url), report: $0) }
            isLoading = false
        }
    }

    /// Asks where to save a copy without metadata, next to the original by default.
    func saveCleanCopy() {
        guard case .success(let opened) = opened, !isSaving else { return }
        let input = opened.url
        let panel = NSSavePanel()
        panel.directoryURL = input.deletingLastPathComponent()
        let ext = input.pathExtension.isEmpty ? "jpg" : input.pathExtension
        panel.nameFieldStringValue = "\(input.deletingPathExtension().lastPathComponent)-clean.\(ext)"
        if let type = UTType(filenameExtension: ext) { panel.allowedContentTypes = [type] }
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let output = panel.url else { return }
        isSaving = true
        Task {
            let result = await offMainThread {
                Result { try imageMetadataSaveClean(input: input.path, output: output.path) }
            }
            if let result { saved = result }
            isSaving = false
        }
    }
}

extension ImageMetadataModel: OpensFiles {
    func open(_ urls: [URL]) {
        if let url = urls.first(where: ImageFiles.accepts) { open(url) }
    }
}

struct ImageMetadataView: View {
    @Bindable var model: ImageMetadataModel
    @ViewState private var importing = false

    var body: some View {
        Group {
            switch model.opened {
            case nil where model.isLoading:
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            case nil:
                ImageDropZone(title: "Drop a Photo Here", message: "Open a photo to see its hidden information.") {
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
            case .success(let opened):
                ImageMetadataReportView(model: model, opened: opened)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let url = urls.first(where: ImageFiles.accepts) else { return false }
            model.open(url)
            return true
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: ImageFiles.contentTypes) { result in
            if let url = try? result.get() { model.open(url) }
        }
        .toolbar {
            ToolbarItemGroup {
                Button("Open Image", systemImage: "photo.badge.plus") { importing = true }
                    .help("Open a photo to see its metadata")
                Button("Save Clean Copy", systemImage: "square.and.arrow.down") { model.saveCleanCopy() }
                    .help("Save a copy without metadata. The original is kept untouched.")
                    .disabled(!model.canSaveCleanCopy)
                if let url = model.url {
                    Button("Show in Finder", systemImage: "folder") { ImageFiles.reveal([url]) }
                        .help("Show the original in Finder")
                }
            }
        }
    }
}

private extension ImageMetadataModel {
    var canSaveCleanCopy: Bool {
        guard case .success(let opened) = opened else { return false }
        return opened.report.canClean && !opened.report.isClean && !isSaving
    }
}

private struct ImageMetadataReportView: View {
    @Bindable var model: ImageMetadataModel
    let opened: ImageMetadataModel.Opened

    private var report: ImageMetadataReport { opened.report }

    var body: some View {
        Form {
            Section {
                header
            } footer: {
                if !report.canClean {
                    FormNote("Lossless cleaning works for JPEG, PNG and WebP. To clean this file, convert it with the Image Converter.")
                }
            }

            if let saved = model.saved {
                Section { savedRow(saved) }
            }

            if let location = report.location {
                Section {
                    LocationWarning(location: location)
                }
            }

            Section {
                if report.isClean {
                    StatusLabel(ok: true, message: "No metadata found: this image is clean.")
                } else {
                    Text(summary).font(.headline)
                }
            }

            ForEach(report.groups, id: \.name) { group in
                Section(isExpanded: expanded(group.name)) {
                    ForEach(Array(group.fields.enumerated()), id: \.offset) { _, field in
                        FieldRow(field: field)
                    }
                } header: {
                    Text("\(group.name) (\(group.fields.count))")
                }
            }

            if !report.other.isEmpty {
                Section("Also Found") {
                    ForEach(report.other, id: \.self) { item in
                        Text(item)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    private var header: some View {
        HStack(alignment: .top, spacing: 16) {
            if let preview = opened.preview {
                Image(nsImage: preview)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: 160, maxHeight: 160)
                    .clipShape(.rect(cornerRadius: 6))
            }
            VStack(alignment: .leading, spacing: 4) {
                Text(opened.url.lastPathComponent)
                    .font(.title3.weight(.semibold))
                    .textSelection(.enabled)
                Text(Int(report.fileSize).byteCount)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                Spacer().frame(height: 8)
                HStack {
                    Button("Save Clean Copy…") { model.saveCleanCopy() }
                        .buttonStyle(.borderedProminent)
                        .disabled(!model.canSaveCleanCopy)
                    if model.isSaving { ProgressView().controlSize(.small) }
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder private func savedRow(_ saved: Result<ImageMetadataSaved, any Error>) -> some View {
        switch saved {
        case .success(let saved):
            let url = URL(filePath: saved.outputPath)
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    if saved.verifiedClean {
                        let kept = saved.keptOrientation ? " Kept only the rotation, so it still displays upright." : ""
                        StatusLabel(ok: true, message: "Saved \(url.lastPathComponent). Checked: no metadata left.\(kept)")
                    } else {
                        StatusLabel(ok: false, message: "Saved \(url.lastPathComponent), but some metadata remains.")
                    }
                    Spacer()
                    Button("Show in Finder") { ImageFiles.reveal([url]) }
                        .controlSize(.small)
                }
                if !saved.removed.isEmpty {
                    Text("Removed: \(saved.removed.joined(separator: ", "))")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
            }
        case .failure(let error):
            Label(error.message, systemImage: "exclamationmark.triangle.fill")
                .foregroundStyle(.red)
        }
    }

    private var summary: String {
        let fields = report.fieldCount == 1 ? "1 EXIF field" : "\(report.fieldCount) EXIF fields"
        let other = report.other.count == 1 ? "1 other block" : "\(report.other.count) other blocks"
        return "\(fields), \(report.sensitiveCount) of them personal, and \(other)"
    }

    private func expanded(_ group: String) -> Binding<Bool> {
        Binding {
            !model.collapsedGroups.contains(group)
        } set: { expanded in
            if expanded { model.collapsedGroups.remove(group) } else { model.collapsedGroups.insert(group) }
        }
    }
}

private struct FieldRow: View {
    let field: ImageMetadataField

    var body: some View {
        LabeledContent {
            Text(field.value)
                .font(.body.monospaced())
                .multilineTextAlignment(.trailing)
                .textSelection(.enabled)
        } label: {
            HStack(spacing: 6) {
                Text(field.name)
                if field.sensitive {
                    Text("personal")
                        .font(.caption)
                        .foregroundStyle(.orange)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(.orange.opacity(0.15), in: .capsule)
                        .help("Personal or identifying: location, device, serial numbers, owner or dates")
                }
            }
        }
    }
}

private struct LocationWarning: View {
    let location: ImageMetadataLocation

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "location.fill")
                .font(.title2)
                .foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 6) {
                Text("This image reveals where it was taken")
                    .font(.headline)
                Text(coordinates)
                    .font(.body.monospacedDigit())
                    .textSelection(.enabled)
                HStack(spacing: 16) {
                    if let maps = URL(string: location.appleMapsUrl) {
                        Link(destination: maps) { Label("Open in Maps", systemImage: "map") }
                            .help("Opens the Maps app; the location is sent to Apple")
                    }
                    if let osm = URL(string: location.openStreetMapUrl) {
                        Link("Show on OpenStreetMap", destination: osm)
                            .help("Opens your browser; the location is sent to openstreetmap.org")
                    }
                    CopyButton(title: "Copy Coordinates", text: String(format: "%.5f, %.5f", location.latitude, location.longitude))
                        .buttonStyle(.link)
                }
            }
        }
        .padding(.vertical, 4)
    }

    private var coordinates: String {
        let altitude = location.altitude.map { String(format: ", %.0f m above sea level", $0) } ?? ""
        return String(format: "Latitude %.5f, longitude %.5f", location.latitude, location.longitude) + altitude
    }
}
