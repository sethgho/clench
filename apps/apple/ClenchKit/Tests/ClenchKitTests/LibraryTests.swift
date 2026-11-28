import Foundation
import Testing
@testable import ClenchKit
@testable import ClenchUI

/// A summary for a document of `words` plain words in `sections` headed
/// sections, built by the engine.
private func summary(words: Int, sections: Int = 1, at token: Int = 0) throws -> ItemSummary {
    var blocks: [Block] = []
    let per = max(1, words / sections)
    for s in 0..<sections {
        blocks.append(Block(kind: .heading, text: "Part \(s + 1)", level: 2, depth: nil, ordered: nil, quote: 0, lang: nil, skip: false))
        let text = Array(repeating: "word", count: per).joined(separator: " ")
        blocks.append(Block(kind: .paragraph, text: text, level: nil, depth: nil, ordered: nil, quote: 0, lang: nil, skip: false))
    }
    let doc = Document(url: "", title: "T", byline: "An Author", siteName: nil, lang: nil, excerpt: nil,
                       source: .plainText, partial: false, paywallHint: false, wordCount: 0, blocks: blocks)
    let session = try Session(document: doc, settingsJson: nil)
    if token > 0 { session.seek(index: UInt64(token)) }
    return ItemSummary(document: doc, session: session)
}

private func item(_ title: String, added: TimeInterval, summary: ItemSummary?, state: QueueItem.State = .queued,
                  lastRead: TimeInterval? = nil) -> QueueItem {
    QueueItem(url: URL(string: "https://example.com/\(title)"), title: title,
              added: Date(timeIntervalSince1970: added), state: state,
              position: .init(index: summary?.mark.token ?? 0, wpm: nil, lastRead: lastRead.map(Date.init(timeIntervalSince1970:))),
              summary: summary, finished: state == .done ? Date(timeIntervalSince1970: added + 10) : nil)
}

@Suite struct LibraryTests {
    @Test func summaryComesFromTheEngine() throws {
        let s = try summary(words: 600, sections: 3)
        #expect(s.words == 600 + 3 * 2, "the body plus two words per heading")
        #expect(s.sections.map(\.title) == ["Part 1", "Part 2", "Part 3"])
        #expect(s.ticks.count == 6)
        #expect(s.ticks.filter(\.heading).count == 3)
        #expect(s.ticks.allSatisfy { $0.heading || $0.weight == 1000 })
        #expect(s.mark.token == 0 && s.mark.tick == 0 && !s.started)
        #expect(s.mark.remaining == s.timing)
        // 600 plain words at 300 wpm is about two minutes, plus pauses.
        let ms = s.timing.ms(at: 300)
        #expect((120_000...150_000).contains(ms))
        #expect(s.timing.ms(at: 600) < ms)
        #expect(s.attribution(host: "example.com") == "An Author · example.com")
    }

    @Test func summaryRoundTripsThroughTheQueueFile() throws {
        let dir = try Repo.temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = QueueStore(directory: dir)
        let s = try summary(words: 300, sections: 2, at: 50)
        let saved = try store.add(item("a", added: 0, summary: s))
        #expect(try store.item(id: saved.id)?.summary == s)
        #expect(s.started && s.mark.tick != nil)
    }

    @Test func olderQueueFilesStillLoad() throws {
        let dir = try Repo.temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let json = #"{"version":1,"items":[{"id":"8E0C9A57-8C59-4B45-9C1E-5E9F6E6C3A11","title":"Old","added":"2026-01-01T00:00:00Z","state":"queued","position":{"index":0}}]}"#
        try Data(json.utf8).write(to: dir.appending(path: "queue.json"))
        let items = try QueueStore(directory: dir).items()
        #expect(items.count == 1)
        #expect(items[0].summary == nil && items[0].failure == nil)
    }

    @Test func budgetChipsFilterByMinutesLeft() throws {
        let short = item("short", added: 1, summary: try summary(words: 300))
        let medium = item("medium", added: 2, summary: try summary(words: 3000))
        let long = item("long", added: 3, summary: try summary(words: 12000))
        let unknown = item("unknown", added: 4, summary: nil)
        let items = [long, unknown, medium, short]

        let any = LibraryShelves(items: items, budget: .any, wpm: 300)
        #expect(any.upNext.map(\.title) == ["short", "medium", "long", "unknown"])
        #expect(LibraryShelves(items: items, budget: .five, wpm: 300).upNext.map(\.title) == ["short"])
        #expect(LibraryShelves(items: items, budget: .fifteen, wpm: 300).upNext.map(\.title) == ["short", "medium"])
        #expect(LibraryShelves(items: items, budget: .fifteen, wpm: 600).upNext.map(\.title) == ["short", "medium"])
        #expect(LibraryShelves(items: items, budget: .thirty, wpm: 600).upNext.map(\.title) == ["short", "medium", "long"])

        let tooShort = LibraryShelves(items: [medium, long], budget: .five, wpm: 300)
        #expect(tooShort.upNext.isEmpty)
        #expect(tooShort.shortestHidden == ReadingTime.minutes(medium.remainingMs(wpm: 300)!))
    }

