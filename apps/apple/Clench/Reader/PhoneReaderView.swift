import ClenchKit
import ClenchUI
import SwiftUI

/// The touch reader (Reader, Speed dial, Terrain scrub boards): press and
/// hold anywhere to read, tap to keep reading, drag up or down for speed and
/// sideways by sentence, and scrub the document map on the right edge.
struct PhoneReaderView: View {
    let reader: ReaderModel
    let title: String
    /// The second line of the hints under the word.
    var hint = "Drag up or down for speed, sideways by sentence"
    var showsIntro = true
    let onClose: () -> Void
    let onSettings: () -> Void

    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var gesture = ReadingGesture()
    @State private var dialShown = false
    @State private var stripShown = false
    @State private var scrubTick: Int?
    @State private var dialHide: Task<Void, Never>?
    @State private var stripHide: Task<Void, Never>?
    @State private var steps = 0
    @State private var opened = false
    @State private var introDone = false

    private var paused: Bool { !reader.isPlaying && scrubTick == nil }

    var body: some View {
        GeometryReader { geo in
            let size = geo.size
            let mid = size.height / 2
            let mapTop: CGFloat = 64
            let mapHeight = max(0, size.height - mapTop - 44)
            ZStack(alignment: .topLeading) {
                colors.bg.ignoresSafeArea()

                surface(size: size)

                WordView(frame: reader.frame, size: preferences.wordSize.points)
                    .frame(width: size.width)
                    .position(x: size.width / 2, y: mid)
                    .opacity(introDone || !showsIntro ? 1 : (opened ? 1 : 0))
                    .animation(reduceMotion ? .easeOut(duration: 0.15) : .easeOut(duration: 0.24).delay(0.38), value: opened)
                    .allowsHitTesting(false)

                if let sentence = reader.sentence {
                    SentenceText(sentence: sentence, size: 15)
                        .frame(width: min(302, size.width - 88))
                        .frame(height: max(0, size.height - mid - 80 - 170), alignment: .top)
                        .clipped()
                        .position(x: size.width / 2, y: mid + 80 + max(0, size.height - mid - 80 - 170) / 2)
                        .chrome(paused)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }

                hints
                    .frame(width: size.width, height: size.height - 84, alignment: .bottom)
                    .position(x: size.width / 2, y: (size.height - 84) / 2)
                    .chrome(paused)
                    .allowsHitTesting(false)

                topBar
                    .frame(width: size.width - 60)
                    .position(x: (size.width - 60) / 2 + 8, y: 56 + 22)
                    .chrome(paused)

                dial(height: min(320, size.height * 0.38))
                    .position(x: preferences.tapeSide == .left ? 75 : size.width - 75 - 52,
                              y: mid + 58 + min(320, size.height * 0.38) / 2)

                if let sentence = reader.sentence {
                    SentenceStrip(sentence: sentence)
                        .frame(width: min(302, size.width - 88), height: 64, alignment: .bottom)
                        .position(x: size.width / 2, y: size.height - 32)
                        .offset(y: stripShown && !reduceMotion ? -28 : 0)
                        .opacity(stripShown ? 1 : 0)
                        .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.16), value: stripShown)
                        .allowsHitTesting(false)
                }

                if let tick = scrubTick {
                    ScrubBubble(section: sectionTitle(forTick: tick), percent: Int((reader.frame.progress * 100).rounded()))
                        .fixedSize()
                        .frame(width: size.width - 50, alignment: .trailing)
                        .position(x: (size.width - 50) / 2, y: mapTop + (CGFloat(tick) + 0.5) / CGFloat(max(1, reader.ticks.count)) * mapHeight)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }

                map(height: mapHeight)
                    .frame(width: 44, height: mapHeight)
                    .position(x: size.width - 22, y: mapTop + mapHeight / 2)

                if showsIntro, !introDone {
                    TerrainMorph(model: terrain, opened: opened,
                                 from: CGRect(x: 24, y: 230, width: size.width - 48, height: 26),
                                 to: CGRect(x: size.width - 44, y: mapTop, width: 44, height: mapHeight))
                        .allowsHitTesting(false)
                }
            }
        }
        .ignoresSafeArea(edges: .vertical)
        .statusBarHiddenIfAvailable(!paused)
        .persistentSystemOverlaysHiddenIfAvailable(!paused)
        .sensoryFeedback(.selection, trigger: Int(reader.frame.wpm) / 50)
        .sensoryFeedback(.selection, trigger: steps)
        .sensoryFeedback(.impact(weight: .light), trigger: scrubHeading)
        .onChange(of: reader.state) { _, state in
            if state != .playing, gesture.latched, !gesture.pressed { gesture.reset() }
        }
        .onAppear {
            gesture.config.touch = preferences.touch
            #if DEBUG
            switch Demo.readerState {
            case "dial": dialShown = true
            case "strip": stripShown = true
            case "scrub":
                let t = Int(Double(reader.ticks.count) * 0.4)
                scrubTick = t
                reader.seek(tick: t)
            case "playing": reader.play()
            default: break
            }
            #endif
            guard showsIntro, !introDone else { return }
            if reduceMotion {
                opened = true
                introDone = true
            } else {
                withAnimation { opened = true }
                Task {
                    try? await Task.sleep(for: .milliseconds(Int(TerrainMorph.duration(count: terrain.marks.count) * 1000)))
                    introDone = true
                }
            }
        }
        .onChange(of: preferences.touch) { _, touch in gesture.config.touch = touch }
    }

    private var terrain: TerrainModel {
        TerrainModel(ticks: reader.ticks, position: reader.position)
    }

    /// The section heading the map is being scrubbed over, for the bump at
    /// each heading.
    private var scrubHeading: Int {
        guard let t = scrubTick, reader.ticks.indices.contains(t), reader.ticks[t].heading else { return -1 }
        return t
    }

    // MARK: - The reading surface

    private func surface(size: CGSize) -> some View {
        ReadingSurface(reader: reader, gesture: $gesture, onWpm: {
            dialShown = true
            hideDial(after: .milliseconds(600))
        }, onStep: {
            steps += 1
            stripShown = true
            stripHide?.cancel()
            stripHide = Task {
                try? await Task.sleep(for: .milliseconds(900))
                if !Task.isCancelled { stripShown = false }
            }
        }, onLift: { hideDial(after: .zero) })
        .frame(width: size.width, height: size.height)
    }

    private func hideDial(after delay: Duration) {
        dialHide?.cancel()
        dialHide = Task {
            if delay > .zero { try? await Task.sleep(for: delay) }
            if !Task.isCancelled { dialShown = false }
        }
    }

    // MARK: - Chrome

    private var topBar: some View {
        HStack(spacing: 8) {
            IconButton(glyph: .chevronDown, label: "Close reader, back to Later", action: onClose)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.plexSans(14, .semibold, relativeTo: .subheadline))
                    .foregroundStyle(colors.ink)
                    .lineLimit(1)
                if let section = reader.currentSection?.title {
                    Text(section)
                        .font(.plexMono(12, relativeTo: .caption))
                        .foregroundStyle(colors.mute)
                        .lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityElement(children: .combine)
            IconButton(glyph: .sliders, label: "Settings", action: onSettings)
        }
    }

    private var hints: some View {
        VStack(spacing: 8) {
            Text(primaryHint)
                .font(.plexSans(16, .medium, relativeTo: .callout))
                .foregroundStyle(colors.ink)
            Text(hint)
                .font(.plexSans(13, relativeTo: .footnote))
                .foregroundStyle(colors.mute)
            Text(speedLine(wpm: reader.frame.wpm, remainingMs: reader.remainingMs))
                .font(.plexMono(12, relativeTo: .caption))
                .foregroundStyle(colors.mute)
                .padding(.top, 10)
        }
        .multilineTextAlignment(.center)
        .padding(.horizontal, 44)
    }

    private var primaryHint: String {
        switch preferences.touch {
        case .either: "Hold to read, or tap to keep going"
        case .hold: "Hold to read"
        case .tap: "Tap to read, tap again to stop"
        }
    }

    private func dial(height: CGFloat) -> some View {
        let fromLeft = preferences.tapeSide == .left
        return SpeedDial(wpm: Int(reader.frame.wpm), height: height, flipped: !fromLeft)
            .opacity(dialShown ? 1 : 0)
            .offset(x: dialShown || reduceMotion ? 0 : (fromLeft ? -24 : 24))
            .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.16), value: dialShown)
            .allowsHitTesting(false)
    }

    // MARK: - The map

    private func map(height: CGFloat) -> some View {
        EdgeMap(terrain)
            .opacity(introDone || !showsIntro ? 1 : 0)
            .contentShape(.rect)
            .gesture(
                DragGesture(minimumDistance: 0, coordinateSpace: .local)
                    .onChanged { value in
                        guard let tick = EdgeMap.tick(at: value.location.y, height: height, count: reader.ticks.count) else { return }
                        if gesture.latched { gesture.reset() }
                        if reader.isPlaying { reader.pause() }
                        if tick != scrubTick {
                            scrubTick = tick
                            reader.seek(tick: tick)
                        }
                    }
                    .onEnded { _ in scrubTick = nil }
            )
            .accessibilityElement()
            .accessibilityLabel("Document map. Drag to move through the document.")
            .accessibilityValue(mapValue)
            .accessibilityAdjustableAction { direction in
                let current = Int(reader.position.section ?? 0)
                switch direction {
                case .increment: reader.seek(section: current + 1)
                case .decrement: reader.seek(section: max(0, current - 1))
                @unknown default: break
                }
            }
    }

    private var mapValue: String {
        let pct = Int((reader.frame.progress * 100).rounded())
        if let s = reader.currentSection?.title { return "\(s), \(pct) percent" }
        return "\(pct) percent"
    }

    private func sectionTitle(forTick tick: Int) -> String {
        guard reader.ticks.indices.contains(tick) else { return title }
        let s = Int(reader.ticks[tick].section)
        return reader.outline.sections.indices.contains(s) ? (reader.outline.sections[s].title ?? title) : title
    }
}

