import SwiftUI

struct ContentView: View {
    @AppStorage("selectedTool") private var selectedTool: Tool = .json
    @ViewState private var search = ""
    @EnvironmentObject private var appDelegate: AppDelegate
    @Environment(ToolStore.self) private var store

    var body: some View {
        NavigationSplitView {
            Sidebar(selection: selection, search: $search)
                .navigationSplitViewColumnWidth(min: 200, ideal: 220, max: 300)
        } detail: {
            ToolDetail(tool: selectedTool)
                .id(selectedTool)
                .navigationTitle(selectedTool.name)
        }
        .onChange(of: appDelegate.openedFiles, initial: true) { openFiles() }
    }

    /// Hands files opened from Finder to the current tool, or to the Image Converter if the
    /// current tool doesn't take files.
    private func openFiles() {
        let urls = appDelegate.openedFiles
        guard !urls.isEmpty else { return }
        appDelegate.openedFiles = []
        if let tool = store.model(for: selectedTool) as? OpensFiles {
            tool.open(urls)
        } else {
            selectedTool = .imageConverter
            store.imageConverter.open(urls)
        }
    }

    /// The sidebar list needs an optional selection; clicking empty space keeps the current tool.
    private var selection: Binding<Tool?> {
        Binding { selectedTool } set: { if let tool = $0 { selectedTool = tool } }
    }
}

private struct Sidebar: View {
    @Binding var selection: Tool?
    @Binding var search: String

    var body: some View {
        ScrollViewReader { proxy in
            List(selection: $selection) {
                ForEach(ToolCategory.allCases) { category in
                    let tools = Tool.tools(in: category).filter(matches)
                    if !tools.isEmpty {
                        Section(category.title) {
                            ForEach(tools) { tool in
                                Label(tool.name, systemImage: tool.symbol)
                                    .help(tool.summary)
                            }
                        }
                    }
                }
            }
            .searchable(text: $search, placement: .sidebar, prompt: "Search Tools")
            .onAppear {
                // The last-used tool may be below the fold.
                if let selection { proxy.scrollTo(selection) }
            }
        }
    }

    private func matches(_ tool: Tool) -> Bool {
        search.isEmpty
            || tool.name.localizedStandardContains(search)
            || tool.summary.localizedStandardContains(search)
    }
}

private struct ToolDetail: View {
    let tool: Tool
    @Environment(ToolStore.self) private var store

    var body: some View {
        switch tool {
        case .password: PasswordView(model: store.password)
        case .names: NamesView(model: store.names)
        case .ids: IdsView(model: store.ids)
        case .qr: QrView(model: store.qr)
        case .lorem: LoremView(model: store.lorem)
        case .fakeData: FakeDataView(model: store.fakeData)
        case .hashes: HashesView(model: store.hashes)
        case .encode: EncodeView(model: store.encode)
        case .timestamps: TimestampsView(model: store.timestamps)
        case .jwt: JwtView(model: store.jwt)
        case .formats: DataFormatsView(model: store.formats)
        case .numberBase: NumberBaseView(model: store.numberBase)
        case .textCase: TextCaseView(model: store.textCase)
        case .cron: CronView(model: store.cron)
        case .json: JsonFormatterView(model: store.json)
        case .regex: RegexTesterView(model: store.regex)
        case .textDiff: TextDiffView(model: store.textDiff)
        case .sql: SqlFormatterView(model: store.sql)
        case .markdown: MarkdownView(model: store.markdown)
        case .colorPicker: ColorPickerView(model: store.colorPicker)
        case .contrast: ContrastView(model: store.contrast)
        case .palettes: PalettesView(model: store.palettes)
        case .gradients: GradientsView(model: store.gradients)
        case .imageConverter: ImageConverterView(model: store.imageConverter)
        case .imageMetadata: ImageMetadataView(model: store.imageMetadata)
        case .iconGenerator: IconGeneratorView(model: store.iconGenerator)
        case .imageBase64: ImageBase64View(model: store.imageBase64)
        }
    }
}

