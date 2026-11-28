import Foundation

/// "I have 5 min": the library's time-budget filter.
public enum TimeBudget: Int, CaseIterable, Codable, Sendable, Identifiable {
    case any = 0
    case five = 5
    case fifteen = 15
    case thirty = 30

    public var id: Int { rawValue }

    public var label: String {
        self == .any ? "Any" : "\(rawValue) min"
    }

    /// Whether an item with `ms` left fits. Items whose length is not known
    /// yet fit only when there is no budget.
    public func fits(_ ms: UInt64?) -> Bool {
        guard self != .any else { return true }
        guard let ms else { return false }
        return ReadingTime.minutes(ms) <= rawValue
    }
}

/// The library screen's groups, computed from the queue.
public struct LibraryShelves: Equatable, Sendable {
    /// The item being read most recently, if any is in progress.
    public var current: QueueItem?
    /// Items still to read, in the order they were saved.
    public var upNext: [QueueItem]
    /// Finished items, most recently finished first.
    public var finished: [QueueItem]
    /// When a budget hides every unread item: the shortest one's length in
    /// minutes, if any length is known.
    public var shortestHidden: Int?

    /// Shelves for `items` filtered by `budget` and `search`, with times
    /// estimated at each item's own speed or `wpm`.
    public init(items: [QueueItem], budget: TimeBudget = .any, search: String = "", wpm: Int) {
        let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
        let matching = items.filter { Self.matches($0, query: query) }
        let unread = matching.filter { $0.state != .done }
        let inProgress = unread
            .filter { $0.state == .reading && $0.position.lastRead != nil }
            .max { ($0.position.lastRead ?? .distantPast) < ($1.position.lastRead ?? .distantPast) }

        let fits: (QueueItem) -> Bool = { budget.fits($0.remainingMs(wpm: $0.wpm(default: wpm))) }
        current = inProgress.flatMap { fits($0) ? $0 : nil }
        upNext = unread
            .filter { $0.id != inProgress?.id && fits($0) }
            .sorted { $0.added < $1.added }
        finished = matching
            .filter { $0.state == .done }
            .sorted { ($0.finished ?? $0.added) > ($1.finished ?? $1.added) }

        if budget != .any, current == nil, upNext.isEmpty {
            shortestHidden = unread
                .compactMap { $0.remainingMs(wpm: $0.wpm(default: wpm)) }
                .map(ReadingTime.minutes)
                .min()
        } else {
            shortestHidden = nil
        }
    }

    /// The item to offer after `id` finishes: the first unread item saved
    /// after it, else the first unread one.
    public static func next(after id: UUID, in items: [QueueItem]) -> QueueItem? {
        let unread = items.filter { $0.state != .done && $0.id != id }.sorted { $0.added < $1.added }
        guard let me = items.first(where: { $0.id == id }) else { return unread.first }
        return unread.first { $0.added > me.added } ?? unread.first
    }

    /// Case- and diacritic-insensitive match on the title, attribution,
    /// link and section titles.
    public static func matches(_ item: QueueItem, query: String) -> Bool {
        guard !query.isEmpty else { return true }
        var fields = [item.title]
        if let a = item.attribution { fields.append(a) }
        if let u = item.url?.absoluteString { fields.append(u) }
        if let s = item.summary {
            fields.append(contentsOf: s.sections.compactMap(\.title))
        }
        return fields.contains {
            $0.range(of: query, options: [.caseInsensitive, .diacriticInsensitive]) != nil
        }
    }
}
