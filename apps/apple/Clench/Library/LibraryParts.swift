import ClenchKit
import ClenchUI
import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

/// "I have [Any] [5 min] [15 min] [30 min]".
struct BudgetChips: View {
    @Binding var budget: TimeBudget
    @Environment(\.colors) private var colors

    var body: some View {
        ViewThatFits(in: .horizontal) {
            row
            ScrollView(.horizontal, showsIndicators: false) { row }
        }
    }

    private var row: some View {
        HStack(spacing: 8) {
            Text("I have")
                .font(.plexSans(15, relativeTo: .subheadline))
                .foregroundStyle(colors.mute)
                .padding(.trailing, 4)
                .fixedSize()
            ForEach(TimeBudget.allCases) { b in
                let on = b == budget
                Button { budget = b } label: {
                    Text(b.label)
                        .font(.plexSans(14, .medium, relativeTo: .subheadline))
                        .fixedSize()
                        .padding(.horizontal, 10)
                        .frame(minHeight: 44)
                        .foregroundStyle(on ? colors.bg : colors.ink)
                        .background(on ? colors.ink : .clear, in: .capsule)
                        .overlay(Capsule().stroke(on ? colors.ink : colors.faint, lineWidth: 1.5))
                        .contentShape(.capsule)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(b == .any ? "Any length" : "\(b.rawValue) minutes or less")
                .accessibilityAddTraits(on ? .isSelected : [])
            }
        }
    }
}

/// The item being read, large, with its map and time left.
struct ContinueCard: View {
    let item: QueueItem
    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Continue").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
            Text(item.title)
                .font(.plexSans(26, .semibold, relativeTo: .title))
                .tracking(-0.26)
                .foregroundStyle(colors.ink)
                .lineLimit(3)
            if let by = item.attribution {
                Text(by).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute).padding(.bottom, 8)
            }
            if let summary = item.summary {
                Barcode(TerrainModel(summary: summary), height: 26)
            }
            HStack {
                Text(item.summary?.currentSectionTitle ?? "")
                    .foregroundStyle(colors.mute)
                    .lineLimit(1)
                Spacer()
                if let ms = item.remainingMs(wpm: item.wpm(default: preferences.defaultWpm)) {
                    Text("\(ReadingTime.minutes(ms)) min left").foregroundStyle(colors.accent)
                }
            }
            .font(.plexMono(12, relativeTo: .caption))
            .padding(.top, 4)
        }
        .padding(.bottom, 20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(.rect)
        .accessibilityElement(children: .combine)
    }
}

/// One row of "Up next": title, attribution, length and map; or the item
/// still being saved, or the item that could not be read.
struct ItemRow: View {
    let item: QueueItem
    let busy: Bool
    var onRetry: () -> Void = {}
    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        Group {
            if let failure = item.failure {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Couldn’t read this page").font(.plexSans(16, .semibold, relativeTo: .headline)).foregroundStyle(colors.ink)
                        Text(failure).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                        Text(item.title).font(.plexMono(11, relativeTo: .caption2)).foregroundStyle(colors.mute).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                    Button("Retry", action: onRetry)
                        .font(.plexSans(15, .medium, relativeTo: .body))
                        .foregroundStyle(colors.ink)
                        .padding(.horizontal, 16)
                        .frame(minHeight: 44)
                        .overlay(RoundedRectangle(cornerRadius: 12).stroke(colors.mute, lineWidth: 1.5))
                        .buttonStyle(.plain)
                }
            } else if let summary = item.summary {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(alignment: .firstTextBaseline, spacing: 16) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.title).font(.plexSans(16, .semibold, relativeTo: .headline)).foregroundStyle(colors.ink).lineLimit(2)
                            if let by = summary.byline ?? item.attribution {
                                Text(by).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute).lineLimit(1)
                            }
                        }
                        Spacer(minLength: 0)
                        if let ms = item.remainingMs(wpm: item.wpm(default: preferences.defaultWpm)) {
                            Text(ReadingTime.label(ms)).font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute).fixedSize()
                        }
                    }
                    Barcode(TerrainModel(summary: summary), height: 12)
                }
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    Text(item.title).font(.plexSans(16, .semibold, relativeTo: .headline)).foregroundStyle(colors.ink).lineLimit(2)
                    Text([item.attribution, busy ? "saving for offline" : "waiting to save"].compactMap { $0 }.joined(separator: " · "))
                        .font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                    Rectangle().fill(colors.faint).frame(width: 120, height: 1).frame(height: 12, alignment: .bottom)
                }
            }
        }
        .padding(.vertical, 14)
        .frame(maxWidth: .infinity, minHeight: 90, alignment: .leading)
        .contentShape(.rect)
        .accessibilityElement(children: .combine)
    }
}

/// "Link on your clipboard" with a paste button. Only whether the clipboard
/// holds a link is checked, which needs no permission; the link itself is
/// read when the button is pressed.
struct ClipboardCard: View {
    let onPaste: (URL) -> Void
    @Environment(\.colors) private var colors

