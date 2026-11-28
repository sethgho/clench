import ClenchKit
import ClenchUI
import Foundation
import Observation
import SwiftUI

/// State for the share sheet: what was shared, the engine's preview of it,
/// and the two actions.
@MainActor
@Observable
final class ShareModel {
    enum Phase: Equatable {
        case loading
        case ready(SharedPayload)
        case nothing
        case failed(String)
        case queuedNotOpened
    }

    /// The largest page the extension reads in for its preview. Bigger
    /// pages are saved as they are and read in by the app.
    static let previewLimit = 3 * 1024 * 1024

    private(set) var phase: Phase = .loading
    private(set) var summary: ItemSummary?
    let preferences = PreferencesStore().load()

    @ObservationIgnored private weak var context: NSExtensionContext?
    @ObservationIgnored private let openURL: @MainActor (URL) async -> Bool
    @ObservationIgnored private var fetched: (data: Data, contentType: String?)?

    init(context: NSExtensionContext?, openURL: @escaping @MainActor (URL) async -> Bool) {
        self.context = context
        self.openURL = openURL
    }

    var title: String {
        if let t = summary?.title, !t.isEmpty { return t }
        guard case let .ready(payload) = phase else { return "" }
        return payload.queueEntry().item.title
    }

    var attribution: String? {
        guard case let .ready(payload) = phase else { return nil }
        let host = payload.queueEntry().item.host
        return summary?.attribution(host: host) ?? host
    }

    func load() async {
        let items = context?.inputItems.compactMap { $0 as? NSExtensionItem } ?? []
        guard let payload = await ShareInput.payload(from: items) else {
            phase = .nothing
            return
        }
        phase = .ready(payload)
        await preview(payload)
    }

    /// Reads the shared page in with the engine for the sheet's map and
    /// length. A link is downloaded here, and the page kept with the item.
    private func preview(_ payload: SharedPayload) async {
        let entry = payload.queueEntry()
        var data = entry.content
        var contentType = entry.contentType
        var url = entry.item.url
        if data == nil, let link = url {
            let loader = DocumentLoader(maxBytes: Self.previewLimit)
            if let page = try? await loader.download(link, timeout: 12) {
                data = page.data
                contentType = page.contentType
                url = page.url
                fetched = (page.data, page.contentType)
            }
        }
        guard let data, data.count <= Self.previewLimit else { return }
        let wpm = preferences.defaultWpm
        let result = await Task.detached(priority: .userInitiated) { () -> ItemSummary? in
            guard let doc = try? DocumentLoader.extract(data, contentType: contentType, url: url),
                  let session = try? Session(document: doc, settingsJson: ReaderModel.settings(wpm: wpm))
            else { return nil }
            return ItemSummary(document: doc, session: session)
        }.value
        summary = result
    }

    func addToQueue() {
        if enqueue() != nil { finish() }
    }

    func readNow() async {
        guard let item = enqueue() else { return }
        if await openURL(DeepLink.read(item.id).url) {
            finish()
        } else {
            phase = .queuedNotOpened
        }
    }

    func cancel() {
        context?.cancelRequest(withError: CocoaError(.userCancelled))
    }

    func finish() {
        context?.completeRequest(returningItems: nil)
    }

    private func enqueue() -> QueueItem? {
        guard case let .ready(payload) = phase else { return nil }
        var entry = payload.queueEntry()
        if let summary {
            entry.item.summary = summary
            entry.item.wordCount = summary.words
            if DocumentLoader.isFallbackTitle(entry.item.title, for: entry.item), !summary.title.isEmpty {
                entry.item.title = summary.title
            }
        }
        if entry.content == nil, let fetched {
            entry.content = fetched.data
            entry.contentType = fetched.contentType
        }
        do {
            return try QueueStore.shared().add(entry.item, content: entry.content, contentType: entry.contentType)
        } catch {
            phase = .failed(error.localizedDescription)
            return nil
        }
    }
}

/// The share sheet (Share to Clench board; Mac capture board on macOS):
/// title, byline and site, the document map, sections and minutes, and
/// Read now or Save for later.
struct ShareView: View {
    let model: ShareModel
    @Environment(\.colors) private var colors

    var body: some View {
        content
            .clenchTheme(model.preferences)
            .task { await model.load() }
    }

    @ViewBuilder
    private var content: some View {
        #if os(macOS)
        sheet
            .padding(.horizontal, 32)
            .padding(.top, 28)
            .padding(.bottom, 24)
            .frame(width: 540)
            .background(BackgroundColor())
        #else
        VStack(spacing: 0) {
            Spacer(minLength: 0)
            sheet
                .padding(.horizontal, 24)
                .padding(.top, 12)
                .padding(.bottom, 44)
                .background(BackgroundColor().clipShape(UnevenRoundedRectangle(topLeadingRadius: 28, topTrailingRadius: 28)))
        }
        .ignoresSafeArea(edges: .bottom)
        #endif
    }

