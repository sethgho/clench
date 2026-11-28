#if os(macOS)
import AppKit
import ClenchKit
import ClenchUI
import SwiftUI

/// The Mac reader (Mac full-screen reader, speed tape and map hover
/// boards): hold Space or the mouse button to read, scroll or the up and
/// down arrows for speed, left and right by sentence, point at the map to
/// see a section and click to go there, Esc to leave.
struct MacReaderView: View {
    let reader: ReaderModel
    let title: String
    let onClose: () -> Void

    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @FocusState private var focused: Bool
    @State private var dialShown = false
    @State private var stripShown = false
    @State private var hoverTick: Int?
    @State private var dialHide: Task<Void, Never>?
    @State private var stripHide: Task<Void, Never>?
    @State private var mouseDown: (y: CGFloat, wpm: Int, dragging: Bool)?
    @State private var wheelMonitor: Any?

    private var paused: Bool { !reader.isPlaying }

    var body: some View {
        GeometryReader { geo in
            let size = geo.size
            let mid = size.height / 2
            let mapHeight = max(0, size.height - 112)
            ZStack(alignment: .topLeading) {
                colors.bg.ignoresSafeArea()

                Rectangle().fill(.clear).contentShape(.rect)
                    .gesture(mouseGesture)
                    .accessibilityElement()
                    .accessibilityLabel("Hold Space or the mouse button to read. Scroll or use the up and down arrows to change speed.")
                    .accessibilityAction { reader.toggle() }

                WordView(frame: reader.frame, size: preferences.wordSize.points * 2)
                    .frame(width: size.width)
                    .position(x: size.width / 2, y: mid)
                    .allowsHitTesting(false)

                if paused, let sentence = reader.sentence {
                    SentenceText(sentence: sentence, size: 18)
                        .frame(width: min(620, size.width - 200))
                        .frame(height: max(0, size.height / 2 - 112 - 150), alignment: .top)
                        .clipped()
                        .position(x: size.width / 2, y: mid + 112 + max(0, size.height / 2 - 112 - 150) / 2)
                        .allowsHitTesting(false)
                        .transition(.opacity)
                }

                if paused {
                    hints
                        .frame(width: size.width, height: size.height - 64, alignment: .bottom)
                        .position(x: size.width / 2, y: (size.height - 64) / 2)
                        .allowsHitTesting(false)
                        .transition(.opacity)
                    VStack(alignment: .leading, spacing: 3) {
                        Text(title).font(.plexSans(15, .semibold, relativeTo: .headline)).foregroundStyle(colors.ink)
                        if let s = reader.currentSection?.title {
                            Text(s).font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                        }
                    }
                    .padding(.leading, 36).padding(.top, 32)
                    .transition(.opacity)
                }

                SpeedDial(wpm: Int(reader.frame.wpm), height: 300, majorWidth: 22)
                    .padding(.leading, 10)
                    .opacity(dialShown ? 1 : 0)
                    .offset(x: dialShown || reduceMotion ? 0 : -24)
                    .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.16), value: dialShown)
                    .position(x: 85, y: size.height - 72 - 150)
                    .allowsHitTesting(false)

                if let sentence = reader.sentence {
                    SentenceStrip(sentence: sentence, width: 280)
                        .frame(width: 320, height: 64, alignment: .bottom)
                        .position(x: size.width / 2, y: size.height - 32)
                        .offset(y: stripShown && !reduceMotion ? -28 : 0)
                        .opacity(stripShown ? 1 : 0)
                        .animation(.easeOut(duration: reduceMotion ? 0.15 : 0.16), value: stripShown)
                        .allowsHitTesting(false)
                }

                map(height: mapHeight)
                    .frame(width: 64, height: mapHeight)
                    .position(x: size.width - 32, y: 56 + mapHeight / 2)

                if let tick = hoverTick {
                    ScrubBubble(section: sectionTitle(tick), percent: percent(tick), fontSize: 13)
                        .fixedSize()
                        .frame(width: size.width - 72, alignment: .trailing)
                        .position(x: (size.width - 72) / 2, y: 56 + (CGFloat(tick) + 0.5) / CGFloat(max(1, reader.ticks.count)) * mapHeight)
                        .allowsHitTesting(false)
                }
            }
            .animation(paused ? .easeOut(duration: 0.2).delay(0.25) : .easeOut(duration: 0.12), value: paused)
        }
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat, .up], action: key)
        .onAppear {
            focused = true
            installWheel()
        }
        .onDisappear { removeWheel() }
        .onChange(of: reader.isPlaying) { _, playing in
            if playing { NSCursor.setHiddenUntilMouseMoves(true) }
        }
    }

    private var hints: some View {
        VStack(spacing: 12) {
            HStack(spacing: 10) {
                Text("Hold"); Keycap("Space"); Text("to read")
            }
            .font(.plexSans(17, .medium, relativeTo: .headline))
            .foregroundStyle(colors.ink)
            HStack(spacing: 18) {
                Text("Scroll up for faster, down for slower")
                HStack(spacing: 6) { Keycap("←"); Keycap("→"); Text("sentence") }
                HStack(spacing: 6) { Keycap("Esc"); Text("leave") }
            }
            .font(.plexSans(13, relativeTo: .footnote))
            .foregroundStyle(colors.mute)
            Text(speedLine(wpm: reader.frame.wpm, remainingMs: reader.remainingMs))
                .font(.plexMono(12, relativeTo: .caption))
                .foregroundStyle(colors.mute)
        }
    }

    // MARK: - Input

    private func key(_ press: KeyPress) -> KeyPress.Result {
        let down = press.phase == .down
        switch press.key {
        case .space:
            switch preferences.spaceBar {
            case .hold:
                if down, !reader.isPlaying { reader.play() }
                if press.phase == .up { reader.pause() }
            case .toggle:
                if down { reader.toggle() }
            }
            return .handled
        case .upArrow where down || press.phase == .repeat:
            nudge(10); return .handled
        case .downArrow where down || press.phase == .repeat:
            nudge(-10); return .handled
        case .leftArrow where down || press.phase == .repeat:
            step(-1); return .handled
        case .rightArrow where down || press.phase == .repeat:
            step(1); return .handled
        case .escape where down:
            reader.pause()
            onClose()
            return .handled
        default:
            return .ignored
        }
    }

    private var mouseGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                guard let start = mouseDown else {
                    mouseDown = (value.startLocation.y, Int(reader.frame.wpm), false)
                    reader.play()
                    return
                }
                let dy = start.y - value.location.y
                guard start.dragging || abs(dy) >= 10 else { return }
                mouseDown?.dragging = true
                let wpm = Int(((Double(start.wpm) + dy * 2) / 5).rounded()) * 5
                reader.setWpm(min(max(wpm, 100), 1500))
                showDial(for: .milliseconds(900))
            }
            .onEnded { _ in
                mouseDown = nil
                reader.pause()
            }
    }

    private func nudge(_ by: Int) {
        reader.setWpm(Int(reader.frame.wpm) + by)
        showDial(for: .milliseconds(900))
    }

    private func step(_ delta: Int) {
        if delta > 0 { reader.nextSentence() } else { reader.prevSentence() }
        stripShown = true
        stripHide?.cancel()
        stripHide = Task {
            try? await Task.sleep(for: .milliseconds(900))
            if !Task.isCancelled { stripShown = false }
        }
    }

    private func showDial(for duration: Duration) {
        dialShown = true
        dialHide?.cancel()
        dialHide = Task {
            try? await Task.sleep(for: duration)
            if !Task.isCancelled { dialShown = false }
        }
    }

    private func installWheel() {
        guard wheelMonitor == nil else { return }
        wheelMonitor = NSEvent.addLocalMonitorForEvents(matching: .scrollWheel) { event in
            let dy = event.scrollingDeltaY
            if abs(dy) > 0.5, event.window?.isKeyWindow == true {
                MainActor.assumeIsolated { nudge(dy > 0 ? 5 : -5) }
            }
            return event
        }
    }

    private func removeWheel() {
        if let wheelMonitor { NSEvent.removeMonitor(wheelMonitor) }
        wheelMonitor = nil
    }

    // MARK: - Map

    private func map(height: CGFloat) -> some View {
        EdgeMap(TerrainModel(ticks: reader.ticks, position: reader.position), style: .mac)
            .contentShape(.rect)
            .onContinuousHover { phase in
                switch phase {
                case let .active(location):
                    hoverTick = EdgeMap.tick(at: location.y, height: height, count: reader.ticks.count)
                case .ended:
                    hoverTick = nil
                }
            }
            .onTapGesture { location in
                if let t = EdgeMap.tick(at: location.y, height: height, count: reader.ticks.count) {
                    reader.seek(tick: t)
                }
            }
            .accessibilityElement()
            .accessibilityLabel("Document map. Point to see a section, click to go there.")
            .accessibilityAdjustableAction { direction in
                let current = Int(reader.position.section ?? 0)
                switch direction {
                case .increment: reader.seek(section: current + 1)
                case .decrement: reader.seek(section: max(0, current - 1))
                @unknown default: break
                }
            }
    }

    private func sectionTitle(_ tick: Int) -> String {
        guard reader.ticks.indices.contains(tick) else { return title }
        let s = Int(reader.ticks[tick].section)
        return reader.outline.sections.indices.contains(s) ? (reader.outline.sections[s].title ?? title) : title
    }

    /// How far into the document a tick starts, from the engine's token
    /// positions.
    private func percent(_ tick: Int) -> Int {
        guard reader.ticks.indices.contains(tick), reader.outline.tokens > 0 else { return 0 }
        return Int((Double(reader.ticks[tick].tokenStart) / Double(reader.outline.tokens) * 100).rounded())
    }
}

/// Gives a SwiftUI view its window, for full screen.
struct WindowReader: NSViewRepresentable {
    let onWindow: (NSWindow) -> Void

    func makeNSView(context: NSViewRepresentableContext<WindowReader>) -> NSView {
        let view = NSView()
        DispatchQueue.main.async { if let w = view.window { onWindow(w) } }
        return view
    }

    func updateNSView(_ nsView: NSView, context: NSViewRepresentableContext<WindowReader>) {}
}
#endif
