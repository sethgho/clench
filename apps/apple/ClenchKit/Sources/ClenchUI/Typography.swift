import ClenchKit
import CoreText
import SwiftUI

/// IBM Plex Sans and Mono, bundled with the package.
public enum ClenchFonts {
    public enum Weight: Sendable {
        case regular, medium, semibold
    }

    /// PostScript names of the bundled faces.
    static func postScriptName(mono: Bool, weight: Weight) -> String {
        let family = mono ? "IBMPlexMono" : "IBMPlexSans"
        switch weight {
        case .regular: return family
        case .medium: return family + "-Medm"
        case .semibold: return family + "-SmBld"
        }
    }

    nonisolated(unsafe) private static var registered = false
    private static let lock = NSLock()

    /// Registers the bundled fonts for this process. Safe to call more than
    /// once; every entry point (app, extension, widget) calls it at start.
    public static func register() {
        lock.lock()
        defer { lock.unlock() }
        guard !registered else { return }
        registered = true
        let urls = Bundle.module.urls(forResourcesWithExtension: "ttf", subdirectory: "Fonts") ?? []
        CTFontManagerRegisterFontURLs(urls as CFArray, .process, true, nil)
    }

    /// The bundled font files, for tests.
    public static var fileNames: [String] {
        (Bundle.module.urls(forResourcesWithExtension: "ttf", subdirectory: "Fonts") ?? [])
            .map(\.lastPathComponent).sorted()
    }
}

public extension Font {
    /// Plex Sans at `size` points, scaled with Dynamic Type like `style`.
    static func plexSans(_ size: CGFloat, _ weight: ClenchFonts.Weight = .regular, relativeTo style: Font.TextStyle = .body) -> Font {
        .custom(ClenchFonts.postScriptName(mono: false, weight: weight), size: size, relativeTo: style)
    }

    /// Plex Mono at `size` points, scaled with Dynamic Type like `style`.
    static func plexMono(_ size: CGFloat, _ weight: ClenchFonts.Weight = .regular, relativeTo style: Font.TextStyle = .body) -> Font {
        .custom(ClenchFonts.postScriptName(mono: true, weight: weight), size: size, relativeTo: style)
    }

    /// Plex Mono at a fixed size that ignores Dynamic Type: the reader's word,
    /// whose size is its own setting.
    static func plexMonoFixed(_ size: CGFloat, _ weight: ClenchFonts.Weight = .regular) -> Font {
        .custom(ClenchFonts.postScriptName(mono: true, weight: weight), fixedSize: size)
    }
}
