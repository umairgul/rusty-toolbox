import SwiftUI
import ToolboxBindings

@Observable
final class CronModel {
    static let nextRuns: UInt32 = 10
    static let examples = cronExamples()

    var pattern = "0 9 * * 1-5"
    var zone = timeZoneSystem() ?? "UTC"

    /// The explanation as of now. Cheap, so it's recomputed whenever the view refreshes.
    var explanation: Result<CronExplanation, any Error> {
        Result { try cronExplain(pattern: pattern, zone: zone, count: Self.nextRuns) }
    }
}

struct CronView: View {
    @Bindable var model: CronModel

    var body: some View {
        // Refreshes every second, so relative times stay current and past runs drop off.
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            Form {
                Section {
                    LabeledContent("Expression") {
                        HStack(spacing: 8) {
                            TextField("Expression", text: $model.pattern, prompt: Text("*/15 * * * *"))
                                .labelsHidden()
                                .font(.body.monospaced())
                                .multilineTextAlignment(.leading)
                                .textFieldStyle(.roundedBorder)
                            examplesMenu
                        }
                    }
                } footer: {
                    FormNote("Fields: minute, hour, day of month, month and day of week, with an optional leading seconds field. Nicknames like @daily work too.")
                }

                switch model.explanation {
                case .success(let explanation):
                    Section {
                        Text(explanation.description)
                            .font(.title3.weight(.semibold))
                            .textSelection(.enabled)
                            .padding(.vertical, 2)
                    }
                    Section("Next \(CronModel.nextRuns) Runs") {
                        LabeledContent("Time Zone") {
                            ZonePickerButton(zone: $model.zone)
                                .help("Time zone for the run times")
                        }
                        if explanation.nextRuns.isEmpty {
                            Text("This expression never runs again.")
                                .foregroundStyle(.secondary)
                        }
                        ForEach(Array(explanation.nextRuns.enumerated()), id: \.offset) { _, run in
                            LabeledContent {
                                Text(run.relative)
                                    .foregroundStyle(.secondary)
                                    .monospacedDigit()
                            } label: {
                                Text(run.time)
                                    .font(.body.monospaced())
                                    .textSelection(.enabled)
                            }
                        }
                    }
                case .failure(let error):
                    Section {
                        Label(error.message, systemImage: "exclamationmark.triangle.fill")
                            .foregroundStyle(.red)
                            .textSelection(.enabled)
                    }
                }
            }
            .formStyle(.grouped)
        }
    }

    private var examplesMenu: some View {
        Menu {
            ForEach(CronModel.examples, id: \.pattern) { example in
                Button("\(example.label)    \(example.pattern)") { model.pattern = example.pattern }
            }
        } label: {
            Label("Examples", systemImage: "list.bullet")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .help("Insert an example expression")
    }
}


