import Foundation

/// The reader's settings, shared by the app, its share extension and its
/// widget through the App Group's defaults.
public struct Preferences: Codable, Hashable, Sendable {
    public enum Appearance: String, Codable, CaseIterable, Sendable {
        case system, light, dark

        public var label: String {
            switch self {
            case .system: "System"
            case .light: "Light"
            case .dark: "Dark"
            }
        }
    }

    /// How a touch on the reader starts and stops reading.
    public enum TouchMode: String, Codable, CaseIterable, Sendable {
        /// Reading lasts while the finger is down.
        case hold
        /// A tap starts reading and the next tap stops it.
        case tap
        /// Both: hold to read, or tap to keep reading.
        case either

        public var label: String {
            switch self {
            case .hold: "Hold"
            case .tap: "Tap"
            case .either: "Either"
            }
        }
    }

    public enum WordSize: String, Codable, CaseIterable, Sendable {
        case small, medium, large

        public var label: String {
            switch self {
            case .small: "Small"
            case .medium: "Medium"
            case .large: "Large"
            }
        }

        /// Point size of the reader's word on a phone; the Mac doubles it.
        public var points: Double {
            switch self {
            case .small: 32
            case .medium: 40
            case .large: 48
            }
        }
    }

    public enum Edge: String, Codable, CaseIterable, Sendable {
        case left, right

        public var label: String {
            switch self {
            case .left: "Left edge"
            case .right: "Right edge"
            }
        }
    }

    /// Where Read now opens on a Mac.
    public enum MacReader: String, Codable, CaseIterable, Sendable {
        case fullScreen, window

        public var label: String {
            switch self {
            case .fullScreen: "Full screen"
            case .window: "A window"
            }
        }
    }

    /// What the space bar does on a Mac.
    public enum SpaceBar: String, Codable, CaseIterable, Sendable {
        case hold, toggle

        public var label: String {
            switch self {
            case .hold: "Hold to read"
            case .toggle: "Tap to start and stop"
            }
        }
    }

    public var theme: ThemeFamily = .default
    public var appearance: Appearance = .system
    /// Words per minute for items not read before. The engine clamps it.
    public var defaultWpm: Int = 320
    public var touch: TouchMode = .either
    public var wordSize: WordSize = .medium
    public var tapeSide: Edge = .left
    public var macReader: MacReader = .fullScreen
    public var spaceBar: SpaceBar = .hold

    public init() {}

    /// Whether to draw dark, given the system's current scheme.
    public func isDark(systemDark: Bool) -> Bool {
        switch appearance {
        case .system: systemDark
        case .light: false
        case .dark: true
        }
    }

    public func palette(systemDark: Bool) -> Palette {
        theme.palette(dark: isDark(systemDark: systemDark))
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let d = Preferences()
        theme = (try? c.decodeIfPresent(ThemeFamily.self, forKey: .theme)) ?? d.theme
        appearance = (try? c.decodeIfPresent(Appearance.self, forKey: .appearance)) ?? d.appearance
        defaultWpm = (try? c.decodeIfPresent(Int.self, forKey: .defaultWpm)) ?? d.defaultWpm
        touch = (try? c.decodeIfPresent(TouchMode.self, forKey: .touch)) ?? d.touch
        wordSize = (try? c.decodeIfPresent(WordSize.self, forKey: .wordSize)) ?? d.wordSize
        tapeSide = (try? c.decodeIfPresent(Edge.self, forKey: .tapeSide)) ?? d.tapeSide
        macReader = (try? c.decodeIfPresent(MacReader.self, forKey: .macReader)) ?? d.macReader
        spaceBar = (try? c.decodeIfPresent(SpaceBar.self, forKey: .spaceBar)) ?? d.spaceBar
    }
}

/// Reads and writes ``Preferences`` in the App Group's defaults (or the
/// standard defaults when there is no App Group).
public struct PreferencesStore: Sendable {
    public static let key = "preferences"
    private let suiteName: String?

    public init(suiteName: String? = AppGroup.identifier) {
        self.suiteName = suiteName
    }

    private var defaults: UserDefaults {
        suiteName.flatMap(UserDefaults.init(suiteName:)) ?? .standard
    }

    public func load() -> Preferences {
        guard let data = defaults.data(forKey: Self.key),
              let prefs = try? JSONDecoder().decode(Preferences.self, from: data)
        else { return Preferences() }
        return prefs
    }

    public func save(_ prefs: Preferences) {
        if let data = try? JSONEncoder().encode(prefs) {
            defaults.set(data, forKey: Self.key)
        }
    }
}
