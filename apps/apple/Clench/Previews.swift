import ClenchFFI
import ClenchKit
import ClenchUI
import SwiftUI

/// Sample content for SwiftUI previews and screenshots: the opening of
/// Emerson's "Nature" (1836, public domain), read in by the engine.
enum Sample {
    static let markdown = """
    # Nature

    ## Introduction

    Our age is retrospective. It builds the sepulchres of the fathers. It writes biographies, histories, and criticism. The foregoing generations beheld God and nature face to face; we, through their eyes.

    Why should not we also enjoy an original relation to the universe? Why should not we have a poetry and philosophy of insight and not of tradition, and a religion by revelation to us, and not the history of theirs?

    ## I. Nature

    To go into solitude, a man needs to retire as much from his chamber as from society. I am not solitary whilst I read and write, though nobody is with me. But if a man would be alone, let him look at the stars. The rays that come from those heavenly worlds, will separate between him and what he touches.

    One might think the atmosphere was made transparent with this design, to give man, in the heavenly bodies, the perpetual presence of the sublime.

    The stars awaken a certain reverence, because though always present, they are inaccessible; but all natural objects make a kindred impression, when the mind is open to their influence.

    ## II. Commodity

    Whoever considers the final cause of the world, will discern a multitude of uses that enter as parts into that result. They all admit of being thrown into one of the following classes; Commodity; Beauty; Language; and Discipline.

    ## III. Beauty

    A nobler want of man is served by nature, namely, the love of Beauty. The ancient Greeks called the world κόσμος, beauty.
    """

    static func document() -> ClenchFFI.Document {
        var doc = (try? DocumentLoader.extract(Data(markdown.utf8), contentType: "text/markdown", url: nil))
            ?? ClenchFFI.Document(url: "", title: "Nature", byline: nil, siteName: nil, lang: nil, excerpt: nil, source: .markdown,
                        partial: false, paywallHint: false, wordCount: 0, blocks: [])
        doc.byline = "Ralph Waldo Emerson"
        doc.siteName = "gutenberg.org"
        return doc
    }

    @MainActor
    static func reader(at token: Int = 40) -> ReaderModel? {
        let reader = try? ReaderModel(document: document(), settingsJSON: ReaderModel.settings(wpm: 320))
        reader?.seek(token)
        return reader
    }

    /// A queue in a temporary directory holding sample items.
    @MainActor
    static func model() -> AppModel {
        let dir = FileManager.default.temporaryDirectory.appending(path: "clench-preview-\(UUID().uuidString)")
        let store = QueueStore(directory: dir)
        let doc = document()
        func summary(at token: Int) -> ItemSummary? {
            guard let s = try? Session(document: doc, settingsJson: ReaderModel.settings(wpm: 320)) else { return nil }
            s.seek(index: UInt64(token))
            return ItemSummary(document: doc, session: s)
        }
        let now = Date.now
        let items = [
            QueueItem(url: URL(string: "https://www.gutenberg.org/nature"), title: "Nature", added: now.addingTimeInterval(-9000),
                      state: .reading, position: .init(index: 40, wpm: 320, lastRead: now), summary: summary(at: 40)),
            QueueItem(url: URL(string: "https://example.org/of-studies"), title: "Of Studies", added: now.addingTimeInterval(-8000),
                      summary: summary(at: 0)),
            QueueItem(url: URL(string: "https://example.org/walking"), title: "Walking", added: now.addingTimeInterval(-7000)),
            QueueItem(url: URL(string: "https://example.org/missing"), title: "example.org", added: now.addingTimeInterval(-6000),
                      failure: "The link saved, but the text didn’t come through."),
        ]
        for item in items { _ = try? store.add(item) }
        return AppModel(store: store, preferencesStore: PreferencesStore(suiteName: "clench-preview"))
    }
}

/// Builds a sample reader for a preview.
struct PreviewReader<Content: View>: View {
    @ViewBuilder let content: (ReaderModel) -> Content
    @State private var reader = Sample.reader()

    var body: some View {
        Group {
            if let reader { content(reader) } else { Text("The engine could not read the sample.") }
        }
        .clenchTheme(Preferences())
        .onAppear { ClenchFonts.register() }
    }
}

#Preview("Library") {
    let model = Sample.model()
    return LibraryView(onSettings: {})
        .environment(model)
        .clenchTheme(model.preferences)
        .onAppear { ClenchFonts.register() }
}

#Preview("Library, wide") {
    let model = Sample.model()
    return SplitLibraryView(onSettings: {})
        .environment(model)
        .clenchTheme(model.preferences)
        .frame(width: 1120, height: 720)
        .onAppear { ClenchFonts.register() }
}

#Preview("Settings") {
    let model = Sample.model()
    return SettingsView(onClose: {})
        .environment(model)
        .clenchTheme(model.preferences)
        .onAppear { ClenchFonts.register() }
}

#Preview("Page reader", traits: .landscapeLeft) {
    PreviewReader { reader in
        PageReaderView(reader: reader, title: "Nature", onClose: {}, onSettings: {})
    }
}

#Preview("Stand reader") {
    PreviewReader { reader in
        StandReaderView(reader: reader, title: "Nature")
    }
}

#Preview("End of item") {
    let model = Sample.model()
    let items = model.shelves().upNext
    return EndOfItemView(finished: model.shelves().current, next: items.first, onContinue: { _ in }, onBack: {})
        .clenchTheme(model.preferences)
        .onAppear { ClenchFonts.register() }
}
