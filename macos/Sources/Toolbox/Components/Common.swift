import AppKit
import UniformTypeIdentifiers
import SwiftUI
import ToolboxBindings

/// Copies `text` to the clipboard, then briefly shows a checkmark. Disabled when `text` is empty.
struct CopyButton: View {
    var title: String = "Copy"
    let text: String
    @ViewState private var copied: String?

    var body: some View {
        Button {
            Pasteboard.copy(text)
            copied = text
        } label: {
            // The checkmark only shows while the clipboard still matches what's on screen.
            Label(title, systemImage: copied == text ? "checkmark" : "doc.on.doc")
                .contentTransition(.symbolEffect(.replace))
        }
        .disabled(text.isEmpty)
        .help("Copy to the clipboard")
        .task(id: copied) {
            guard copied != nil else { return }
            try? await Task.sleep(for: .seconds(1.5))
            copied = nil
        }
    }
}

enum Pasteboard {
    static func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    static var string: String? {
        NSPasteboard.general.string(forType: .string)
    }
}

/// A titled pane for an editor or result, with a bar of accessories along the top, like the
/// panes in Xcode's inspectors.
struct Pane<Content: View, Accessories: View>: View {
    let title: String
    @ViewBuilder var content: Content
    @ViewBuilder var accessories: Accessories

    init(
        _ title: String,
        @ViewBuilder content: () -> Content,
        @ViewBuilder accessories: () -> Accessories = { EmptyView() }
    ) {
        self.title = title
        self.content = content()
        self.accessories = accessories()
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                accessories
                    .buttonStyle(.borderless)
                    .labelStyle(.iconOnly)
                    .controlSize(.small)
            }
            .padding(.horizontal, 10)
            .frame(height: 30)
            .background(.bar)
            Divider()
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(minWidth: 220, minHeight: 120)
    }
}

/// A one-line bar along the bottom of a tool, for status and sizes.
struct StatusBar<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(spacing: 12) { content }
                .font(.callout)
                .padding(.horizontal, 12)
                .frame(height: 28)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.bar)
        }
    }
}

/// A green check or red cross with a message, e.g. "Valid JSON" or "Signature doesn't match".
struct StatusLabel: View {
    let ok: Bool
    let message: String

    var body: some View {
        Label {
            Text(message)
        } icon: {
            Image(systemName: ok ? "checkmark.circle.fill" : "xmark.octagon.fill")
                .foregroundStyle(ok ? .green : .red)
        }
    }
}

extension Error {
    /// The message to show the user. Use this rather than `localizedDescription`: UniFFI's
    /// generated `ToolError.errorDescription` is `String(reflecting: self)`, which shows the
    /// enum case instead of the message.
    nonisolated var message: String {
        if case .Failed(let message) = self as? ToolError { message } else { localizedDescription }
    }
}

extension Int {
    /// e.g. "512 bytes" or "1.5 MB", as Finder shows file sizes.
    var byteCount: String {
        ByteCountFormatter.string(fromByteCount: Int64(self), countStyle: .file)
    }
}

/// Runs `work` off the main thread and returns its result, or nil if the calling task was
/// cancelled meanwhile (e.g. the input changed again), so stale results are never shown.
func offMainThread<T: Sendable>(_ work: @escaping @Sendable () -> T) async -> T? {
    let result = await Task.detached(priority: .userInitiated, operation: work).value
    return Task.isCancelled ? nil : result
}

/// A form row with a label and a monospaced, selectable value, plus a button to copy it.
/// `copy` overrides what's copied, e.g. digits shown grouped but copied plain.
struct ValueRow: View {
    let label: String
    let value: String
    var copy: String?

    init(_ label: String, value: String, copy: String? = nil) {
        self.label = label
        self.value = value
        self.copy = copy
    }

    var body: some View {
        LabeledContent(label) {
            HStack(alignment: .firstTextBaseline) {
                Text(value)
                    .font(.body.monospaced())
                    .multilineTextAlignment(.trailing)
                    .textSelection(.enabled)
                CopyButton(text: copy ?? value)
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
            }
        }
    }
}

/// Explanatory text under a form section, aligned like System Settings' footers.
struct FormNote: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.callout)
            .foregroundStyle(.secondary)
            .multilineTextAlignment(.leading)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}

enum Files {
    /// Asks where to save, then writes `data`. Returns where it was saved, or nil if cancelled.
    /// Throws if writing fails.
    @discardableResult
    static func save(_ data: Data, suggestedName: String, allowedTypes: [UTType] = []) throws -> URL? {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = suggestedName
        panel.canCreateDirectories = true
        if !allowedTypes.isEmpty { panel.allowedContentTypes = allowedTypes }
        guard panel.runModal() == .OK, let url = panel.url else { return nil }
        try data.write(to: url, options: .atomic)
        return url
    }

    /// Asks for files to open. Returns an empty array if cancelled.
    static func open(allowedTypes: [UTType], multiple: Bool = false) -> [URL] {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = allowedTypes
        panel.allowsMultipleSelection = multiple
        return panel.runModal() == .OK ? panel.urls : []
    }
}
