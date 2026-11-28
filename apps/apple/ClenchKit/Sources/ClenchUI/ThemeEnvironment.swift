import ClenchKit
import SwiftUI

public extension Color {
    init(_ rgb: RGB) {
        self.init(.sRGB, red: Double(rgb.red) / 255, green: Double(rgb.green) / 255, blue: Double(rgb.blue) / 255, opacity: 1)
    }
}

/// A palette resolved to SwiftUI colours.
public struct Colors: Equatable, Sendable {
    public var bg: Color
    public var sheet: Color
    public var ink: Color
    public var mute: Color
    public var faint: Color
    public var accent: Color
    public var onAccent: Color

    public init(_ p: Palette) {
        bg = Color(p.bg)
        sheet = Color(p.sheet)
        ink = Color(p.ink)
        mute = Color(p.mute)
        faint = Color(p.faint)
        accent = Color(p.accent)
        onAccent = Color(p.onAccent)
    }
}

public extension EnvironmentValues {
    /// The theme's colours for the current appearance.
    @Entry var colors = Colors(ThemeFamily.default.light)
    /// The reader's preferences.
    @Entry var preferences = Preferences()
}

/// Resolves the theme from the preferences and the system appearance and
/// puts its colours in the environment. Colours crossfade over 300 ms on a
/// change (a fade either way when Reduce Motion is on).
public struct ThemeRoot: ViewModifier {
    let preferences: Preferences
    @Environment(\.colorScheme) private var systemScheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(preferences: Preferences) {
        self.preferences = preferences
    }

    public func body(content: Content) -> some View {
        let dark = preferences.isDark(systemDark: systemScheme == .dark)
        let palette = preferences.theme.palette(dark: dark)
        content
            .environment(\.colors, Colors(palette))
            .environment(\.preferences, preferences)
            .preferredColorScheme(preferences.appearance == .system ? nil : (dark ? .dark : .light))
            .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.3), value: palette)
    }
}

public extension View {
    /// Applies the clench theme for `preferences`.
    func clenchTheme(_ preferences: Preferences) -> some View {
        modifier(ThemeRoot(preferences: preferences))
    }
}
