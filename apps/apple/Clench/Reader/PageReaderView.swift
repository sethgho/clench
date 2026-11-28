import ClenchKit
import ClenchUI
import SwiftUI

/// The word beside its page (Duo open board), for a two-screen phone opened
/// flat, an iPad, or any wide window: the current block on the left with
/// the word in the accent, the word reader and map on the right.
struct PageReaderView: View {
    let reader: ReaderModel
    let title: String
    let onClose: () -> Void
    let onSettings: () -> Void

    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var gesture = ReadingGesture()
    @State private var dialShown = false
    @State private var dialHide: Task<Void, Never>?

    var body: some View {
        GeometryReader { geo in
            let half = geo.size.width / 2
            let mapTop: CGFloat = 40
            let mapHeight = max(0, geo.size.height - 80)
            ZStack(alignment: .topLeading) {
                colors.bg.ignoresSafeArea()

                ReadingSurface(reader: reader, gesture: $gesture,
                               label: "Hold to read. While holding, drag up to speed up or down to slow down.",
                               onWpm: showDial, onLift: { hide(after: .zero) })
                    .frame(width: geo.size.width, height: geo.size.height)

                page
                    .frame(width: min(356, half - 100), alignment: .topLeading)
                    .padding(.leading, max(24, min(64, half - 420)))
                    .padding(.top, 90)
                    .opacity(reader.isPlaying ? 0.55 : 1)
                    .animation(.easeOut(duration: 0.2), value: reader.isPlaying)
                    .allowsHitTesting(false)

                WordView(frame: reader.frame, size: preferences.wordSize.points * 1.1)
                    .frame(width: half)
                    .position(x: half + half / 2, y: geo.size.height / 2)
                    .allowsHitTesting(false)

                VStack(spacing: 8) {
                    Text(preferences.touch == .tap ? "Tap to read" : "Hold to read")
                        .font(.plexSans(16, .medium, relativeTo: .callout)).foregroundStyle(colors.ink)
                    Text("Drag up for faster, down for slower")
                        .font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                    Text(speedLine(wpm: reader.frame.wpm, remainingMs: reader.remainingMs))
                        .font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute).padding(.top, 10)
                }
                .frame(width: half, height: geo.size.height - 56, alignment: .bottom)
                .position(x: half + half / 2, y: (geo.size.height - 56) / 2)
                .chrome(!reader.isPlaying)
                .allowsHitTesting(false)

                HStack(spacing: 8) {
                    IconButton(glyph: .chevronDown, label: "Close reader, back to Later", action: onClose)
                    Text(title)
                        .font(.plexSans(14, .semibold, relativeTo: .subheadline))
                        .foregroundStyle(colors.ink)
                        .lineLimit(1)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    IconButton(glyph: .sliders, label: "Settings", action: onSettings)
                }
                .frame(width: max(0, half - 68))
                .position(x: half + 12 + (half - 68) / 2, y: 28 + 22)
                .chrome(!reader.isPlaying)

                SpeedDial(wpm: Int(reader.frame.wpm), height: 200)
                    .opacity(dialShown ? 1 : 0)
                    .offset(x: dialShown || reduceMotion ? 0 : -24)
                    .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.16), value: dialShown)
                    .position(x: half + 75, y: geo.size.height / 2 + 106 + 100)
                    .allowsHitTesting(false)

                EdgeMap(TerrainModel(ticks: reader.ticks, position: reader.position))
                    .frame(width: 44, height: mapHeight)
                    .position(x: geo.size.width - 22, y: mapTop + mapHeight / 2)
                    .accessibilityHidden(true)
                    .allowsHitTesting(false)
            }
        }
        .ignoresSafeArea(edges: .vertical)
        .sensoryFeedback(.selection, trigger: Int(reader.frame.wpm) / 50)
        .onAppear { gesture.config.touch = preferences.touch }
    }

    @ViewBuilder
    private var page: some View {
        VStack(alignment: .leading, spacing: 18) {
            if let section = reader.currentSection?.title {
                Text(section).font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
            }
            if let context = reader.context {
                PageText(context: context)
                    .font(.plexSans(19, relativeTo: .body))
            }
        }
        .accessibilityElement(children: .combine)
    }

    private func showDial() {
        dialShown = true
        hide(after: .milliseconds(600))
    }

    private func hide(after delay: Duration) {
        dialHide?.cancel()
        dialHide = Task {
            if delay > .zero { try? await Task.sleep(for: delay) }
            if !Task.isCancelled { dialShown = false }
        }
    }
}