    var body: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Link on your clipboard").font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                Text("Paste it to save for later").font(.plexSans(15, .medium, relativeTo: .body)).foregroundStyle(colors.ink)
            }
            Spacer(minLength: 0)
            PasteButton(payloadType: URL.self) { urls in
                if let url = urls.first { onPaste(url) }
            }
            .labelStyle(.titleOnly)
            .buttonBorderShape(.roundedRectangle(radius: 12))
            .tint(colors.ink)
            .frame(minHeight: 44)
        }
        .padding(.vertical, 12)
        .padding(.leading, 16)
        .padding(.trailing, 12)
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(colors.faint, lineWidth: 1))
    }
}

/// The sections of an item (Peek board), with time left per section from
/// the engine's stored outline; tap one to start there.
struct SectionList: View {
    let item: QueueItem
    let onPick: (Int) -> Void
    var rowHeight: CGFloat = 46
    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        if let summary = item.summary {
            let current = item.state == .done ? summary.sections.count : (summary.mark.section ?? summary.sections.count)
            let wpm = item.wpm(default: preferences.defaultWpm)
            VStack(spacing: 0) {
                ForEach(Array(summary.sections.enumerated()), id: \.offset) { i, section in
                    let read = i < current
                    let isCurrent = i == current && summary.started
                    Button { onPick(section.tokenStart) } label: {
                        HStack(spacing: 12) {
                            Rectangle()
                                .fill(isCurrent ? colors.accent : (read ? colors.faint : colors.ink))
                                .frame(width: isCurrent ? 30 : 22, height: isCurrent ? 3 : 2)
                                .frame(width: 30, alignment: .leading)
                            Text(section.title ?? item.title)
                                .font(.plexSans(15, .medium, relativeTo: .body))
                                .foregroundStyle(read ? colors.mute : colors.ink)
                                .lineLimit(1)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            sectionTicks(summary, section: i, read: read)
                            Text(read ? "read" : ReadingTime.label(i == current ? summary.mark.sectionRemaining.ms(at: wpm) : section.timing.ms(at: wpm)))
                                .font(.plexMono(12, relativeTo: .caption))
                                .foregroundStyle(colors.mute)
                                .frame(width: 52, alignment: .trailing)
                        }
                        .frame(minHeight: max(44, rowHeight))
                        .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("\(section.title ?? item.title), \(read ? "read" : ReadingTime.label(section.timing.ms(at: wpm)))")
                    .accessibilityHint("Start reading here")
                }
            }
        }
    }

    private func sectionTicks(_ summary: ItemSummary, section: Int, read: Bool) -> some View {
        let ticks = summary.ticks.filter { $0.section == section && !$0.heading }.prefix(12)
        let marks = ticks.map { TerrainModel.Mark(heading: false, weight: Double($0.weight) / 1000) }
        return Barcode(TerrainModel(marks: Array(marks), current: nil, finished: read), height: 14)
            .frame(width: CGFloat(marks.count) * 3)
    }
}

/// Item detail for a sheet or a side pane: title, attribution, sections,
/// and the Continue button.
struct ItemDetail: View {
    let item: QueueItem
    let onOpen: (Int?) -> Void
    var buttonHint: String? = "Tap a section to start there"
    var titleSize: CGFloat = 26
    @Environment(\.colors) private var colors
    @Environment(\.preferences) private var preferences

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(item.title)
                .font(.plexSans(titleSize, .semibold, relativeTo: .title))
                .tracking(-0.26)
                .foregroundStyle(colors.ink)
            if let by = item.attribution {
                Text(by).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute).padding(.top, 4)
            }
            ScrollView {
                SectionList(item: item, onPick: { onOpen($0) })
            }
            .scrollBounceBehavior(.basedOnSize)
            .padding(.top, 16)
            VStack(spacing: 12) {
                Button { onOpen(nil) } label: { Text(continueLabel) }
                    .buttonStyle(PrimaryButtonStyle())
                    .keyboardShortcut(.defaultAction)
                if let buttonHint {
                    Text(buttonHint).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                }
            }
            .padding(.top, 12)
        }
    }

    private var continueLabel: String {
        let wpm = item.wpm(default: preferences.defaultWpm)
        guard let ms = item.remainingMs(wpm: wpm), item.state != .done else { return item.state == .done ? "Read again" : "Read now" }
        let started = item.summary?.started ?? false
        return started ? "Continue · \(ReadingTime.minutes(ms)) min left" : "Read now · \(ReadingTime.label(ms))"
    }
}

/// Watches whether the clipboard holds a link.
@MainActor
@Observable
final class ClipboardWatch {
    private(set) var hasLink = false
    @ObservationIgnored private var observer: NSObjectProtocol?

    func refresh() {
        #if os(iOS)
        hasLink = UIPasteboard.general.hasURLs
        #elseif os(macOS)
        hasLink = NSPasteboard.general.canReadObject(forClasses: [NSURL.self], options: nil)
        #endif
    }

    func start() {
        refresh()
        #if os(iOS)
        guard observer == nil else { return }
        observer = NotificationCenter.default.addObserver(
            forName: UIPasteboard.changedNotification, object: nil, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.refresh() }
        }
        #endif
    }
}
