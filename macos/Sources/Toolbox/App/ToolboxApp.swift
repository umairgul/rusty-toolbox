import SwiftUI

@main
struct ToolboxApp: App {
    @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate
    @ViewState private var store = ToolStore()

    var body: some Scene {
        Window("Toolbox", id: "main") {
            ContentView()
                .environment(store)
                .frame(minWidth: 780, minHeight: 480)
        }
        .defaultSize(width: 1120, height: 760)
        .commands {
            SidebarCommands()
            ToolbarCommands()
            TextEditingCommands()
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    /// Files opened from Finder ("Open With", or dropped on the Dock icon), waiting for
    /// ContentView to hand them to a tool.
    @Published var openedFiles: [URL] = []

    func applicationDidFinishLaunching(_ notification: Notification) {
        // `-appearance dark` or `-appearance light` on the command line overrides the system
        // appearance, e.g. to check both for screenshots.
        switch UserDefaults.standard.string(forKey: "appearance") {
        case "dark": NSApp.appearance = NSAppearance(named: .darkAqua)
        case "light": NSApp.appearance = NSAppearance(named: .aqua)
        default: break
        }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        openedFiles += urls.filter(\.isFileURL)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
