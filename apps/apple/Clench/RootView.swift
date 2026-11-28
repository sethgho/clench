import ClenchKit
import ClenchUI
import SwiftUI

/// The library, and the reader over it when an item is open (iPhone, iPad).
/// Wide windows get the two-column library.
struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors
    @State private var settings = false
    @State private var pasting = false

    var body: some View {
        @Bindable var model = model
        GeometryReader { geo in
            Group {
                if geo.size.width >= 700 {
                    SplitLibraryView(onSettings: { settings = true })
                } else {
                    LibraryView(onSettings: { settings = true })
                }
            }
        }
        .background(colors.bg.ignoresSafeArea())
        #if os(iOS)
        .fullScreenCover(item: $model.reading) { id in
            ReaderScreen(itemID: id, onClose: { model.reading = nil })
                .clenchTheme(model.preferences)
        }
        #endif
        .sheet(isPresented: $settings) {
            NavigationStack {
                SettingsView(onClose: { settings = false })
                    .hideNavigationBar()
            }
            .clenchTheme(model.preferences)
        }
        .sheet(isPresented: $pasting) {
            PasteView().clenchTheme(model.preferences)
        }
        .alert("clench", isPresented: Binding(get: { model.message != nil }, set: { if !$0 { model.message = nil } })) {
            Button("OK") { model.message = nil }
        } message: {
            Text(model.message ?? "")
        }
    }
}

extension UUID: @retroactive Identifiable {
    public var id: UUID { self }
}
