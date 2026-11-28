import ClenchKit
import SwiftUI

struct QueueView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        if model.items.isEmpty {
            ContentUnavailableView(
                "Nothing queued",
                systemImage: "text.book.closed",
                description: Text("Share a page to clench, or paste text or a link."))
        } else {
            List {
                ForEach(model.items) { item in
                    NavigationLink(value: item.id) {
                        QueueRow(item: item)
                    }
                }
                .onDelete { offsets in
                    model.remove(offsets.map { model.items[$0].id })
                }
            }
        }
    }
}

private struct QueueRow: View {
    let item: QueueItem

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(item.title).lineLimit(2)
            HStack(spacing: 8) {
                if let host = item.url?.host() { Text(host) }
                if let words = item.wordCount { Text("\(words) words") }
                Text(item.state.rawValue)
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }
}