    private var sheet: some View {
        SheetContent(model: model)
    }
}

private struct BackgroundColor: View {
    @Environment(\.colors) private var colors
    var body: some View { colors.sheet }
}

private struct SheetContent: View {
    let model: ShareModel
    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            #if os(iOS)
            Capsule().fill(colors.faint).frame(width: 36, height: 4).frame(maxWidth: .infinity)
            #endif
            HStack {
                Wordmark()
                Spacer()
                #if os(iOS)
                Button { model.cancel() } label: {
                    GlyphX().stroke(style: StrokeStyle(lineWidth: 1.6, lineCap: .round))
                        .frame(width: 18, height: 18)
                        .foregroundStyle(colors.mute)
                        .frame(width: 44, height: 44)
                        .contentShape(.rect)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Cancel")
                .padding(.trailing, -12)
                #endif
            }
            .frame(minHeight: 44)

            switch model.phase {
            case .loading:
                ProgressView().tint(colors.mute).frame(maxWidth: .infinity, minHeight: 120)
            case .ready:
                ready
            case .nothing:
                message("Nothing here clench can read.")
            case let .failed(text):
                message(text)
            case .queuedNotOpened:
                message("Saved to Later. Open clench to read it.")
                Button("Done") { model.finish() }.buttonStyle(SecondaryButtonStyle())
            }
            #if os(macOS)
            Button("Cancel") { model.cancel() }
                .keyboardShortcut(.cancelAction)
                .buttonStyle(.plain)
                .foregroundStyle(colors.mute)
                .font(.plexSans(13, relativeTo: .footnote))
            #endif
        }
    }

    @ViewBuilder
    private var ready: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(model.title)
                .font(.plexSans(isMac ? 30 : 28, .semibold, relativeTo: .title))
                .tracking(-0.28)
                .foregroundStyle(colors.ink)
                .lineLimit(3)
            if let by = model.attribution {
                Text(by).font(.plexSans(14, relativeTo: .subheadline)).foregroundStyle(colors.mute)
            }
        }
        if let summary = model.summary {
            VStack(spacing: 10) {
                Barcode(TerrainModel(summary: summary), height: 30, gap: isMac ? 3 : nil)
                    .accessibilityElement()
                    .accessibilityLabel("Document map: \(summary.sections.count) sections")
                HStack {
                    Text(summary.sections.count == 1 ? "1 section" : "\(summary.sections.count) sections")
                    Spacer()
                    Text("\(ReadingTime.minutes(summary.timing.ms(at: preferences.defaultWpm))) min at \(preferences.defaultWpm) wpm")
                }
                .font(.plexMono(12, relativeTo: .caption))
                .foregroundStyle(colors.mute)
            }
        }
        buttons
        #if os(macOS)
        Text("Read now takes over the whole screen. Esc brings you back here.")
            .font(.plexSans(13, relativeTo: .footnote))
            .foregroundStyle(colors.mute)
        #endif
    }

    @ViewBuilder
    private var buttons: some View {
        #if os(macOS)
        HStack(spacing: 12) {
            Button { Task { await model.readNow() } } label: {
                HStack(spacing: 10) { Text("Read now"); Text("Return").font(.plexMono(12, relativeTo: .caption)) }
            }
            .buttonStyle(PrimaryButtonStyle(height: 52))
            .keyboardShortcut(.defaultAction)
            Button { model.addToQueue() } label: {
                HStack(spacing: 10) {
                    Text("Save for later")
                    Text("⌘S").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                }
            }
            .buttonStyle(SecondaryButtonStyle(height: 52))
            .keyboardShortcut("s", modifiers: .command)
        }
        #else
        VStack(spacing: 10) {
            Button("Read now") { Task { await model.readNow() } }
                .buttonStyle(PrimaryButtonStyle())
            Button("Save for later") { model.addToQueue() }
                .buttonStyle(SecondaryButtonStyle())
        }
        #endif
    }

    private func message(_ text: String) -> some View {
        Text(text)
            .font(.plexSans(16, relativeTo: .body))
            .foregroundStyle(colors.ink)
            .frame(maxWidth: .infinity, minHeight: 80, alignment: .leading)
    }

    private var isMac: Bool {
        #if os(macOS)
        true
        #else
        false
        #endif
    }
}

private struct GlyphX: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let s = rect.width / 18
        p.move(to: CGPoint(x: 4 * s, y: 4 * s)); p.addLine(to: CGPoint(x: 14 * s, y: 14 * s))
        p.move(to: CGPoint(x: 14 * s, y: 4 * s)); p.addLine(to: CGPoint(x: 4 * s, y: 14 * s))
        return p
    }
}
