import SwiftUI
import ToolboxBindings

/// Every IANA time zone, grouped by region, with a search field. Meant for a popover.
struct ZoneList: View {
    /// Zones shown with a checkmark, e.g. the current choice or zones already added.
    var checked: Set<String> = []
    /// Whether checked zones can be picked again.
    var disablesChecked = false
    let pick: (String) -> Void

    @ViewState private var search = ""
    @FocusState private var searchFocused: Bool

    /// "Other" holds zones without a region (UTC, GMT, EST…) and goes last.
    private static let regions: [(region: String, zones: [String])] = {
        var regions: [String: [String]] = [:]
        for zone in timeZoneNames() {
            let region = zone.contains("/") ? String(zone.prefix { $0 != "/" }) : "Other"
            regions[region, default: []].append(zone)
        }
        return regions.keys.sorted { a, b in
            a == "Other" ? false : b == "Other" ? true : a < b
        }.map { ($0, regions[$0]!) }
    }()

    var body: some View {
        VStack(spacing: 0) {
            TextField("Search", text: $search, prompt: Text("Search, e.g. Paris or Asia/"))
                .textFieldStyle(.roundedBorder)
                .focused($searchFocused)
                .padding(10)
            Divider()
            let matches = filtered
            if matches.isEmpty {
                ContentUnavailableView.search(text: search)
            } else {
                ScrollViewReader { proxy in
                    List {
                        ForEach(matches, id: \.region) { group in
                            Section(group.region) {
                                ForEach(group.zones, id: \.self) { zone in
                                    row(zone, region: group.region)
                                }
                            }
                        }
                    }
                    .listStyle(.sidebar)
                    .onAppear {
                        if let first = checked.sorted().first { proxy.scrollTo(first, anchor: .center) }
                    }
                }
            }
        }
        .frame(width: 320, height: 420)
        .onAppear { searchFocused = true }
    }

    private func row(_ zone: String, region: String) -> some View {
        let isChecked = checked.contains(zone)
        // "America/Argentina/Buenos_Aires" shows as "Argentina/Buenos Aires" under America.
        let name = region == "Other" ? zone : String(zone.dropFirst(region.count + 1))
        return Button { pick(zone) } label: {
            HStack {
                Text(name.replacingOccurrences(of: "_", with: " "))
                Spacer()
                if isChecked { Image(systemName: "checkmark").foregroundStyle(.secondary) }
            }
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .disabled(disablesChecked && isChecked)
        .help(zone)
        .id(zone)
    }

    /// Zones whose name contains the search text, ignoring case, with spaces matching
    /// underscores (so "new york" finds America/New_York).
    private var filtered: [(region: String, zones: [String])] {
        let needle = search.trimmingCharacters(in: .whitespaces).lowercased().replacingOccurrences(of: " ", with: "_")
        guard !needle.isEmpty else { return Self.regions }
        return Self.regions.compactMap { group in
            let zones = group.zones.filter { $0.lowercased().contains(needle) }
            return zones.isEmpty ? nil : (group.region, zones)
        }
    }
}

/// A button showing the chosen time zone, which opens a searchable `ZoneList`.
struct ZonePickerButton: View {
    @Binding var zone: String
    @ViewState private var isPresented = false

    var body: some View {
        Button {
            isPresented = true
        } label: {
            HStack(spacing: 4) {
                Text(zone)
                Image(systemName: "chevron.up.chevron.down")
                    .imageScale(.small)
                    .foregroundStyle(.secondary)
            }
        }
        .popover(isPresented: $isPresented, arrowEdge: .bottom) {
            ZoneList(checked: [zone]) { picked in
                zone = picked
                isPresented = false
            }
        }
    }
}
