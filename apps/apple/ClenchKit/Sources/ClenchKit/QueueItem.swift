import Foundation

/// One entry of the read-later queue.
///
/// The field set follows the queue format planned for every host
/// (https://github.com/sethgho/clench/issues/6); until that format lives in
/// the Rust core, this is the Apple app's own on-disk shape.
public struct QueueItem: Codable, Identifiable, Hashable, Sendable {
    public enum State: String, Codable, Sendable {
        case queued, reading, done
    }

    /// Where reading stopped.
    public struct Position: Codable, Hashable, Sendable {
        /// Index of the current token in the session's script.
        public var index: Int
        /// Words per minute when reading stopped.
        public var wpm: Int?
        public var lastRead: Date?

        public init(index: Int = 0, wpm: Int? = nil, lastRead: Date? = nil) {
            self.index = index
            self.wpm = wpm
            self.lastRead = lastRead
        }
    }

    /// Bytes stored with the item (page HTML from Safari, pasted text), so
    /// it can be read without fetching. `file` is relative to the queue's
    /// content directory.
    public struct StoredContent: Codable, Hashable, Sendable {
        public var file: String
        public var contentType: String?

        public init(file: String, contentType: String?) {
            self.file = file
            self.contentType = contentType
        }
    }

    public var id: UUID
    public var url: URL?
    public var title: String
    public var added: Date
    public var state: State
    /// Words the default script reads; nil until the item is first opened.
    public var wordCount: Int?
    public var position: Position
    public var content: StoredContent?
    /// The engine's outline and position, once the item has been read in.
    public var summary: ItemSummary?
    /// Why the item could not be read in, when it could not.
    public var failure: String?
    /// When the item was finished.
    public var finished: Date?

    public init(
        id: UUID = UUID(), url: URL?, title: String, added: Date = .now,
        state: State = .queued, wordCount: Int? = nil,
        position: Position = Position(), content: StoredContent? = nil,
        summary: ItemSummary? = nil, failure: String? = nil, finished: Date? = nil
    ) {
        self.id = id
        self.url = url
        self.title = title
        self.added = added
        self.state = state
        self.wordCount = wordCount
        self.position = position
        self.content = content
        self.summary = summary
        self.failure = failure
        self.finished = finished
    }

    /// The site to credit: the document's site name, else the link's host.
    public var host: String? {
        url?.host().map { $0.hasPrefix("www.") ? String($0.dropFirst(4)) : $0 }
    }

    /// "Byline · site", when either is known.
    public var attribution: String? {
        summary?.attribution(host: host) ?? host
    }

    /// Words per minute to estimate this item at.
    public func wpm(default preferred: Int) -> Int {
        position.wpm ?? preferred
    }

    /// Time left to read at `wpm`, by the engine's estimate; nil until the
    /// item has been read in.
    public func remainingMs(wpm: Int) -> UInt64? {
        guard let summary else { return nil }
        if state == .done { return 0 }
        return summary.mark.remaining.ms(at: wpm)
    }

    /// Time to read the whole item at `wpm`; nil until read in.
    public func totalMs(wpm: Int) -> UInt64? {
        summary?.timing.ms(at: wpm)
    }
}
