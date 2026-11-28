import Foundation
import Observation

/// Observable state for a reader view: a session, the frame to draw and the
/// clock that drives it. Views call the controls; everything about what is
/// shown and when comes from the engine.
@MainActor
@Observable
public final class ReaderModel {
    public let document: Document
    /// Sections and totals; fixed for the session.
    public let outline: Outline
    /// The document map at ``tickCount`` ticks; fixed for the session.
    public let ticks: [TerrainTick]
    public let tickCount: UInt32

    public private(set) var frame: Frame
    public private(set) var context: Context?
    public private(set) var position: ReadingPosition
    public private(set) var sentence: SentenceView?

    @ObservationIgnored private let session: Session
    @ObservationIgnored private var driver: ReaderDriver?

    public init(document: Document, settingsJSON: String? = nil, tickCount: UInt32 = 96,
                clock: DriverClock = SystemClock()) throws {
        self.document = document
        let session = try Session(document: document, settingsJson: settingsJSON)
        self.session = session
        self.tickCount = tickCount
        outline = session.outline()
        ticks = session.terrain(maxTicks: tickCount)
        frame = session.frame()
        context = session.context()
        position = session.position(maxTicks: tickCount)
        sentence = session.sentence()
        driver = ReaderDriver(playback: session, clock: clock) { [weak self] in
            self?.refresh()
        }
    }

    /// Settings JSON for a session at `wpm`.
    public nonisolated static func settings(wpm: Int) -> String {
        #"{"wpm": \#(max(1, wpm))}"#
    }

    public var state: PlayState { frame.state }
    public var isPlaying: Bool { frame.state == .playing }
    public var wordCount: Int { Int(session.wordCount()) }
    public var omitted: [Omitted] { session.omitted() }

    /// Exact time left at the current speed, from the engine's player.
    public var remainingMs: UInt64 { session.remainingMs() }
    /// Time for the whole script at the current speed.
    public var totalMs: UInt64 { session.totalMs() }

    /// The current section, if reading has not finished.
    public var currentSection: OutlineSection? {
        position.section.flatMap { s in outline.sections.indices.contains(Int(s)) ? outline.sections[Int(s)] : nil }
    }

    /// Time left in `section` (an index into ``Outline/sections``): all of it
    /// ahead of the current one, what is left of the current one, none
    /// behind.
    public func remainingMs(inSection index: Int) -> UInt64 {
        guard outline.sections.indices.contains(index) else { return 0 }
        guard let current = position.section.map(Int.init) else { return 0 }
        if index < current { return 0 }
        let timing = index == current ? position.sectionRemaining : outline.sections[index].timing
        return readingMs(timing: timing, wpm: frame.wpm)
    }

    /// A summary for the library at the current position.
    public func summary() -> ItemSummary {
        ItemSummary(document: document, session: session)
    }

    public func toggle() { apply { $0.toggle() } }
    public func play() { apply { $0.play() } }
    public func pause() { apply { $0.pause() } }
    public func restart() { apply { $0.restart() } }
    public func step(_ delta: Int32) { apply { $0.step(delta: delta) } }
    public func nextSentence() { apply { $0.nextSentence() } }
    public func prevSentence() { apply { $0.prevSentence() } }
    public func nextParagraph() { apply { $0.nextParagraph() } }
    public func prevParagraph() { apply { $0.prevParagraph() } }
    public func seek(_ index: Int) { apply { $0.seek(index: UInt64(max(0, index))) } }
    public func setWpm(_ wpm: Int) { apply { $0.setWpm(wpm: UInt32(clamping: max(1, wpm))) } }
    public func nudgeWpm(_ delta: Int32) { apply { $0.nudgeWpm(delta: delta) } }

    /// Moves to the start of `tick` in ``ticks``.
    public func seek(tick: Int) {
        guard ticks.indices.contains(tick) else { return }
        seek(Int(ticks[tick].tokenStart))
    }

    /// Moves to the start of section `index`.
    public func seek(section index: Int) {
        guard outline.sections.indices.contains(index) else { return }
        seek(Int(outline.sections[index].tokenStart))
    }

    /// Stops the clock; call when the view goes away.
    public func stop() {
        driver?.control { session.pause() }
        driver?.stop()
    }

    private func apply(_ control: (Session) -> Void) {
        let session = self.session
        driver?.control { control(session) }
    }

    private func refresh() {
        frame = session.frame()
        position = session.position(maxTicks: tickCount)
        sentence = session.sentence()
        context = session.context()
    }
}