/// Hands-free reading on a propped-up foldable (Duo stand board): the word
/// and map on the top half, and on the bottom half one large button to
/// pause or resume and a speed slider.
struct StandReaderView: View {
    let reader: ReaderModel
    let title: String

    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        GeometryReader { geo in
            let top = geo.size.height / 2
            ZStack(alignment: .topLeading) {
                colors.bg.ignoresSafeArea()
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).font(.plexSans(14, .semibold, relativeTo: .subheadline)).foregroundStyle(colors.ink)
                    if let s = reader.currentSection?.title {
                        Text(s).font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                    }
                }
                .padding(.leading, 40).padding(.top, 40)

                WordView(frame: reader.frame, size: preferences.wordSize.points * 1.6)
                    .frame(width: geo.size.width)
                    .position(x: geo.size.width / 2, y: top / 2)

                EdgeMap(TerrainModel(ticks: reader.ticks, position: reader.position))
                    .frame(width: 44, height: max(0, top - 72))
                    .position(x: geo.size.width - 22, y: 36 + (top - 72) / 2)
                    .accessibilityHidden(true)

                VStack(spacing: 52) {
                    Button {
                        reader.toggle()
                    } label: {
                        VStack(spacing: 8) {
                            Text(reader.isPlaying ? "Tap to pause" : "Tap to read")
                                .font(.plexSans(20, .medium, relativeTo: .title3)).foregroundStyle(colors.ink)
                            Text("Reading on its own while the phone stands")
                                .font(.plexSans(14, relativeTo: .subheadline)).foregroundStyle(colors.mute)
                        }
                        .frame(maxWidth: .infinity, minHeight: 160)
                        .overlay(RoundedRectangle(cornerRadius: 28).stroke(colors.faint, lineWidth: 1.5))
                        .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(reader.isPlaying ? "Pause reading" : "Start reading")
                    .keyboardShortcut(.space, modifiers: [])

                    SpeedSlider(wpm: Binding(get: { Int(reader.frame.wpm) }, set: { reader.setWpm($0) }))
                }
                .padding(.horizontal, 40)
                .frame(width: geo.size.width)
                .position(x: geo.size.width / 2, y: top + (geo.size.height - top) / 2)
            }
        }
        .ignoresSafeArea(edges: .vertical)
    }
}

/// A horizontal speed scale with the speed above its marker.
struct SpeedSlider: View {
    @Binding var wpm: Int
    @Environment(\.colors) private var colors
    private let scale = SpeedScale()

    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width
            let x = (1 - scale.fraction(wpm)) * w
            ZStack(alignment: .topLeading) {
                HStack(alignment: .firstTextBaseline, spacing: 5) {
                    Text(verbatim: "\(wpm)").font(.plexMonoFixed(28, .medium)).foregroundStyle(colors.ink)
                    Text("wpm").font(.plexMonoFixed(11)).foregroundStyle(colors.mute)
                }
                .fixedSize()
                .position(x: min(max(x, 40), w - 40), y: 16)
                HStack(alignment: .bottom, spacing: 0) {
                    ForEach(scale.ticks.reversed(), id: \.wpm) { t in
                        Rectangle().fill(t.major ? colors.ink : colors.mute)
                            .frame(width: t.major ? 2 : 1, height: t.major ? 18 : 9)
                            .frame(maxWidth: .infinity)
                    }
                }
                .frame(width: w, height: 30, alignment: .bottom)
                .offset(y: 48)
                Rectangle().fill(colors.accent).frame(width: 3, height: 36).offset(x: x - 1.5, y: 42)
                HStack {
                    Text(verbatim: "\(scale.range.lowerBound)")
                    Spacer()
                    Text(verbatim: "\(scale.range.upperBound)")
                }
                .font(.plexMono(11, relativeTo: .caption2)).foregroundStyle(colors.mute)
                .offset(y: 88)
            }
            .contentShape(.rect)
            .gesture(DragGesture(minimumDistance: 0).onChanged { v in
                let f = min(max(v.location.x / max(1, w), 0), 1)
                let raw = Double(scale.range.lowerBound) + f * Double(scale.range.upperBound - scale.range.lowerBound)
                wpm = Int((raw / 5).rounded()) * 5
            })
        }
        .frame(height: 110)
        .accessibilityElement()
        .accessibilityLabel("Reading speed in words per minute")
        .accessibilityValue("\(wpm)")
        .accessibilityAdjustableAction { direction in
            switch direction {
            case .increment: wpm += 25
            case .decrement: wpm -= 25
            @unknown default: break
            }
        }
    }
}
