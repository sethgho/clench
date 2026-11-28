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

    public init(
        id: UUID = UUID(), url: URL?, title: String, added: Date = .now,
        state: State = .queued, wordCount: Int? = nil,
        position: Position = Position(), content: StoredContent? = nil
    ) {
        self.id = id
        self.url = url
        self.title = title
        self.added = added
        self.state = state
        self.wordCount = wordCount
        self.position = position
        self.content = content
    }
}
