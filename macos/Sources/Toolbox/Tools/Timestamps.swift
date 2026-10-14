import SwiftUI
import ToolboxBindings

@Observable
final class TimestampsModel {
    var input = String(timestampsNow())
    /// nil guesses the unit of a plain number from its size.
    var unit: TimestampUnit?
    /// The zones listed under Time Zones, in order.
    var zones: [String]
    /// The zone that dates without an offset are read in.
    var inputZone: String

    init() {
        let zones = timestampsDefaultZones()
        self.zones = zones
        inputZone = zones.first ?? "UTC"
    }

    func useNow() {
        input = String(timestampsNow())
    }

    func add(zone: String) {
        if !zones.contains(zone) { zones.append(zone) }
    }

    func remove(zone: String) {
        guard let index = zones.firstIndex(of: zone) else { return }
        zones.remove(at: index)
        if inputZone == zone {
            inputZone = zones.isEmpty ? "UTC" : zones[max(index - 1, 0)]
        }
    }

    /// The choices for `inputZone`: the listed zones, or UTC when there are none.
    var inputZoneChoices: [String] {
        zones.isEmpty ? ["UTC"] : zones
    }

    func convert() -> Result<TimestampConversion, any Error> {
        Result { try timestampsConvert(input: input, unit: unit, inputZone: inputZone, zones: zones) }
    }
}

extension TimestampUnit: CaseIterable {
    public static let allCases: [TimestampUnit] = [.seconds, .milliseconds, .microseconds, .nanoseconds]

    /// e.g. "Seconds".
    var label: String { timestampsUnitLabel(unit: self).capitalizedFirst }
}

private extension String {
    var capitalizedFirst: String { prefix(1).uppercased() + dropFirst() }
}

struct TimestampsView: View {
    @Bindable var model: TimestampsModel
    @ViewState private var addingZone = false

    var body: some View {
        // Relative times and the current Unix time change every second.
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            let conversion = model.convert()
            Form {
                Section {
                    ValueRow("Current Unix time", value: String(timestampsNow()))
                        .monospacedDigit()
                }

                Section {
                    TextField("Timestamp or Date", text: $model.input, prompt: Text("1700000000, 2023-11-14T22:13:20Z, …"))
                        .labelsHidden()
                        .font(.body.monospaced())
                        .textFieldStyle(.roundedBorder)
                    Picker("Numbers are", selection: $model.unit) {
                        Text("Auto-detected").tag(TimestampUnit?.none)
                        Divider()
                        ForEach(TimestampUnit.allCases, id: \.self) { unit in
                            Text(unit.label).tag(Optional(unit))
                        }
                    }
                    Picker("Dates without an offset are in", selection: $model.inputZone) {
                        ForEach(model.inputZoneChoices, id: \.self) { Text($0) }
                    }
                } header: {
                    Text("Input")
                } footer: {
                    switch conversion {
                    case .success(let conversion):
                        FormNote("Read as \(conversion.source).")
                    case .failure(let error):
                        Label(error.message, systemImage: "exclamationmark.triangle.fill")
                            .font(.callout)
                            .foregroundStyle(.red)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }

                if case .success(let conversion) = conversion {
                    Section("Formats") {
                        ForEach(conversion.values, id: \.label) { value in
                            ValueRow(value.label, value: value.value)
                        }
                    }
                }

                Section {
                    let times = (try? conversion.get()).map { Dictionary($0.zones.map { ($0.zone, $0.time) }, uniquingKeysWith: { a, _ in a }) } ?? [:]
                    ForEach(model.zones, id: \.self) { zone in
                        ZoneRow(zone: zone, time: times[zone]) { model.remove(zone: zone) }
                    }
                } header: {
                    HStack {
                        Text("Time Zones")
                        Spacer()
                        Button("Add Time Zone…", systemImage: "plus") { addingZone = true }
                            .buttonStyle(.borderless)
                            .font(.callout)
                            .help("Show this time in another time zone")
                            .popover(isPresented: $addingZone, arrowEdge: .bottom) {
                                ZoneList(checked: Set(model.zones), disablesChecked: true) { zone in
                                    model.add(zone: zone)
                                    addingZone = false
                                }
                            }
                    }
                }
            }
            .formStyle(.grouped)
        }
        .toolbar {
            ToolbarItem {
                Button("Now", systemImage: "clock.arrow.circlepath") { model.useNow() }
                    .help("Convert the current time")
            }
        }
    }
}

/// A zone's name and the time there, with buttons to copy the time or remove the zone.
private struct ZoneRow: View {
    let zone: String
    let time: String?
    let remove: () -> Void

    /// This Mac's zone, read once rather than for every row every second.
    private static let systemZone = timeZoneSystem()

    var body: some View {
        LabeledContent {
            HStack(alignment: .firstTextBaseline) {
                Text(time ?? "—")
                    .font(.body.monospaced())
                    .foregroundStyle(time == nil ? .secondary : .primary)
                    .textSelection(.enabled)
                CopyButton(text: time ?? "")
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
                Button("Remove", systemImage: "minus.circle", action: remove)
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
                    .help("Remove this time zone")
            }
        } label: {
            if zone == Self.systemZone {
                Text("\(zone) \(Text("(this Mac)").foregroundStyle(.secondary))")
            } else {
                Text(zone)
            }
        }
    }
}


