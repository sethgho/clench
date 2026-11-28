import Foundation

/// What the library shows about an item without opening it: metadata, the
/// engine's outline and document map, and where reading stopped. Everything
/// here is copied from the engine (``Session/outline()``,
/// ``Session/terrain(maxTicks:)``, ``Session/position(maxTicks:)``); times
/// are kept speed-independent and turned into minutes by the engine's
/// ``readingMs(timing:wpm:)``.
public struct ItemSummary: Codable, Hashable, Sendable {
    /// Number of ticks in a stored document map.
    public static let tickCount: UInt32 = 64

    /// ``ClenchFFI/Timing`` in a storable form.
    public struct StoredTiming: Codable, Hashable, Sendable {
        public var beats: UInt64
        public var fixedMs: UInt64

        public init(beats: UInt64, fixedMs: UInt64) {
            self.beats = beats
            self.fixedMs = fixedMs
        }

        public init(_ t: Timing) {
            self.init(beats: t.beats, fixedMs: t.fixedMs)
        }

        public var timing: Timing { Timing(beats: beats, fixedMs: fixedMs) }

        public static let zero = StoredTiming(beats: 0, fixedMs: 0)

        /// Milliseconds at `wpm`, by the engine's estimate.
        public func ms(at wpm: Int) -> UInt64 {
            readingMs(timing: timing, wpm: UInt32(clamping: wpm))
        }
    }

    public struct Section: Codable, Hashable, Sendable {
        public var title: String?
        public var tokenStart: Int
        public var tokenEnd: Int
        public var words: Int
        public var timing: StoredTiming
    }

    public struct Tick: Codable, Hashable, Sendable {
        public var heading: Bool
        public var section: Int
        public var tokenStart: Int
        public var tokenEnd: Int
        /// 0 to 1000, relative to the largest non-heading tick.
        public var weight: Int

        public init(heading: Bool, section: Int, tokenStart: Int, tokenEnd: Int, weight: Int) {
            self.heading = heading
            self.section = section
            self.tokenStart = tokenStart
            self.tokenEnd = tokenEnd
            self.weight = weight
        }

        public init(_ t: TerrainTick) {
            self.init(heading: t.heading, section: Int(t.section), tokenStart: Int(t.tokenStart),
                      tokenEnd: Int(t.tokenEnd), weight: Int(t.weight))
        }
    }

    /// Where reading stopped, as the engine placed it.
    public struct Mark: Codable, Hashable, Sendable {
        public var token: Int
        /// Current section; nil when finished.
        public var section: Int?
        /// Current tick in ``ItemSummary/ticks``; nil when finished.
        public var tick: Int?
        public var remaining: StoredTiming
        public var sectionRemaining: StoredTiming

        public init(token: Int, section: Int?, tick: Int?, remaining: StoredTiming, sectionRemaining: StoredTiming) {
            self.token = token
            self.section = section
            self.tick = tick
            self.remaining = remaining
            self.sectionRemaining = sectionRemaining
        }

        public init(_ p: ReadingPosition) {
            self.init(token: Int(p.token), section: p.section.map(Int.init), tick: p.tick.map(Int.init),
                      remaining: StoredTiming(p.remaining), sectionRemaining: StoredTiming(p.sectionRemaining))
        }
    }

    public var title: String
    public var byline: String?
    public var siteName: String?
    public var words: Int
    public var timing: StoredTiming
    public var sections: [Section]
    public var ticks: [Tick]
    public var mark: Mark

    public init(title: String, byline: String?, siteName: String?, words: Int, timing: StoredTiming,
                sections: [Section], ticks: [Tick], mark: Mark) {
        self.title = title
        self.byline = byline
        self.siteName = siteName
        self.words = words
        self.timing = timing
        self.sections = sections
        self.ticks = ticks
        self.mark = mark
    }

    /// A summary of `session`, which reads `document`, at its current
    /// position.
    public init(document: Document, session: Session) {
        let outline = session.outline()
        title = document.title.trimmingCharacters(in: .whitespacesAndNewlines)
        byline = document.byline.flatMap(Self.nonEmpty)
        siteName = document.siteName.flatMap(Self.nonEmpty)
        words = Int(outline.words)
        timing = StoredTiming(outline.timing)
        sections = outline.sections.map {
            Section(title: $0.title.flatMap(Self.nonEmpty), tokenStart: Int($0.tokenStart),
                    tokenEnd: Int($0.tokenEnd), words: Int($0.words), timing: StoredTiming($0.timing))
        }
        ticks = session.terrain(maxTicks: Self.tickCount).map(Tick.init)
        mark = Mark(session.position(maxTicks: Self.tickCount))
    }

    /// The same summary with the position of `session` (which reads the
    /// same document).
    public func marked(by session: Session) -> ItemSummary {
        var copy = self
        copy.mark = Mark(session.position(maxTicks: Self.tickCount))
        return copy
    }

    /// Whether reading has moved past the start.
    public var started: Bool { mark.token > 0 || mark.tick == nil }

    /// The current section's title, if it has one.
    public var currentSectionTitle: String? {
        mark.section.flatMap { sections.indices.contains($0) ? sections[$0].title : nil }
    }

    /// A line like "Ralph Waldo Emerson · gutenberg.org".
    public func attribution(host: String?) -> String? {
        let parts = [byline, siteName ?? host].compactMap { $0 }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    private static func nonEmpty(_ s: String) -> String? {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        return t.isEmpty ? nil : t
    }
}

/// Minutes for display.
public enum ReadingTime {
    /// Whole minutes for `ms`, rounded to the nearest and never below one
    /// for a non-zero time.
    public static func minutes(_ ms: UInt64) -> Int {
        guard ms > 0 else { return 0 }
        return max(1, Int((Double(ms) / 60_000).rounded()))
    }

    /// "42 min".
    public static func label(_ ms: UInt64) -> String {
        "\(minutes(ms)) min"
    }
}