    @Test func currentFinishedAndUpNextAreSeparate() throws {
        let reading = item("reading", added: 1, summary: try summary(words: 900, at: 400), state: .reading, lastRead: 50)
        let older = item("older", added: 0, summary: try summary(words: 900, at: 100), state: .reading, lastRead: 20)
        let done = item("done", added: 2, summary: try summary(words: 100), state: .done)
        let queued = item("queued", added: 3, summary: try summary(words: 100))
        let shelves = LibraryShelves(items: [queued, done, older, reading], wpm: 300)
        #expect(shelves.current?.title == "reading")
        #expect(shelves.upNext.map(\.title) == ["older", "queued"])
        #expect(shelves.finished.map(\.title) == ["done"])
        #expect(done.remainingMs(wpm: 300) == 0)
        #expect(reading.remainingMs(wpm: 300)! < reading.totalMs(wpm: 300)!)
    }

    @Test func searchMatchesTitlesAttributionAndSections() throws {
        let a = item("Self-Reliance", added: 0, summary: try summary(words: 100, sections: 2))
        let b = item("Café society", added: 1, summary: nil)
        let items = [a, b]
        #expect(LibraryShelves(items: items, search: "reliance", wpm: 300).upNext.map(\.title) == ["Self-Reliance"])
        #expect(LibraryShelves(items: items, search: "cafe", wpm: 300).upNext.map(\.title) == ["Café society"])
        #expect(LibraryShelves(items: items, search: "part 2", wpm: 300).upNext.map(\.title) == ["Self-Reliance"])
        #expect(LibraryShelves(items: items, search: "an author", wpm: 300).upNext.count == 1)
        #expect(LibraryShelves(items: items, search: "example.com", wpm: 300).upNext.count == 2)
        #expect(LibraryShelves(items: items, search: "  ", wpm: 300).upNext.count == 2)
    }

    @Test func upNextAfterAnItem() throws {
        let a = item("a", added: 0, summary: nil, state: .done)
        let b = item("b", added: 1, summary: nil)
        let c = item("c", added: 2, summary: nil)
        #expect(LibraryShelves.next(after: b.id, in: [a, b, c])?.title == "c")
        #expect(LibraryShelves.next(after: c.id, in: [a, b, c])?.title == "b")
        #expect(LibraryShelves.next(after: c.id, in: [a, c]) == nil)
    }

    @Test func minutesRoundToTheNearestAndNeverShowZeroForSomething() {
        #expect(ReadingTime.minutes(0) == 0)
        #expect(ReadingTime.minutes(1) == 1)
        #expect(ReadingTime.minutes(89_999) == 1)
        #expect(ReadingTime.minutes(90_000) == 2)
        #expect(ReadingTime.label(42 * 60_000) == "42 min")
    }

    @Test func terrainStatesFollowTheEnginePosition() throws {
        let fresh = TerrainModel(summary: try summary(words: 600, sections: 3))
        #expect(fresh.current == nil)
        #expect(fresh.state(0) == .unread)

        let midway = try summary(words: 600, sections: 3, at: 250)
        let model = TerrainModel(summary: midway)
        let current = try #require(midway.mark.tick)
        #expect(model.current == current)
        #expect(model.state(current) == .current)
        #expect(model.state(0) == .read)
        #expect(model.state(current + 1) == .unread)
        let tick = midway.ticks[current]
        #expect(tick.tokenStart <= 250 && 250 < tick.tokenEnd)

        let done = TerrainModel(summary: midway, done: true)
        #expect(done.state(midway.ticks.count - 1) == .read)
    }

    @Test func readerModelExposesTheEngineMap() async throws {
        let doc = try DocumentLoader.extract(try Repo.fixture("06-wikipedia.html").data, contentType: "text/html",
                                             url: try Repo.fixture("06-wikipedia.html").url)
        let model = try await ReaderModel(document: doc, settingsJSON: ReaderModel.settings(wpm: 320), tickCount: 60)
        await MainActor.run {
            #expect(model.ticks.count <= max(60, 2 * model.outline.sections.count))
            #expect(model.position.tick == 0)
            #expect(model.sentence != nil)
            model.seek(tick: model.ticks.count - 1)
            #expect(model.position.tick == UInt64(model.ticks.count - 1))
            let last = model.outline.sections.count - 1
            #expect(model.position.section == UInt64(last))
            #expect(model.remainingMs(inSection: 0) == 0)
            #expect(model.remainingMs(inSection: last) > 0)
            model.setWpm(5000)
            #expect(model.frame.wpm == 1500)
            model.stop()
        }
    }
}
