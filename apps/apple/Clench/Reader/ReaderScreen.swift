import ClenchKit
import ClenchUI
import SwiftUI

/// Loads a queued item and shows it in the reader that fits the window,
/// saving the position whenever reading stops and when the reader goes away.
struct ReaderScreen: View {
    let itemID: UUID
    let onClose: () -> Void

    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors
    @Environment(\.scenePhase) private var scenePhase
    @State private var phase: Phase = .loading
    @State private var settings = false
    @State private var hinge = HingePosture.unknown

    private enum Phase {
        case loading
        case ready(ReaderModel)
        case failed(String)
    }

    var body: some View {
        ZStack {
            colors.bg.ignoresSafeArea()
            switch phase {
            case .loading:
                ProgressView().tint(colors.mute)
            case let .ready(reader):
                reading(reader)
                    .onChange(of: reader.state) { old, new in
                        if old == .playing { save(reader) }
                        if new == .finished { save(reader) }
                    }
                    .onChange(of: scenePhase) { _, phase in
                        if phase != .active {
                            reader.pause()
                            save(reader)
                        }
                    }
                    .onDisappear {
                        reader.stop()
                        save(reader)
                    }
            case let .failed(message):
                failure(message)
            }
        }
        .observeHinge($hinge)
        .task(id: itemID) { await load() }
        .sheet(isPresented: $settings) {
            SettingsView(onClose: { settings = false })
                .presentationBackground(colors.bg)
                .clenchTheme(model.preferences)
        }
    }

    @ViewBuilder
    private func reading(_ reader: ReaderModel) -> some View {
        let title = model.item(id: itemID)?.title ?? reader.document.title
        if reader.state == .finished {
            EndOfItemView(
                finished: model.item(id: itemID), next: model.next(after: itemID),
                onContinue: { next in model.reading = next.id },
                onBack: onClose)
            .transition(.opacity)
        } else {
            #if os(macOS)
            MacReaderView(reader: reader, title: title, onClose: onClose)
            #else
            GeometryReader { geo in
                switch layout(for: geo.size) {
                case .stand:
                    StandReaderView(reader: reader, title: title)
                case .page:
                    PageReaderView(reader: reader, title: title, onClose: onClose, onSettings: { settings = true })
                case .phone:
                    PhoneReaderView(
                        reader: reader, title: title,
                        hint: hinge == .closed ? "Open the phone for the page view" : "Drag up or down for speed, sideways by sentence",
                        onClose: onClose, onSettings: { settings = true })
                }
            }
            .ignoresSafeArea(edges: .vertical)
            #endif
        }
    }

    private func layout(for size: CGSize) -> ReaderLayout {
        #if DEBUG
        switch Demo.value("-clench-layout") {
        case "page": return .page
        case "stand": return .stand
        default: break
        }
        #endif
        return ReaderLayout(size: size, hinge: hinge)
    }

    private func failure(_ message: String) -> some View {
        VStack(spacing: 16) {
            Text("Couldn’t read this page")
                .font(.plexSans(20, .semibold, relativeTo: .title3))
                .foregroundStyle(colors.ink)
            Text(message)
                .font(.plexSans(15, relativeTo: .body))
                .foregroundStyle(colors.mute)
                .multilineTextAlignment(.center)
            Button("Back to Later", action: onClose)
                .buttonStyle(SecondaryButtonStyle(height: 48))
                .frame(maxWidth: 280)
        }
        .padding(32)
    }

    private func load() async {
        phase = .loading
        guard let store = model.store, let item = model.item(id: itemID) else {
            phase = .failed("This item is no longer in the queue.")
            return
        }
        do {
            let document = try await DocumentLoader().document(for: item, in: store)
            let reader = try ReaderModel(
                document: document, settingsJSON: ReaderModel.settings(wpm: model.wpm(for: item)),
                tickCount: ReaderLayout.tickCount)
            if let start = model.takeSeek(itemID) {
                reader.seek(start)
            } else if item.state != .done {
                reader.seek(item.position.index)
            }
            phase = .ready(reader)
            #if DEBUG
            if Demo.screen == "end" {
                reader.seek(Int(reader.outline.tokens) - 1)
                reader.play()
            }
            #endif
        } catch {
            phase = .failed(DocumentLoader.describe(error))
        }
    }

