import ClenchKit
import ClenchUI
import SwiftUI

@main
struct ClenchApp: App {
    @State private var model: AppModel
    @Environment(\.scenePhase) private var scenePhase

    init() {
        ClenchFonts.register()
        _model = State(initialValue: AppModel())
    }

    var body: some Scene {
        #if os(macOS)
        Window("clench", id: "library") {
            MacLibraryWindow()
                .environment(model)
                .clenchTheme(model.preferences)
                .frame(minWidth: 820, minHeight: 560)
                .onOpenURL { model.handle($0) }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active { model.reload() }
                }
        }
        .defaultSize(width: 1120, height: 720)
        .commands {
            CommandGroup(after: .pasteboard) {
                PasteCommand(model: model)
            }
        }

        WindowGroup("Reader", id: "reader", for: UUID.self) { $id in
            if let id {
                MacReaderWindow(itemID: id)
                    .environment(model)
                    .clenchTheme(model.preferences)
            }
        }
        .defaultSize(width: 1280, height: 800)

        Settings {
            SettingsView()
                .environment(model)
                .clenchTheme(model.preferences)
                .frame(width: 560)
                .frame(minHeight: 620)
        }
        #else
        WindowGroup {
            RootView()
                .environment(model)
                .clenchTheme(model.preferences)
                .onOpenURL { model.handle($0) }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active { model.reload() }
                }
        }
        #endif
    }
}

#if os(macOS)
/// The library window; opening an item opens a reader window.
struct MacLibraryWindow: View {
    @Environment(AppModel.self) private var model
    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        SplitLibraryView(onSettings: { openSettings() })
            .onChange(of: model.reading) { _, id in
                guard let id else { return }
                openWindow(id: "reader", value: id)
                model.reading = nil
            }
    }
}

/// A reader window, full screen unless the settings say otherwise. Esc
/// leaves full screen and closes it.
struct MacReaderWindow: View {
    let itemID: UUID
    @Environment(AppModel.self) private var model
    @State private var current: UUID?
    @State private var window: NSWindow?

    var body: some View {
        ReaderScreen(itemID: current ?? itemID, onClose: close)
            .background(WindowReader { w in
                window = w
                w.titlebarAppearsTransparent = true
                w.titleVisibility = .hidden
                w.title = model.item(id: itemID)?.title ?? "Reader"
                if model.preferences.macReader == .fullScreen, !w.styleMask.contains(.fullScreen) {
                    w.toggleFullScreen(nil)
                }
            })
            .onChange(of: model.reading) { _, id in
                guard let id else { return }
                current = id
                model.reading = nil
            }
    }

    private func close() {
        guard let window else { return }
        if window.styleMask.contains(.fullScreen) {
            window.toggleFullScreen(nil)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { window.close() }
        } else {
            window.close()
        }
    }
}

struct PasteCommand: View {
    let model: AppModel

    var body: some View {
        Button("Save Clipboard for Later") {
            let pb = NSPasteboard.general
            if let url = pb.readObjects(forClasses: [NSURL.self])?.first as? URL {
                model.add(.url(url, title: nil))
            } else if let text = pb.string(forType: .string), let payload = ShareInput.payload(fromPaste: text) {
                model.add(payload)
            }
        }
        .keyboardShortcut("v", modifiers: [.command, .shift])
    }
}
#endif
