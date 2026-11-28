import ClenchKit
import Foundation
import Observation
import SwiftUI

/// State for the share sheet: what was shared and the two actions. The
/// extension only queues; extraction happens in the app, which keeps the
/// extension well within its memory budget.
@MainActor
@Observable
final class ShareModel {
    enum Phase: Equatable {
        case loading
        case ready(SharedPayload)
        case nothing
        case failed(String)
        case queuedNotOpened
    }

    private(set) var phase: Phase = .loading

    @ObservationIgnored private weak var context: NSExtensionContext?
    @ObservationIgnored private let openURL: @MainActor (URL) async -> Bool

    init(context: NSExtensionContext?, openURL: @escaping @MainActor (URL) async -> Bool) {
        self.context = context
        self.openURL = openURL
    }

    var title: String {
        guard case let .ready(payload) = phase else { return "" }
        return payload.queueEntry().item.title
    }

    func load() async {
        let items = context?.inputItems.compactMap { $0 as? NSExtensionItem } ?? []
        if let payload = await ShareInput.payload(from: items) {
            phase = .ready(payload)
        } else {
            phase = .nothing
        }
    }

    func addToQueue() {
        if enqueue() != nil { finish() }
    }

    func readNow() async {
        guard let item = enqueue() else { return }
        if await openURL(DeepLink.read(item.id).url) {
            finish()
        } else {
            phase = .queuedNotOpened
        }
    }

    func cancel() {
        context?.cancelRequest(withError: CocoaError(.userCancelled))
    }

    func finish() {
        context?.completeRequest(returningItems: nil)
    }

    private func enqueue() -> QueueItem? {
        guard case let .ready(payload) = phase else { return nil }
        let entry = payload.queueEntry()
        do {
            return try QueueStore.shared().add(entry.item, content: entry.content, contentType: entry.contentType)
        } catch {
            phase = .failed(error.localizedDescription)
            return nil
        }
    }
}

/// Placeholder share sheet: the shared item's title, Read now, Add to queue.
struct ShareView: View {
    let model: ShareModel

    var body: some View {
        VStack(spacing: 16) {
            switch model.phase {
            case .loading:
                ProgressView()
            case .ready:
                Text(model.title).font(.headline).lineLimit(3).multilineTextAlignment(.center)
                HStack {
                    Button("Add to queue") { model.addToQueue() }
                    Button("Read now") { Task { await model.readNow() } }
                        .buttonStyle(.borderedProminent)
                }
            case .nothing:
                Text("Nothing here clench can read.")
            case let .failed(message):
                Text(message)
            case .queuedNotOpened:
                Text("Added to your queue. Open clench to read it.")
                Button("Done") { model.finish() }
            }
            Button("Cancel", role: .cancel) { model.cancel() }
        }
        .padding()
        .frame(minWidth: 320, minHeight: 180)
        .task { await model.load() }
    }
}
