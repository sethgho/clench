import Foundation

/// An sRGB colour as three 8-bit channels.
public struct RGB: Hashable, Sendable {
    public var red: UInt8
    public var green: UInt8
    public var blue: UInt8

    /// A colour from `0xRRGGBB`.
    public init(_ hex: UInt32) {
        red = UInt8((hex >> 16) & 0xFF)
        green = UInt8((hex >> 8) & 0xFF)
        blue = UInt8(hex & 0xFF)
    }

    /// `#RRGGBB`.
    public var hex: String {
        String(format: "#%02X%02X%02X", red, green, blue)
    }

    /// Relative luminance (WCAG 2.x).
    public var luminance: Double {
        func channel(_ c: UInt8) -> Double {
            let v = Double(c) / 255
            return v <= 0.03928 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
    }

    /// WCAG contrast ratio between two colours, 1 to 21.
    public static func contrast(_ a: RGB, _ b: RGB) -> Double {
        let (hi, lo) = a.luminance > b.luminance ? (a, b) : (b, a)
        return (hi.luminance + 0.05) / (lo.luminance + 0.05)
    }
}

/// The colour roles every screen draws with.
public struct Palette: Hashable, Sendable {
    /// Screen background.
    public var bg: RGB
    /// Raised surfaces: sheets, panels.
    public var sheet: RGB
    /// Primary text and marks.
    public var ink: RGB
    /// Secondary text and unread marks.
    public var mute: RGB
    /// Hairlines, tick marks already read, disabled marks.
    public var faint: RGB
    /// The pivot letter, the current position, primary buttons.
    public var accent: RGB
    /// Text on an accent fill.
    public var onAccent: RGB
}

/// A colour family, each with a light and a dark palette.
public enum ThemeFamily: String, CaseIterable, Codable, Sendable, Identifiable {
    case ember, harbor, sage, graphite

    public var id: String { rawValue }

    public var label: String {
        switch self {
        case .ember: "Ember"
        case .harbor: "Harbor"
        case .sage: "Sage"
        case .graphite: "Graphite"
        }
    }

    public static let `default` = ThemeFamily.ember

    public var light: Palette {
        switch self {
        case .ember:
            Palette(bg: RGB(0xF3F2EE), sheet: RGB(0xFFFFFF), ink: RGB(0x1B1A18), mute: RGB(0x6B6862),
                    faint: RGB(0xC9C6BE), accent: RGB(0xD02519), onAccent: RGB(0xFFFFFF))
        case .harbor:
            Palette(bg: RGB(0xE9ECF2), sheet: RGB(0xF6F8FB), ink: RGB(0x18202E), mute: RGB(0x56607A),
                    faint: RGB(0xBCC4D4), accent: RGB(0x9A5B00), onAccent: RGB(0xFFFFFF))
        case .sage:
            Palette(bg: RGB(0xE4E9E0), sheet: RGB(0xF2F5EF), ink: RGB(0x1D2A21), mute: RGB(0x566559),
                    faint: RGB(0xB5C0B3), accent: RGB(0xB23A12), onAccent: RGB(0xFFFFFF))
        case .graphite:
            Palette(bg: RGB(0xF0F0F0), sheet: RGB(0xFFFFFF), ink: RGB(0x151515), mute: RGB(0x636363),
                    faint: RGB(0xC6C6C6), accent: RGB(0x0B5FC0), onAccent: RGB(0xFFFFFF))
        }
    }

    public var dark: Palette {
        switch self {
        case .ember:
            Palette(bg: RGB(0x0F0F10), sheet: RGB(0x1B1B1D), ink: RGB(0xECEAE5), mute: RGB(0x9A978F),
                    faint: RGB(0x3A3936), accent: RGB(0xFF5442), onAccent: RGB(0x0F0F10))
        case .harbor:
            Palette(bg: RGB(0x141A26), sheet: RGB(0x1D2535), ink: RGB(0xDCE1EC), mute: RGB(0x8E98AB),
                    faint: RGB(0x34405A), accent: RGB(0xF0A63A), onAccent: RGB(0x141A26))
        case .sage:
            Palette(bg: RGB(0x131A15), sheet: RGB(0x1C261F), ink: RGB(0xDCE5DC), mute: RGB(0x8FA093),
                    faint: RGB(0x2F3D33), accent: RGB(0xF0794A), onAccent: RGB(0x131A15))
        case .graphite:
            Palette(bg: RGB(0x111111), sheet: RGB(0x1C1C1C), ink: RGB(0xE8E8E8), mute: RGB(0x989898),
                    faint: RGB(0x383838), accent: RGB(0x62A8FF), onAccent: RGB(0x111111))
        }
    }

    public func palette(dark: Bool) -> Palette {
        dark ? self.dark : light
    }
}

/// Colours that do not change with the theme.
public enum FixedColor {
    /// The destructive swipe action's fill.
    public static let delete = RGB(0xB3261E)
    public static let onDelete = RGB(0xFFFFFF)
    /// The dimmed page behind a share sheet.
    public static let backdrop = RGB(0x55534F)
}
