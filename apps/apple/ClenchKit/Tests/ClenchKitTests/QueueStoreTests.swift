import Foundation
import Testing
@testable import ClenchKit

@Suite struct QueueStoreTests {
    @Test func roundTripsItemsAndContent() throws {
        let dir = try Repo.temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = QueueStore(directory: dir)
        #expect(try store.items().isEmpty)

        let added = Date(timeIntervalSince1970: 1_800_000_000)
        let page = SharedPayload.page(html: "<p>Hello there.</p>", url: URL(string: "https://example.com/post"), title: "A post")
        let entry = page.queueEntry(added: added)
        let saved = try store.add(entry.item, content: entry.content, contentType: entry.contentType)
        let link = SharedPayload.url(URL(string: "https://example.org/x")!, title: nil).queueEntry(added: added.addingTimeInterval(60))
        try store.add(link.item)

        // A second store on the same directory sees the same queue, as the
        // share extension and the app do.
        let other = QueueStore(directory: dir)
        let items = try other.items()
        #expect(items.map(\.title) == ["example.org", "A post"])
        let loaded = try #require(try other.item(id: saved.id))
        #expect(loaded == saved)
        #expect(loaded.added == added)
        #expect(loaded.content?.contentType == "text/html; charset=utf-8")
        #expect(try other.content(of: loaded) == Data("<p>Hello there.</p>".utf8))

        let read = Date(timeIntervalSince1970: 1_800_000_500)
        try other.update(id: saved.id) {
            $0.state = .reading
            $0.wordCount = 2
            $0.position = .init(index: 1, wpm: 420, lastRead: read)
        }
        let updated = try #require(try store.item(id: saved.id))
        #expect(updated.state == .reading)
        #expect(updated.position == .init(index: 1, wpm: 420, lastRead: read))

        try store.remove(id: saved.id)
        #expect(try store.items().map(\.id) == [link.item.id])
        #expect(!FileManager.default.fileExists(atPath: store.contentDirectory.appending(path: saved.id.uuidString).path(percentEncoded: false)))
    }

    @Test func storedPageIsReadableByTheEngine() async throws {
        let dir = try Repo.temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = QueueStore(directory: dir)
        let entry = SharedPayload.text("First line here.\n\nSecond paragraph.", title: nil).queueEntry()
        #expect(entry.item.title == "First line here.")
        let item = try store.add(entry.item, content: entry.content, contentType: entry.contentType)
        let doc = try await DocumentLoader().document(for: item, in: store)
        #expect(doc.wordCount == 5)
    }

    @Test func refusesNewerFormats() throws {
        let dir = try Repo.temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = QueueStore(directory: dir)
        try Data(#"{"version": 99, "items": []}"#.utf8).write(to: store.queueURL)
        #expect(throws: QueueStore.StoreError.unsupportedVersion(99)) { try store.items() }
    }

    @Test func deepLinksRoundTrip() throws {
        let id = UUID()
        #expect(DeepLink(url: DeepLink.read(id).url) == .read(id))
        #expect(DeepLink(url: URL(string: "clench://queue")!) == .queue)
        #expect(DeepLink(url: URL(string: "clench://read?id=nope")!) == nil)
        #expect(DeepLink(url: URL(string: "https://example.com")!) == nil)
    }
}
