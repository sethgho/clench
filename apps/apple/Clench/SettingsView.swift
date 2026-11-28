import ClenchKit
import ClenchUI
import SwiftUI

/// Settings (Settings and themes board; Mac settings board on macOS).
struct SettingsView: View {
    var onClose: (() -> Void)?

    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors

    var body: some View {
        @Bindable var model = model
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                #if os(iOS)
                if let onClose {
                    IconButton(glyph: .chevronLeft, label: "Back to reader", action: onClose)
                        .padding(.leading, -12)
                        .frame(height: 44)
                }
                Text("Settings")
                    .font(.plexSans(34, .semibold, relativeTo: .largeTitle))
                    .tracking(-0.7)
                    .foregroundStyle(colors.ink)
                    .padding(.top, 12)
                    .padding(.bottom, 18)
                    .accessibilityAddTraits(.isHeader)
                #else
                Text("Settings")
                    .font(.plexSans(22, .semibold, relativeTo: .title2))
                    .foregroundStyle(colors.ink)
                    .padding(.bottom, 24)
                #endif

                VStack(alignment: .leading, spacing: controlSpacing) {
                    #if os(macOS)
                    VStack(alignment: .leading, spacing: 8) {
                        SegmentedChoice("Read now opens in", options: Preferences.MacReader.allCases.map { ($0, $0.label) },
                                        selection: $model.preferences.macReader, height: 36)
                        Text("Full screen hides everything but the word and the map.")
                            .font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                    }
                    SegmentedChoice("Space bar", options: Preferences.SpaceBar.allCases.map { ($0, $0.label) },
                                    selection: $model.preferences.spaceBar, height: 36)
                    #endif
                    SegmentedChoice("Appearance", options: Preferences.Appearance.allCases.map { ($0, $0.label) },
                                    selection: $model.preferences.appearance, height: segmentHeight)
                    themes
                    #if os(iOS)
                    SegmentedChoice("Touch to read", options: Preferences.TouchMode.allCases.map { ($0, $0.label) },
                                    selection: $model.preferences.touch)
                    #endif
                    SegmentedChoice("Word size", options: Preferences.WordSize.allCases.map { ($0, $0.label) },
                                    selection: $model.preferences.wordSize, height: segmentHeight)
                    #if os(iOS)
                    SegmentedChoice("Speed tape slides in from", options: Preferences.Edge.allCases.map { ($0, $0.label) },
                                    selection: $model.preferences.tapeSide)
                    #endif
                    defaultSpeed($model.preferences.defaultWpm)
                }
            }
            .padding(.horizontal, horizontalPadding)
            .padding(.top, topPadding)
            .padding(.bottom, 36)
            .frame(maxWidth: 560, alignment: .leading)
        }
        .background(colors.bg.ignoresSafeArea())
    }

    #if os(macOS)
    private let segmentHeight: CGFloat = 36
    private let controlSpacing: CGFloat = 24
    private let horizontalPadding: CGFloat = 36
    private let topPadding: CGFloat = 32
    #else
    private let segmentHeight: CGFloat = 44
    private let controlSpacing: CGFloat = 18
    private let horizontalPadding: CGFloat = 24
    private let topPadding: CGFloat = 20
    #endif

    private var themes: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Theme")
                .font(.plexSans(14, .semibold, relativeTo: .subheadline))
                .foregroundStyle(colors.ink)
                .accessibilityAddTraits(.isHeader)
            #if os(macOS)
            let columns = Array(repeating: GridItem(.flexible(), spacing: 10), count: 4)
            #else
            let columns = Array(repeating: GridItem(.flexible(), spacing: 10), count: 2)
            #endif
            LazyVGrid(columns: columns, spacing: 10) {
                ForEach(ThemeFamily.allCases) { family in
                    let on = family == model.preferences.theme
                    Button {
                        model.preferences.theme = family
                    } label: {
                        #if os(macOS)
                        ThemeSwatch(family, selected: on, compact: true)
                        #else
                        ThemeSwatch(family, selected: on)
                        #endif
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("\(family.label) theme")
                    .accessibilityAddTraits(on ? .isSelected : [])
                }
            }
        }
    }

    private func defaultSpeed(_ wpm: Binding<Int>) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Starting speed")
                .font(.plexSans(14, .semibold, relativeTo: .subheadline))
                .foregroundStyle(colors.ink)
            HStack(spacing: 4) {
                stepButton("−", label: "Slower") { wpm.wrappedValue = max(100, wpm.wrappedValue - 10) }
                Text(verbatim: "\(wpm.wrappedValue) wpm")
                    .font(.plexMono(15, .medium, relativeTo: .body))
                    .foregroundStyle(colors.ink)
                    .monospacedDigit()
                    .frame(maxWidth: .infinity)
                stepButton("+", label: "Faster") { wpm.wrappedValue = min(1500, wpm.wrappedValue + 10) }
            }
            .padding(3)
            .overlay(RoundedRectangle(cornerRadius: 14).stroke(colors.faint, lineWidth: 1))
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Starting speed")
            .accessibilityValue("\(wpm.wrappedValue) words per minute")
            .accessibilityAdjustableAction { direction in
                switch direction {
                case .increment: wpm.wrappedValue = min(1500, wpm.wrappedValue + 10)
                case .decrement: wpm.wrappedValue = max(100, wpm.wrappedValue - 10)
                @unknown default: break
                }
            }
            Text("For things you haven’t started. Each item keeps the speed you last read it at.")
                .font(.plexSans(13, relativeTo: .footnote))
                .foregroundStyle(colors.mute)
        }
    }

    private func stepButton(_ symbol: String, label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(symbol)
                .font(.plexMono(20, relativeTo: .title3))
                .foregroundStyle(colors.ink)
                .frame(width: 56, height: segmentHeight)
                .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .buttonRepeatBehavior(.enabled)
        .accessibilityLabel(label)
    }
}
