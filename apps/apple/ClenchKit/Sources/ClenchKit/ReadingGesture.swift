import Foundation

/// The reader's touch model, as a pure state machine: feed it touches and
/// apply the effects it returns. It knows nothing about words or timing;
/// those come from the engine.
///
/// * Press and hold plays; lifting pauses.
/// * A quick tap (no drag, under ``Config/tapDuration``) latches playing; the
///   next tap pauses.
/// * Once the finger moves ``Config/axisLock`` points, the drag locks to an
///   axis. Vertical: speed, ``Config/wpmPerPoint`` per point upward, snapped
///   to ``Config/wpmStep``. Horizontal: one sentence per
///   ``Config/sentenceStep`` points.
/// * A drag during a latched read keeps it playing when lifted.
public struct ReadingGesture: Sendable {
    public struct Config: Sendable, Equatable {
        public var touch: Preferences.TouchMode = .either
        public var tapDuration: TimeInterval = 0.22
        public var axisLock: Double = 10
        public var sentenceStep: Double = 36
        public var wpmPerPoint: Double = 2
        public var wpmStep: Int = 5
        /// The engine's speed range (its `Pacing` defaults).
        public var wpmRange: ClosedRange<Int> = 100...1500

        public init() {}
    }

    public enum Axis: Sendable, Equatable { case horizontal, vertical }

    public enum Effect: Sendable, Equatable {
        case play
        case pause
        /// Set the speed and show the speed dial.
        case setWpm(Int)
        /// Move by sentences: +1 forward, -1 back. Show the sentence strip.
        case stepSentence(Int)
    }

    public var config: Config
    public private(set) var pressed = false
    public private(set) var axis: Axis?
    /// Reading continues after the finger lifts.
    public private(set) var latched = false

    private var start = (x: 0.0, y: 0.0)
    private var anchorX = 0.0
    private var startWpm = 0
    private var downAt: TimeInterval = 0
    private var wasLatched = false
    private var startedPlaying = false

    public init(config: Config = Config()) {
        self.config = config
    }

    /// A finger came down at (`x`, `y`) at `time` (seconds). `playing` and
    /// `wpm` describe the session now.
    public mutating func down(x: Double, y: Double, time: TimeInterval, playing: Bool, wpm: Int) -> [Effect] {
        pressed = true
        axis = nil
        start = (x, y)
        anchorX = x
        startWpm = wpm
        downAt = time
        wasLatched = latched && playing
        if !playing { latched = false }
        startedPlaying = false
        guard !wasLatched, config.touch != .tap else { return [] }
        startedPlaying = true
        return [.play]
    }

    /// The finger moved to (`x`, `y`). Up is negative `y`, as on screen.
    public mutating func move(x: Double, y: Double) -> [Effect] {
        guard pressed else { return [] }
        let dx = x - start.x
        let dy = start.y - y
        if axis == nil {
            guard max(abs(dx), abs(dy)) >= config.axisLock else { return [] }
            axis = abs(dx) > abs(dy) ? .horizontal : .vertical
        }
        switch axis {
        case .vertical:
            let raw = (Double(startWpm) + dy * config.wpmPerPoint) / Double(config.wpmStep)
            let snapped = Int(raw.rounded()) * config.wpmStep
            return [.setWpm(min(max(snapped, config.wpmRange.lowerBound), config.wpmRange.upperBound))]
        case .horizontal:
            let d = x - anchorX
            guard abs(d) >= config.sentenceStep else { return [] }
            anchorX = x
            return [.stepSentence(d > 0 ? 1 : -1)]
        case nil:
            return []
        }
    }

    /// The finger lifted at `time`.
    public mutating func up(time: TimeInterval) -> [Effect] {
        guard pressed else { return [] }
        pressed = false
        let tap = axis == nil && time - downAt < config.tapDuration
        if wasLatched {
            if axis == nil {
                latched = false
                return [.pause]
            }
            return []
        }
        switch config.touch {
        case .either where tap:
            latched = true
            return []
        case .tap where tap:
            latched = true
            return [.play]
        case .tap:
            return []
        case .hold, .either:
            return startedPlaying ? [.pause] : []
        }
    }

    /// The touch was cancelled by the system: stop reading.
    public mutating func cancel() -> [Effect] {
        guard pressed else { return [] }
        pressed = false
        latched = false
        return wasLatched || startedPlaying ? [.pause] : []
    }

    /// Reading stopped for another reason (finished, a key, the app went
    /// away): forget the latch.
    public mutating func reset() {
        pressed = false
        latched = false
        axis = nil
    }
}

/// The speed dial's scale: ticks every 50 wpm over the engine's range, a
/// major tick every 100, and where a speed sits on it.
public struct SpeedScale: Sendable, Equatable {
    public struct Tick: Sendable, Equatable {
        public var wpm: Int
        public var major: Bool
    }

    public var range: ClosedRange<Int>
    public var step: Int

    public init(range: ClosedRange<Int> = 100...1500, step: Int = 50) {
        self.range = range
        self.step = step
    }

    /// Ticks from fastest (top) to slowest (bottom).
    public var ticks: [Tick] {
        stride(from: range.upperBound, through: range.lowerBound, by: -step).map {
            Tick(wpm: $0, major: $0 % (step * 2) == 0)
        }
    }

    /// Where `wpm` sits from the top tick (0) to the bottom tick (1).
    public func fraction(_ wpm: Int) -> Double {
        let span = Double(range.upperBound - range.lowerBound)
        guard span > 0 else { return 0 }
        let clamped = min(max(wpm, range.lowerBound), range.upperBound)
        return Double(range.upperBound - clamped) / span
    }
}
