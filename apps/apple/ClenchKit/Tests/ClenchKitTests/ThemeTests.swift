import CoreText
import Foundation
import Testing
@testable import ClenchKit
@testable import ClenchUI

@Suite struct ThemeTests {
    @Test func emberMatchesTheDesignTokens() {
        let l = ThemeFamily.ember.light
        #expect([l.bg, l.sheet, l.ink, l.mute, l.faint, l.accent, l.onAccent].map(\.hex)
            == ["#F3F2EE", "#FFFFFF", "#1B1A18", "#6B6862", "#C9C6BE", "#D02519", "#FFFFFF"])
        let d = ThemeFamily.ember.dark
        #expect([d.bg, d.sheet, d.ink, d.mute, d.faint, d.accent, d.onAccent].map(\.hex)
            == ["#0F0F10", "#1B1B1D", "#ECEAE5", "#9A978F", "#3A3936", "#FF5442", "#0F0F10"])
    }

    @Test func everyFamilyHasDistinctLightAndDarkPalettes() {
        #expect(ThemeFamily.allCases.map(\.rawValue) == ["ember", "harbor", "sage", "graphite"])
        #expect(ThemeFamily.harbor.dark.accent.hex == "#F0A63A")
        #expect(ThemeFamily.sage.light.accent.hex == "#B23A12")
        #expect(ThemeFamily.graphite.dark.bg.hex == "#111111")
        for family in ThemeFamily.allCases {
            #expect(family.light != family.dark)
            #expect(family.palette(dark: true) == family.dark)
        }
    }

    @Test func textIsLegibleInEveryPalette() {
        for family in ThemeFamily.allCases {
            for p in [family.light, family.dark] {
                #expect(RGB.contrast(p.ink, p.bg) >= 7, "\(family) ink")
                #expect(RGB.contrast(p.mute, p.bg) >= 4.5, "\(family) mute")
                #expect(RGB.contrast(p.accent, p.bg) >= 3, "\(family) accent")
                #expect(RGB.contrast(p.onAccent, p.accent) >= 3, "\(family) on accent")
            }
        }
    }

    @Test func appearanceResolvesAgainstTheSystem() {
        var prefs = Preferences()
        #expect(prefs.isDark(systemDark: true))
        #expect(!prefs.isDark(systemDark: false))
        prefs.appearance = .light
        #expect(!prefs.isDark(systemDark: true))
        prefs.appearance = .dark
        prefs.theme = .sage
        #expect(prefs.palette(systemDark: false) == ThemeFamily.sage.dark)
    }

    @Test func preferencesSurviveUnknownAndMissingFields() throws {
        let json = #"{"theme": "harbor", "defaultWpm": 450, "touch": "laser"}"#
        let prefs = try JSONDecoder().decode(Preferences.self, from: Data(json.utf8))
        #expect(prefs.theme == .harbor)
        #expect(prefs.defaultWpm == 450)
        #expect(prefs.touch == .either)
        #expect(prefs.wordSize == .medium)
    }

    @Test func preferencesRoundTripThroughDefaults() {
        let suite = "clench-tests-\(UUID().uuidString)"
        defer { UserDefaults().removePersistentDomain(forName: suite) }
        let store = PreferencesStore(suiteName: suite)
        #expect(store.load() == Preferences())
        var prefs = Preferences()
        prefs.theme = .graphite
        prefs.tapeSide = .right
        store.save(prefs)
        #expect(PreferencesStore(suiteName: suite).load() == prefs)
    }

    @Test func fontsAreBundledAndNamed() {
        #expect(ClenchFonts.fileNames == [
            "IBMPlexMono-Medium.ttf", "IBMPlexMono-Regular.ttf", "IBMPlexMono-SemiBold.ttf",
            "IBMPlexSans-Medium.ttf", "IBMPlexSans-Regular.ttf", "IBMPlexSans-SemiBold.ttf",
        ])
        ClenchFonts.register()
        for name in ["IBMPlexSans", "IBMPlexSans-Medm", "IBMPlexSans-SmBld", "IBMPlexMono", "IBMPlexMono-Medm"] {
            let font = CTFontCreateWithName(name as CFString, 12, nil)
            #expect(CTFontCopyPostScriptName(font) as String == name)
        }
    }
}
