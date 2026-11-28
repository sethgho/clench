import ClenchKit
import Foundation
import Observation

/// App-wide state: the queue and navigation into the reader.
@MainActor
@Observable
final class AppModel {
    private(set) var items: [QueueItem] = []
    var path: [UUID] = []
    var message: String?

    @ObservationIgnored let store: QueueStore?

    init() {
        do {
            store = try QueueStore.shared()
        } catch {
            store = nil
            message = "The queue is unavailable: \(error.localizedDescription)"
        }
        reload()
    }

    func reload() {
        guard let store else { return }
        do {
            items = try store.items()
        } catch {
            message = "Could not read the queue: \(error.localizedDescription)"
        }
    }

    /// Queues `payload`; returns the new item.
    @discardableResult
    func add(_ payload: SharedPayload) -> QueueItem? {
        guard let store else { return nil }
        let entry = payload.queueEntry()
        do {
            let item = try store.add(entry.item, content: entry.content, contentType: entry.contentType)
            reload()
            return item
        } catch {
            message = "Could not add to the queue: \(error.localizedDescription)"
            return nil
        }
    }

    func read(_ payload: SharedPayload) {
        if let item = add(payload) { path = [item.id] }
    }

    func remove(_ ids: [UUID]) {
        guard let store else { return }
        for id in ids { try? store.remove(id: id) }
        reload()
    }

    func item(id: UUID) -> QueueItem? {
        items.first { $0.id == id } ?? (try? store?.item(id: id)) ?? nil
    }

    func handle(_ url: URL) {
        reload()
        switch DeepLink(url: url) {
        case let .read(id)?:
            path = [id]
        case .queue?, nil:
            path = []
        }
    }

    func savePosition(id: UUID, index: Int, wpm: Int, wordCount: Int, finished: Bool) {
        guard let store else { return }
        _ = try? store.update(id: id) { item in
            item.position = .init(index: index, wpm: wpm, lastRead: .now)
            item.wordCount = wordCount
            item.state = finished ? .done : .reading
        }
        reload()
    }
}
