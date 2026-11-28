import ClenchKit
import SwiftUI

struct RootView: View {
    @Environment(AppModel.self) private var model
    @State private var pasting = false

    var body: some View {
        @Bindable var model = model
        NavigationStack(path: $model.path) {
            QueueView()
                .navigationTitle("clench")
                .toolbar {
                    ToolbarItem(placement: .primaryAction) {
                        Button("Paste", systemImage: "doc.on.clipboard") { pasting = true }
                    }
                }
                .navigationDestination(for: UUID.self) { id in
                    ReaderScreen(itemID: id)
                }
        }
        .sheet(isPresented: $pasting) {
            PasteView()
        }
        .alert("clench", isPresented: Binding(get: { model.message != nil }, set: { if !$0 { model.message = nil } })) {
            Button("OK") { model.message = nil }
        } message: {
            Text(model.message ?? "")
        }
    }
}
