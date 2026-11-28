import ClenchKit
import SwiftUI

/// A row of mutually exclusive options in a hairline frame; the chosen one
/// is filled with ink.
public struct SegmentedChoice<Value: Hashable>: View {
    let title: String
    let options: [(value: Value, label: String)]
    @Binding var selection: Value
    let height: CGFloat
    @Environment(\.colors) private var colors

    public init(_ title: String, options: [(value: Value, label: String)], selection: Binding<Value>, height: CGFloat = 44) {
        self.title = title
        self.options = options
        _selection = selection
        self.height = height
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.plexSans(14, .semibold, relativeTo: .subheadline))
                .foregroundStyle(colors.ink)
                .accessibilityAddTraits(.isHeader)
            HStack(spacing: 4) {
                ForEach(options, id: \.value) { option in
                    let on = option.value == selection
                    Button {
                        selection = option.value
                    } label: {
                        Text(option.label)
                            .font(.plexSans(height >= 44 ? 15 : 14, .medium, relativeTo: .body))
                            .lineLimit(1)
                            .minimumScaleFactor(0.8)
                            .frame(maxWidth: .infinity, minHeight: height)
                            .foregroundStyle(on ? colors.bg : colors.ink)
                            .background(on ? colors.ink : .clear, in: .rect(cornerRadius: height >= 44 ? 11 : 8))
                            .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                }
            }
            .padding(3)
            .overlay(RoundedRectangle(cornerRadius: height >= 44 ? 14 : 11).stroke(colors.faint, lineWidth: 1))
            .accessibilityElement(children: .contain)
            .accessibilityLabel(title)
        }
    }
}

/// The filled accent button ("Read now").
public struct PrimaryButtonStyle: ButtonStyle {
    var height: CGFloat
    @Environment(\.colors) private var colors

    public init(height: CGFloat = 56) {
        self.height = height
    }

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.plexSans(17, .semibold, relativeTo: .headline))
            .frame(maxWidth: .infinity, minHeight: height)
            .foregroundStyle(colors.onAccent)
            .background(colors.accent, in: .rect(cornerRadius: height > 52 ? 16 : 14))
            .opacity(configuration.isPressed ? 0.85 : 1)
            .contentShape(.rect)
    }
}

/// The outlined button ("Save for later").
public struct SecondaryButtonStyle: ButtonStyle {
    var height: CGFloat
    @Environment(\.colors) private var colors

    public init(height: CGFloat = 56) {
        self.height = height
    }

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.plexSans(17, .medium, relativeTo: .headline))
            .frame(maxWidth: .infinity, minHeight: height)
            .foregroundStyle(colors.ink)
            .overlay(RoundedRectangle(cornerRadius: height > 52 ? 16 : 14).stroke(colors.mute, lineWidth: 1.5))
            .opacity(configuration.isPressed ? 0.7 : 1)
            .contentShape(.rect)
    }
}

/// A key drawn as a small outlined cap ("Space", "Esc").
public struct Keycap: View {
    let label: String
    @Environment(\.colors) private var colors

    public init(_ label: String) {
        self.label = label
    }

    public var body: some View {
        Text(label)
            .font(.plexMono(12, relativeTo: .caption))
            .foregroundStyle(colors.ink)
            .padding(.horizontal, 7)
            .frame(minWidth: 26, minHeight: 26)
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(colors.mute, lineWidth: 1))
    }
}

/// The theme swatch from the settings: each theme's light and dark
/// background with the accent, and its name.
public struct ThemeSwatch: View {
    let family: ThemeFamily
    let selected: Bool
    let compact: Bool
    @Environment(\.colors) private var colors

    public init(_ family: ThemeFamily, selected: Bool, compact: Bool = false) {
        self.family = family
        self.selected = selected
        self.compact = compact
    }

    public var body: some View {
        let l = family.light, d = family.dark
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                sample(bg: l.bg, ink: l.ink, accent: l.accent)
                sample(bg: d.bg, ink: d.ink, accent: d.accent)
            }
            Text(family.label)
                .font(.plexSans(12, .medium, relativeTo: .caption))
                .foregroundStyle(Color(l.ink))
                .frame(maxWidth: .infinity, minHeight: 24)
                .background(Color(l.bg))
        }
        .frame(height: compact ? 64 : 74)
        .clipShape(.rect(cornerRadius: compact ? 12 : 14))
        .overlay(RoundedRectangle(cornerRadius: compact ? 12 : 14).stroke(selected ? colors.ink : colors.faint, lineWidth: 2))
    }

    @ViewBuilder
    private func sample(bg: RGB, ink: RGB, accent: RGB) -> some View {
        Group {
            if compact {
                Rectangle().fill(Color(accent)).frame(width: 14, height: 3)
            } else {
                (Text("so").foregroundStyle(Color(ink)) + Text("l").foregroundStyle(Color(accent)).font(.plexMonoFixed(13, .medium))
                    + Text("itude").foregroundStyle(Color(ink)))
                    .font(.plexMonoFixed(13))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(bg))
    }
}
