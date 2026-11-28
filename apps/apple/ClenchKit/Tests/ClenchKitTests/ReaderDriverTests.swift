import Foundation
import Testing
@testable import ClenchKit

/// A clock that only moves when the test says so. Its timers never fire on
/// their own; tests call `wake()` instead.
final class ManualClock: DriverClock, @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0.0

    func now() -> Double { lock.withLock { value } }
    func move(by ms: Double) { lock.withLock { value += ms } }

    func sleep(ms: UInt32) async throws {
        try await Task.sleep(for: .seconds(3600))
    }
}

@MainActor
@Suite struct ReaderDriverTests {
    func session() throws -> Session {
        let doc = try DocumentLoader.extract(
            Data("Read the post yourself. It is short and it is clear.".utf8),
            contentType: "text/plain", url: nil)
        return try Session(document: doc, settingsJson: #"{"wpm": 300, "pacing": {"ease_in_words": 0}}"#)
    }

    @Test func advancesWhenTheClockReachesTheNextChange() throws {
        let session = try session()
        let clock = ManualClock()
        var changes = 0
        let driver = ReaderDriver(playback: session, clock: clock) { changes += 1 }
        driver.control { session.play() }
        #expect(changes == 1)
        let due = try #require(session.nextChangeIn())
        clock.move(by: Double(due))
        driver.wake()
        #expect(session.index() == 1)
        #expect(changes == 2)
        driver.stop()
    }

    @Test func aStallNeverSkipsWordsUnseen() throws {
        let session = try session()
        let clock = ManualClock()
        let driver = ReaderDriver(playback: session, clock: clock) {}
        driver.control { session.play() }
        clock.move(by: 60_000)
        driver.wake()
        #expect(session.index() == 1)
        #expect(session.state() == .playing)
        #expect(driver.playedMs < 60_000)
        driver.stop()
    }

    @Test func carriesSubMillisecondTime() throws {
        let session = try session()
        let clock = ManualClock()
        let driver = ReaderDriver(playback: session, clock: clock) {}
        driver.control { session.play() }
        let due = try #require(session.nextChangeIn())
        for _ in 0..<Int(due) * 2 {
            clock.move(by: 0.5)
            driver.wake()
        }
        #expect(session.index() == 1)
        driver.stop()
    }
}
