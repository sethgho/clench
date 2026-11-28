#if DEBUG
import ClenchKit
import Foundation
import SwiftUI

/// Debug-only launch arguments for screenshots and manual testing:
///
/// * `-clench-demo` fills an empty queue with sample items.
/// * `-clench-screen reader|settings|peek|finished|end` opens that screen.
/// * `-clench-reader dial|scrub|strip|playing` shows the reader in that state.
enum Demo {
    static var arguments: [String] { ProcessInfo.processInfo.arguments }

    static func value(_ name: String) -> String? {
        guard let i = arguments.firstIndex(of: name), arguments.indices.contains(i + 1) else { return nil }
        return arguments[i + 1]
    }

    static var enabled: Bool { arguments.contains("-clench-demo") }
    static var screen: String? { value("-clench-screen") }
    static var readerState: String? { value("-clench-reader") }

    private static let studies = """
    # Of Studies

    Studies serve for delight, for ornament, and for ability. Their chief use for delight, is in privateness and retiring; for ornament, is in discourse; and for ability, is in the judgment, and disposition of business.

    Read not to contradict and confute; nor to believe and take for granted; nor to find talk and discourse; but to weigh and consider. Some books are to be tasted, others to be swallowed, and some few to be chewed and digested.

    Reading maketh a full man; conference a ready man; and writing an exact man.
    """

    /// Seeds an empty queue in `store`.
    @MainActor
    static func seed(_ store: QueueStore, wpm: Int) {
        guard enabled, (try? store.items().isEmpty) == true else { return }
        let now = Date.now
        func add(_ title: String, _ markdown: String, url: String, added: TimeInterval, byline: String, at token: Int? = nil,
                 state: QueueItem.State = .queued) {
            let data = Data(markdown.utf8)
            guard var doc = try? DocumentLoader.extract(data, contentType: "text/markdown", url: URL(string: url)) else { return }
            doc.byline = byline
            doc.siteName = "gutenberg.org"
            guard let session = try? Session(document: doc, settingsJson: ReaderModel.settings(wpm: wpm)) else { return }
            if let token { session.seek(index: UInt64(token)) }
            let summary = ItemSummary(document: doc, session: session)
            let item = QueueItem(url: URL(string: url), title: title, added: now.addingTimeInterval(added), state: state,
                                 wordCount: summary.words,
                                 position: .init(index: token ?? 0, wpm: token == nil ? nil : wpm, lastRead: token == nil ? nil : now),
                                 summary: summary, finished: state == .done ? now : nil)
            _ = try? store.add(item, content: data, contentType: "text/markdown")
        }
        add("Nature", Sample.markdown, url: "https://www.gutenberg.org/ebooks/29433", added: -9000,
            byline: "Ralph Waldo Emerson", at: 52, state: .reading)
        add("Of Studies", studies, url: "https://www.gutenberg.org/ebooks/575", added: -8000, byline: "Francis Bacon")
        add("Self-Reliance", Sample.markdown + "\n\n" + studies + "\n\n" + Sample.markdown, url: "https://www.gutenberg.org/ebooks/16643",
            added: -7000, byline: "Ralph Waldo Emerson")
        add("A Modest Proposal", studies, url: "https://www.gutenberg.org/ebooks/1080", added: -10000,
            byline: "Jonathan Swift", state: .done)
        _ = try? store.add(QueueItem(url: URL(string: "https://example.invalid/walking"), title: "Walking",
                                     added: now.addingTimeInterval(-5000),
                                     failure: "The link saved, but the text didn’t come through."))
    }
}
#endif