extension View {
    /// Chrome that fades out quickly when reading starts (120 ms) and back
    /// in after a short wait when it stops (200 ms after 250 ms), so a brief
    /// lift shows nothing.
    func chrome(_ visible: Bool) -> some View {
        modifier(ChromeFade(visible: visible))
    }

    @ViewBuilder
    func statusBarHiddenIfAvailable(_ hidden: Bool) -> some View {
        #if os(iOS)
        statusBarHidden(hidden)
        #else
        self
        #endif
    }

    @ViewBuilder
    func persistentSystemOverlaysHiddenIfAvailable(_ hidden: Bool) -> some View {
        #if os(iOS)
        persistentSystemOverlays(hidden ? .hidden : .automatic)
        #else
        self
        #endif
    }
}

private struct ChromeFade: ViewModifier {
    let visible: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .opacity(visible ? 1 : 0)
            .allowsHitTesting(visible)
            .animation(visible ? .easeOut(duration: 0.2).delay(0.25) : .easeOut(duration: 0.12), value: visible)
            .accessibilityHidden(!visible)
    }
}

/// Opening an item: its barcode turns into the edge map tick by tick
/// (450 ms per tick, a 2 ms ripple between ticks).
struct TerrainMorph: View {
    let model: TerrainModel
    let opened: Bool
    let from: CGRect
    let to: CGRect
    @Environment(\.colors) private var colors

