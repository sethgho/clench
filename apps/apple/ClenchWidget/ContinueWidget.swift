import ClenchKit
import ClenchUI
import SwiftUI
import WidgetKit

/// What the Continue widget shows: the item in progress (or the next one),
/// its time left by the engine's estimate, and its map.
struct ContinueEntry: TimelineEntry {
    let date: Date
    let item: QueueItem?
    let minutesLeft: Int?
    let preferences: Preferences

    static func current(at date: Date = .now) -> ContinueEntry {
        let prefs = PreferencesStore().load()
        let items = (try? QueueStore.shared().items()) ?? []
        let shelves = LibraryShelves(items: items, wpm: prefs.defaultWpm)
        let item = shelves.current ?? shelves.upNext.first { $0.summary != nil }
        let ms = item.flatMap { $0.remainingMs(wpm: $0.wpm(default: prefs.defaultWpm)) }
        return ContinueEntry(date: date, item: item, minutesLeft: ms.map(ReadingTime.minutes), preferences: prefs)
    }
}

struct ContinueProvider: TimelineProvider {
    typealias Entry = ContinueEntry

    func placeholder(in context: TimelineProviderContext) -> ContinueEntry {
        ContinueEntry(date: .now, item: nil, minutesLeft: nil, preferences: Preferences())
    }

    func getSnapshot(in context: TimelineProviderContext, completion: @escaping @Sendable (ContinueEntry) -> Void) {
        completion(.current())
    }

    func getTimeline(in context: TimelineProviderContext, completion: @escaping @Sendable (Timeline<ContinueEntry>) -> Void) {
        // The app reloads the timeline whenever reading stops or the queue
        // changes; the hourly refresh only catches changes made elsewhere.
        completion(Timeline(entries: [.current()], policy: .after(.now.addingTimeInterval(3600))))
    }
}

struct ContinueWidgetView: View {
    let entry: ContinueEntry
    @Environment(\.colors) private var colors

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Wordmark(size: 13)
            Spacer(minLength: 8)
            if let item = entry.item {
                VStack(alignment: .leading, spacing: 4) {
                    Text(item.title)
                        .font(.plexSans(18, .semibold, relativeTo: .headline))
                        .foregroundStyle(colors.ink)
                        .lineLimit(2)
                    if let minutes = entry.minutesLeft {
                        Text("\(minutes) min left")
                            .font(.plexMono(12, relativeTo: .caption))
                            .foregroundStyle(colors.accent)
                            .padding(.bottom, 8)
                    }
                    if let summary = item.summary {
                        Barcode(TerrainModel(summary: summary), height: 18, gap: 1)
                    }
                }
            } else {
                Text("Nothing to read")
                    .font(.plexSans(16, .semibold, relativeTo: .headline))
                    .foregroundStyle(colors.ink)
                Text("Share a page to clench")
                    .font(.plexSans(12, relativeTo: .caption))
                    .foregroundStyle(colors.mute)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .containerBackground(for: .widget) { colors.bg }
        .widgetURL(entry.item.map { DeepLink.read($0.id).url } ?? DeepLink.queue.url)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(entry.item.map { "Continue \($0.title)" + (entry.minutesLeft.map { ", \($0) minutes left" } ?? "") } ?? "clench, nothing to read")
    }
}

struct ContinueWidget: Widget {
    init() {
        ClenchFonts.register()
    }

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "Continue", provider: ContinueProvider()) { entry in
            ContinueWidgetView(entry: entry)
                .clenchTheme(entry.preferences)
        }
        .configurationDisplayName("Continue")
        .description("Pick up where you stopped reading.")
        .supportedFamilies([.systemSmall])
    }
}

@main
struct ClenchWidgets: WidgetBundle {
    var body: some Widget {
        ContinueWidget()
    }
}

#Preview(as: .systemSmall) {
    ContinueWidget()
} timeline: {
    ContinueEntry(date: .now, item: QueueItem(url: URL(string: "https://www.gutenberg.org/nature"), title: "Nature"),
                  minutesLeft: 39, preferences: Preferences())
}
