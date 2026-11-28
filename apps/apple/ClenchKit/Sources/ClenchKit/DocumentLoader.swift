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
        var request = URLRequest(url: url)
        request.timeoutInterval = 30
        let (data, response) = try await session.data(for: request)
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw LoadError.httpStatus(http.statusCode)
        }
        if data.count > maxBytes { throw LoadError.tooLarge(limit: maxBytes) }
        let contentType = (response as? HTTPURLResponse)?.value(forHTTPHeaderField: "Content-Type")
        return try Self.extract(data, contentType: contentType, url: response.url ?? url)
    }

    /// Runs the engine's extractor. It runs on its own large-stack thread
    /// inside the engine, so any caller thread is fine.
    public static func extract(_ data: Data, contentType: String?, url: URL?) throws -> Document {
        try ClenchFFI.extract(bytes: data, contentType: contentType, url: url?.absoluteString)
    }
}
