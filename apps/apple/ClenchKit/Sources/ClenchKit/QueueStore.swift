import Foundation

/// The read-later queue on disk: `queue.json` plus a `Content` directory of
/// stored payloads, in a directory both the app and the share extension can
/// reach (see ``AppGroup``).
///
/// Every operation is a coordinated read-modify-write of `queue.json`, so
/// the app and the extension can change the queue at the same time without
/// losing each other's writes.
public struct QueueStore: Sendable {
    public static let formatVersion = 1

    public enum StoreError: Error, Equatable {
        case unsupportedVersion(Int)
        case missingItem(UUID)
    }

    private struct File: Codable {
        var version: Int
        var items: [QueueItem]
    }

    public let directory: URL

    public var queueURL: URL { directory.appending(path: "queue.json") }
    public var contentDirectory: URL { directory.appending(path: "Content", directoryHint: .isDirectory) }

    public init(directory: URL) {
        self.directory = directory
    }

    /// The store in the shared App Group container (or its fallback).
    public static func shared() throws -> QueueStore {
        QueueStore(directory: try AppGroup.storageDirectory())
    }

    /// All items, newest first.
    public func items() throws -> [QueueItem] {
        try coordinated(writing: false) { try read().items }
            .sorted { $0.added > $1.added }
    }

    public func item(id: UUID) throws -> QueueItem? {
        try items().first { $0.id == id }
    }

    /// Adds `item`, storing `content` next to it when given.
    @discardableResult
    public func add(_ item: QueueItem, content: Data? = nil, contentType: String? = nil) throws -> QueueItem {
        var item = item
        if let content {
            try FileManager.default.createDirectory(at: contentDirectory, withIntermediateDirectories: true)
            let name = item.id.uuidString
            try content.write(to: contentDirectory.appending(path: name), options: .atomic)
            item.content = QueueItem.StoredContent(file: name, contentType: contentType)
        }
        let stored = item
        try coordinated(writing: true) {
            var file = try read()
            file.items.removeAll { $0.id == stored.id }
            file.items.append(stored)
            try write(file)
        }
        return stored
    }

    /// Applies `change` to the item with `id` and saves it.
    @discardableResult
    public func update(id: UUID, _ change: @Sendable (inout QueueItem) -> Void) throws -> QueueItem {
        try coordinated(writing: true) {
            var file = try read()
            guard let i = file.items.firstIndex(where: { $0.id == id }) else {
                throw StoreError.missingItem(id)
            }
            change(&file.items[i])
            try write(file)
            return file.items[i]
        }
    }

    /// Stores `data` as the content of the item with `id`, replacing any.
    public func setContent(id: UUID, data: Data, contentType: String?) throws {
        try FileManager.default.createDirectory(at: contentDirectory, withIntermediateDirectories: true)
        let name = id.uuidString
        try data.write(to: contentDirectory.appending(path: name), options: .atomic)
        try update(id: id) { $0.content = QueueItem.StoredContent(file: name, contentType: contentType) }
    }

    /// Removes the item and its stored content.
    public func remove(id: UUID) throws {
        let removed: QueueItem? = try coordinated(writing: true) {
            var file = try read()
            guard let i = file.items.firstIndex(where: { $0.id == id }) else { return nil }
            let item = file.items.remove(at: i)
            try write(file)
            return item
        }
        if let name = removed?.content?.file {
            try? FileManager.default.removeItem(at: contentDirectory.appending(path: name))
        }
    }

    /// The stored payload of `item`, if it has one.
    public func content(of item: QueueItem) throws -> Data? {
        guard let name = item.content?.file else { return nil }
        return try Data(contentsOf: contentDirectory.appending(path: name))
    }

    // MARK: - File access

    private func read() throws -> File {
        guard FileManager.default.fileExists(atPath: queueURL.path(percentEncoded: false)) else {
            return File(version: Self.formatVersion, items: [])
        }
        let file = try Self.decoder.decode(File.self, from: Data(contentsOf: queueURL))
        guard file.version <= Self.formatVersion else {
            throw StoreError.unsupportedVersion(file.version)
        }
        return file
    }

    private func write(_ file: File) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Self.encoder.encode(file).write(to: queueURL, options: .atomic)
    }

    private func coordinated<T>(writing: Bool, _ body: () throws -> T) throws -> T {
        let coordinator = NSFileCoordinator()
        var coordinationError: NSError?
        var result: Result<T, Error>?
        if writing {
            coordinator.coordinate(writingItemAt: queueURL, options: [], error: &coordinationError) { _ in
                result = Result { try body() }
            }
        } else {
            coordinator.coordinate(readingItemAt: queueURL, options: [], error: &coordinationError) { _ in
                result = Result { try body() }
            }
        }
        if let coordinationError { throw coordinationError }
        guard let result else { throw CocoaError(.fileReadUnknown) }
        return try result.get()
    }

    private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.prettyPrinted, .sortedKeys]
        return e
    }()

    private static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()
}
