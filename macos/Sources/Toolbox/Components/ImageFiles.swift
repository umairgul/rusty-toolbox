import AppKit
import SwiftUI
import ToolboxBindings
import UniformTypeIdentifiers

/// Opening, dropping and revealing image files, shared by the image tools.
enum ImageFiles {
    /// Extensions the Rust core can read.
    static let extensions = Set(imageInputExtensions())

    /// For open panels and drops.
    static let contentTypes: [UTType] = {
        var types: [UTType] = []
        for type in extensions.sorted().compactMap({ UTType(filenameExtension: $0) }) where !types.contains(type) {
            types.append(type)
        }
        return types
    }()

    static func accepts(_ url: URL) -> Bool {
        extensions.contains(url.pathExtension.lowercased())
    }

    static func reveal(_ urls: [URL]) {
        NSWorkspace.shared.activateFileViewerSelecting(urls)
    }

    static func icon(for url: URL) -> NSImage {
        NSWorkspace.shared.icon(forFile: url.path)
    }

    /// Asks for a folder, e.g. where to save results.
    static func chooseFolder(prompt: String = "Choose") -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.prompt = prompt
        return panel.runModal() == .OK ? panel.url : nil
    }

    /// "~/Pictures/Export" rather than "/Users/name/Pictures/Export".
    static func abbreviated(_ url: URL) -> String {
        (url.path as NSString).abbreviatingWithTildeInPath
    }
}

/// The empty state of an image tool: a dashed drop target with a button to open files.
struct ImageDropZone: View {
    var title = "Drop Images Here"
    var message = "PNG, JPEG, WebP, GIF, HEIC, SVG and more"
    let open: () -> Void

    var body: some View {
        ContentUnavailableView {
            Label(title, systemImage: "photo.on.rectangle.angled")
        } description: {
            Text(message)
        } actions: {
            Button("Choose Images…", action: open)
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .strokeBorder(.tertiary, style: StrokeStyle(lineWidth: 2, dash: [8, 6]))
                .padding(16)
        }
    }
}
