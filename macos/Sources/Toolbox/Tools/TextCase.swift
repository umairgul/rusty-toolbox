import SwiftUI
import ToolboxBindings

@Observable
final class TextCaseModel {
    var input = "parseHTTPResponse"
    private(set) var values: [TextCaseValue] = []

    var isEmpty: Bool { input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    fileprivate func convert(_ input: String) async {
        if let values = await offMainThread({ textCaseConvertAll(input: input) }) {
            self.values = values
        }
    }
}

struct TextCaseView: View {
    @Bindable var model: TextCaseModel

    var body: some View {
        VStack(spacing: 0) {
            Pane("Input") {
                CodeEditor(text: $model.input, placeholder: "Text or identifiers, one per line")
            } accessories: {
                Button("Paste", systemImage: "doc.on.clipboard") {
                    model.input = Pasteboard.string ?? model.input
                }
                .help("Replace the input with the clipboard")
                Button("Clear", systemImage: "xmark.circle") { model.input = "" }
                    .help("Clear the input")
                    .disabled(model.input.isEmpty)
            }
            .frame(height: 130)
            Divider()

            Group {
                if model.isEmpty {
                    ContentUnavailableView(
                        "No Text",
                        systemImage: "textformat",
                        description: Text("Type text or identifiers above. Each line is converted separately.")
                    )
                } else {
                    Form {
                        Section {
                            ForEach(model.values, id: \.label) { value in
                                ValueRow(value.label, value: value.value)
                            }
                        } footer: {
                            FormNote("Each line is converted separately. Words are split at spaces, punctuation and changes of case, so parseHTTPResponse becomes parse, HTTP and Response.")
                        }
                    }
                    .formStyle(.grouped)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .task(id: model.input) {
            await model.convert(model.input)
        }
    }
}