    private func save(_ reader: ReaderModel) {
        model.save(id: itemID, reader: reader)
    }
}

/// Which reader fits a window.
enum ReaderLayout: Equatable {
    /// One word at a time, full screen.
    case phone
    /// The word beside its page: a two-screen phone opened flat, an iPad or
    /// any wide window.
    case page
    /// A foldable phone propped half open: the word on the top half, hands
    /// free controls below.
    case stand

    #if os(macOS)
    static let tickCount: UInt32 = 120
    #else
    static let tickCount: UInt32 = 96
    #endif

    init(size: CGSize, hinge: HingePosture) {
        if hinge == .partiallyOpen, size.height > size.width {
            self = .stand
        } else if size.width >= 700, size.width > size.height * 1.15 {
            self = .page
        } else {
            self = .phone
        }
    }
}

/// End of an item (Next board): the finished item's map folded into a
/// barcode at the top, and the next title risen into the centre.
struct EndOfItemView: View {
    let finished: QueueItem?
    let next: QueueItem?
    let onContinue: (QueueItem) -> Void
    let onBack: () -> Void

    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var risen = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 8) {
                Text("Finished").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                Text(finished?.title ?? "")
                    .font(.plexSans(18, .semibold, relativeTo: .headline))
                    .foregroundStyle(colors.mute)
                if let summary = finished?.summary {
                    Barcode(TerrainModel(summary: summary, done: true), height: 18)
                }
            }
            .padding(.top, 120)

            Spacer(minLength: 40)

            if let next {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Up next").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                    Text(next.title)
                        .font(.plexSans(36, .semibold, relativeTo: .largeTitle))
                        .tracking(-0.7)
                        .foregroundStyle(colors.ink)
                        .lineLimit(3)
                    Text(nextLine(next))
                        .font(.plexSans(14, relativeTo: .subheadline))
                        .foregroundStyle(colors.mute)
                        .padding(.bottom, 10)
                    if let summary = next.summary {
                        Barcode(TerrainModel(summary: summary), height: 26)
                    }
                }
                .offset(y: risen || reduceMotion ? 0 : 40)
                .opacity(risen ? 1 : 0)
            }

            Spacer(minLength: 40)

            VStack(spacing: 4) {
                if let next {
                    Text("Hold to continue")
                        .font(.plexSans(16, .medium, relativeTo: .callout))
                        .foregroundStyle(colors.ink)
                        .padding(.horizontal, 24)
                        .frame(minHeight: 44)
                        .contentShape(.rect)
                        .onLongPressGesture(minimumDuration: 0.3) { onContinue(next) }
                        .accessibilityAddTraits(.isButton)
                        .accessibilityLabel("Continue with \(next.title)")
                        .accessibilityAction { onContinue(next) }
                }
                Button(action: onBack) {
                    Text("Back to Later")
                        .font(.plexSans(14, relativeTo: .subheadline))
                        .underline()
                        .foregroundStyle(colors.mute)
                        .padding(.horizontal, 24)
                        .frame(minHeight: 44)
                }
                .buttonStyle(.plain)
            }
            .frame(maxWidth: .infinity)
            .padding(.bottom, 64)
        }
        .padding(.horizontal, 24)
        .frame(maxWidth: 560)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(colors.bg.ignoresSafeArea())
        .sensoryFeedback(.impact(weight: .light, intensity: 0.6), trigger: risen)
        .onAppear {
            withAnimation(reduceMotion ? .easeOut(duration: 0.15) : .easeOut(duration: 0.63).delay(0.3)) { risen = true }
        }
    }

    private func nextLine(_ item: QueueItem) -> String {
        let minutes = item.remainingMs(wpm: item.wpm(default: preferences.defaultWpm)).map(ReadingTime.label)
        return [item.summary?.byline ?? item.attribution, minutes].compactMap { $0 }.joined(separator: " · ")
    }
}
