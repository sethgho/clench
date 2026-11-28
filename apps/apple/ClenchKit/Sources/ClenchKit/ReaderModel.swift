import Foundation
import Observation

/// Observable state for a reader view: a session, the frame to draw and the
/// clock that drives it. Views call the controls; everything about what is
/// shown and when comes from the engine.
@MainActor
@Observable
public final class ReaderModel {
    public let document: Document
    public private(set) var frame: Frame
    public private(set) var context: Context?

    @ObservationIgnored private let session: Session
    @ObservationIgnored private var driver: ReaderDriver?

    public init(document: Document, settingsJSON: String? = nil, clock: DriverClock = SystemClock()) throws {
        self.document = document
        let session = try Session(document: document, settingsJson: settingsJSON)
        self.session = session
        self.frame = session.frame()
        self.context = session.context()
        self.driver = ReaderDriver(playback: session, clock: clock) { [weak self] in
            self?.refresh()
        }
    }

    public var state: PlayState { frame.state }
    public var wordCount: Int { Int(session.wordCount()) }
    public var omitted: [Omitted] { session.omitted() }

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
    public func nudgeWpm(_ delta: Int32) { apply { $0.nudgeWpm(delta: delta) } }

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
        context = frame.state == .playing ? nil : session.context()
    }
}