    static func duration(count: Int) -> Double {
        0.45 + Double(count) * 0.002 + 0.05
    }

    var body: some View {
        let n = model.marks.count
        ZStack(alignment: .topLeading) {
            ForEach(0..<n, id: \.self) { i in
                let r = rect(i, n: n)
                Rectangle()
                    .fill(model.color(i, colors))
                    .frame(width: r.width, height: r.height)
                    .position(x: r.midX, y: r.midY)
                    .animation(.timingCurve(0.65, 0, 0.35, 1, duration: 0.45).delay(Double(opened ? i : n - i) * 0.002), value: opened)
            }
        }
        .accessibilityHidden(true)
    }

    private func rect(_ i: Int, n: Int) -> CGRect {
        let m = model.marks[i]
        let current = model.state(i) == .current
        if opened {
            let pitch = to.height / CGFloat(max(1, n))
            let w: CGFloat = current ? 34 : (m.heading ? 22 : 5 + 9 * m.weight)
            let h: CGFloat = current ? 3 : (m.heading ? 2 : 1)
            return CGRect(x: to.maxX - w, y: to.minY + (CGFloat(i) + 0.5) * pitch - h / 2, width: w, height: h)
        }
        let pitch = min(3.2, from.width / CGFloat(max(1, n)))
        let h: CGFloat = current || m.heading ? from.height : from.height * (0.3 + 0.45 * m.weight)
        let w: CGFloat = current ? 3 : (m.heading ? 2 : 1)
        return CGRect(x: from.minX + CGFloat(i) * pitch, y: from.maxY - h, width: w, height: h)
    }
}

#Preview("Reader, paused") {
    PreviewReader { reader in
        PhoneReaderView(reader: reader, title: "Nature", showsIntro: false, onClose: {}, onSettings: {})
    }
}
