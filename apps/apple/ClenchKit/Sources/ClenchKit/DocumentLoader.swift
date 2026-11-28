import Foundation

/// Turns queue items and fetched pages into engine documents. Extraction is
/// the Rust engine's; this only moves bytes.
public struct DocumentLoader: Sendable {
    public enum LoadError: Error, Equatable {
        case nothingToRead
        case httpStatus(Int)
        case tooLarge(limit: Int)
    }

    /// Largest response body read from the network.
    public var maxBytes: Int
    public var session: URLSession

    public init(maxBytes: Int = 20 * 1024 * 1024, session: URLSession = .shared) {
        self.maxBytes = maxBytes
        self.session = session
    }

    /// The document for `item`: its stored content when it has some,
    /// otherwise its URL fetched now.
    public func document(for item: QueueItem, in store: QueueStore) async throws -> Document {
        if let data = try store.content(of: item) {
            return try Self.extract(data, contentType: item.content?.contentType, url: item.url)
        }
        guard let url = item.url else { throw LoadError.nothingToRead }
        return try await fetch(url)
    }

    /// Fetches `url` and extracts it.
    public func fetch(_ url: URL) async throws -> Document {
        let page = try await download(url)
        return try Self.extract(page.data, contentType: page.contentType, url: page.url)
    }

    /// Fetches `url`'s bytes without extracting them.
    public func download(_ url: URL, timeout: TimeInterval = 30) async throws -> (data: Data, contentType: String?, url: URL) {
        var request = URLRequest(url: url)
        request.timeoutInterval = timeout
        let (data, response) = try await session.data(for: request)
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw LoadError.httpStatus(http.statusCode)
        }
        if data.count > maxBytes { throw LoadError.tooLarge(limit: maxBytes) }
        let contentType = (response as? HTTPURLResponse)?.value(forHTTPHeaderField: "Content-Type")
        return (data, contentType, response.url ?? url)
    }

    /// Reads `item` in for the library: extracts its stored content, or
    /// downloads its link and stores the page so it reads offline, then
    /// records the engine's summary (or why it failed) in `store`.
    @discardableResult
    public func readIn(_ item: QueueItem, store: QueueStore, wpm: Int) async -> QueueItem? {
        do {
            let document: Document
            if let data = try store.content(of: item) {
                document = try Self.extract(data, contentType: item.content?.contentType, url: item.url)
            } else if let url = item.url {
                let page = try await download(url)
                document = try Self.extract(page.data, contentType: page.contentType, url: page.url)
                try store.setContent(id: item.id, data: page.data, contentType: page.contentType)
            } else {
                throw LoadError.nothingToRead
            }
            let session = try Session(document: document, settingsJson: ReaderModel.settings(wpm: item.wpm(default: wpm)))
            if item.position.index > 0 { session.seek(index: UInt64(item.position.index)) }
            let summary = ItemSummary(document: document, session: session)
            return try store.update(id: item.id) {
                $0.summary = summary
                $0.failure = nil
                $0.wordCount = summary.words
                if Self.isFallbackTitle($0.title, for: $0), !summary.title.isEmpty {
                    $0.title = summary.title
                }
            }
        } catch {
            return try? store.update(id: item.id) { $0.failure = Self.describe(error) }
        }
    }

    /// Whether `title` is a stand-in made when the item was saved (the
    /// link's host or address), which the document's own title should
    /// replace.
    public static func isFallbackTitle(_ title: String, for item: QueueItem) -> Bool {
        guard let url = item.url else { return false }
        return title == url.host() || title == url.absoluteString || title == "Web page"
    }

    /// A short reason for the library row.
    public static func describe(_ error: Error) -> String {
        switch error {
        case LoadError.httpStatus(let code): "The site answered with error \(code)."
        case LoadError.tooLarge: "The page is too large to read."
        case LoadError.nothingToRead, ClenchError.NoContent: "The link saved, but the text didn’t come through."
        case is URLError: "Couldn’t reach the site. Check the connection and retry."
        default: "The link saved, but the text didn’t come through."
        }
    }

    /// Runs the engine's extractor. It runs on its own large-stack thread
    /// inside the engine, so any caller thread is fine.
    public static func extract(_ data: Data, contentType: String?, url: URL?) throws -> Document {
        try ClenchFFI.extract(bytes: data, contentType: contentType, url: url?.absoluteString)
    }
}
