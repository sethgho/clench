import ClenchKit
import Foundation
import Observation
#if canImport(WidgetKit)
import WidgetKit
#endif

/// App-wide state: the queue, the preferences and which item is open.
@MainActor
@Observable
final class AppModel {
    private(set) var items: [QueueItem] = []
    /// The item open in the reader (iPhone, iPad: full screen).
    var reading: QueueItem.ID?
    var message: String?
    /// Where to start an item that is being opened at a section.
    @ObservationIgnored private var pendingSeek: [UUID: Int] = [:]
    /// Items being downloaded or summarized now.
    private(set) var readingIn: Set<UUID> = []

    var preferences: Preferences {
        didSet {
            guard preferences != oldValue else { return }
            preferencesStore.save(preferences)
            reloadWidgets()
        }
    }

    @ObservationIgnored let store: QueueStore?
    @ObservationIgnored private let preferencesStore: PreferencesStore
    @ObservationIgnored private let loader = DocumentLoader()

    init(store: QueueStore? = nil, preferencesStore: PreferencesStore = PreferencesStore()) {
        self.preferencesStore = preferencesStore
        preferences = preferencesStore.load()
        if let store {
            self.store = store
        } else {
            do {
                self.store = try QueueStore.shared()
            } catch {
                self.store = nil
                message = "The queue is unavailable: \(error.localizedDescription)"
            }
        }
        #if DEBUG
        if let store = self.store { Demo.seed(store, wpm: preferences.defaultWpm) }
        #endif
        reload()
    }

    func reload() {
        guard let store else { return }
        do {
            items = try store.items()
            preferences = preferencesStore.load()
        } catch {
            message = "Could not read the queue: \(error.localizedDescription)"
        }
        readInPending()
    }

    func shelves(budget: TimeBudget = .any, search: String = "") -> LibraryShelves {
        LibraryShelves(items: items, budget: budget, search: search, wpm: preferences.defaultWpm)
    }

    func item(id: UUID) -> QueueItem? {
        items.first { $0.id == id } ?? (try? store?.item(id: id)) ?? nil
    }

    /// Opens `id` in the reader, at token `token` when given.
    func open(_ id: UUID, at token: Int? = nil) {
        pendingSeek[id] = token
        reading = id
    }

    /// The start token asked for by ``open(_:at:)``, once.
    func takeSeek(_ id: UUID) -> Int? {
        pendingSeek.removeValue(forKey: id)
    }

    func isBusy(_ id: UUID) -> Bool {
        readingIn.contains(id)
    }

    func next(after id: UUID) -> QueueItem? {
        LibraryShelves.next(after: id, in: items)
    }

    func wpm(for item: QueueItem) -> Int {
        item.wpm(default: preferences.defaultWpm)
    }

    // MARK: - Changing the queue

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
        if let item = add(payload) { reading = item.id }
    }

    func remove(_ ids: [UUID]) {
        guard let store else { return }
        for id in ids { try? store.remove(id: id) }
        reload()
        reloadWidgets()
    }

    func markFinished(_ id: UUID, _ done: Bool = true) {
        guard let store else { return }
        _ = try? store.update(id: id) {
            $0.state = done ? .done : .queued
            $0.finished = done ? .now : nil
            if !done { $0.position = .init() }
        }
        reload()
        reloadWidgets()
    }

    /// Records where reading stopped, with the engine's summary at that point.
    func save(id: UUID, reader: ReaderModel) {
        guard let store else { return }
        let finished = reader.state == .finished
        let summary = reader.summary()
        let index = Int(reader.frame.index), wpm = Int(reader.frame.wpm), words = reader.wordCount
        _ = try? store.update(id: id) { item in
            item.position = .init(index: index, wpm: wpm, lastRead: .now)
            item.wordCount = words
            item.summary = summary
            item.failure = nil
            if finished {
                item.state = .done
                item.finished = .now
            } else {
                item.state = .reading
            }
            if DocumentLoader.isFallbackTitle(item.title, for: item), !summary.title.isEmpty {
                item.title = summary.title
            }
        }
        reload()
        reloadWidgets()
    }

    func retry(_ id: UUID) {
        guard let store else { return }
        _ = try? store.update(id: id) { $0.failure = nil }
        reload()
    }

    /// Downloads and summarizes items that have no summary yet, a few at a
    /// time, so the library can show their length and map.
    private func readInPending() {
        guard let store else { return }
        let pending = items.filter { $0.summary == nil && $0.failure == nil && !readingIn.contains($0.id) }
        guard !pending.isEmpty else { return }
        let wpm = preferences.defaultWpm
        let loader = self.loader
        for item in pending.prefix(3) {
            readingIn.insert(item.id)
            Task { [weak self] in
                await loader.readIn(item, store: store, wpm: wpm)
                guard let self else { return }
                self.readingIn.remove(item.id)
                self.items = (try? store.items()) ?? self.items
                self.readInPending()
                self.reloadWidgets()
            }
        }
    }

    func handle(_ url: URL) {
        reload()
        switch DeepLink(url: url) {
        case let .read(id)?:
            reading = id
        case .queue?, nil:
            reading = nil
        }
    }

    private func reloadWidgets() {
        #if canImport(WidgetKit)
        WidgetCenter.shared.reloadAllTimelines()
        #endif
    }
}
