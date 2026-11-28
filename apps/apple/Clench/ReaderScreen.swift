import ClenchKit
import SwiftUI

/// Loads a queued item and shows it in the reader, saving the position when
/// the reader goes away.
struct ReaderScreen: View {
    let itemID: UUID

    @Environment(AppModel.self) private var model
    @AppStorage("wpm") private var preferredWpm = 300
    @State private var phase: Phase = .loading

    private enum Phase {
        case loading
        case ready(ReaderModel)
        case failed(String)
    }

    var body: some View {
        Group {
            switch phase {
            case .loading:
                ProgressView()
            case let .ready(reader):
                ReaderView(reader: reader)
                    .onDisappear { save(reader) }
            case let .failed(message):
                ContentUnavailableView("Can’t read this", systemImage: "exclamationmark.triangle", description: Text(message))
            }
        }
        .navigationTitle(model.item(id: itemID)?.title ?? "")
        .task(id: itemID) { await load() }
    }

    private func load() async {
        guard let store = model.store, let item = model.item(id: itemID) else {
            phase = .failed("This item is no longer in the queue.")
            return
        }
        do {
            let document = try await DocumentLoader().document(for: item, in: store)
            let wpm = item.position.wpm ?? preferredWpm
            let reader = try ReaderModel(document: document, settingsJSON: #"{"wpm": \#(wpm)}"#)
            if item.state != .done { reader.seek(item.position.index) }
            phase = .ready(reader)
        } catch {
            phase = .failed(String(describing: error))
        }
    }

    private func save(_ reader: ReaderModel) {
        reader.stop()
        preferredWpm = Int(reader.frame.wpm)
        model.savePosition(
            id: itemID, index: Int(reader.frame.index), wpm: Int(reader.frame.wpm),
            wordCount: reader.wordCount, finished: reader.state == .finished)
    }
}
