import ClenchKit
import ClenchUI
import SwiftUI

/// Paste text, HTML or a link, then read it now or save it.
struct PasteView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colors) private var colors
    @State private var text = ""

    private var payload: SharedPayload? {
        ShareInput.payload(fromPaste: text)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Wordmark()
                Spacer()
                IconButton(glyph: .close, label: "Cancel") { dismiss() }.padding(.trailing, -12)
            }
            TextEditor(text: $text)
                .font(.plexMono(15, relativeTo: .body))
                .foregroundStyle(colors.ink)
                .scrollContentBackground(.hidden)
                .padding(8)
                .overlay(RoundedRectangle(cornerRadius: 12).stroke(colors.faint, lineWidth: 1))
                .accessibilityLabel("Text or link to read")
            Button("Read now") {
                if let payload { model.read(payload) }
                dismiss()
            }
            .buttonStyle(PrimaryButtonStyle())
            .disabled(payload == nil)
            Button("Save for later") {
                if let payload { model.add(payload) }
                dismiss()
            }
            .buttonStyle(SecondaryButtonStyle())
            .disabled(payload == nil)
        }
        .padding(24)
        .background(colors.sheet.ignoresSafeArea())
        .frame(minWidth: 420, minHeight: 360)
    }
}
