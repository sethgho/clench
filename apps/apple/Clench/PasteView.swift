import ClenchKit
import SwiftUI

/// Paste text, HTML or a link, then read it now or queue it.
struct PasteView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var text = ""

    private var payload: SharedPayload? {
        ShareInput.payload(fromPaste: text)
    }

    var body: some View {
        NavigationStack {
            TextEditor(text: $text)
                .font(.body.monospaced())
                .padding()
                .navigationTitle("Paste")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel") { dismiss() }
                    }
                    ToolbarItemGroup(placement: .confirmationAction) {
                        Button("Add to queue") {
                            if let payload { model.add(payload) }
                            dismiss()
                        }
                        .disabled(payload == nil)
                        Button("Read now") {
                            if let payload { model.read(payload) }
                            dismiss()
                        }
                        .disabled(payload == nil)
                    }
                }
        }
        .frame(minWidth: 420, minHeight: 320)
    }
}
