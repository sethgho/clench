import ClenchKit
import ClenchUI
import SwiftUI
import UniformTypeIdentifiers

/// The library in two columns (Mac library window, Duo library boards):
/// the list on the left, the chosen item's sections on the right. Used on
/// the Mac and in any wide window.
struct SplitLibraryView: View {
    let onSettings: () -> Void

    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors
    @State private var budget = TimeBudget.any
    @State private var search = ""
    @State private var selection: UUID?
    @State private var clipboard = ClipboardWatch()
    @State private var dropTargeted = false
    @FocusState private var searchFocused: Bool

    var body: some View {
        let shelves = model.shelves(budget: budget, search: search)
        let chosen = selection.flatMap(model.item(id:)) ?? shelves.current ?? shelves.upNext.first
        GeometryReader { geo in
            HStack(spacing: 0) {
                list(shelves, selected: chosen?.id)
                    .frame(width: min(460, geo.size.width * 0.45))
                    .overlay(alignment: .trailing) { Rectangle().fill(colors.faint).frame(width: 1) }
                Group {
                    if let chosen {
                        ItemDetail(item: chosen, onOpen: { model.open(chosen.id, at: $0) },
                                   buttonHint: hint, titleSize: 30)
                            .id(chosen.id)
                    } else {
                        Text("Nothing selected")
                            .font(.plexSans(15, relativeTo: .body))
                            .foregroundStyle(colors.mute)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                }
                .padding(.horizontal, 48)
                .padding(.top, 60)
                .padding(.bottom, 32)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            }
        }
        .background(colors.bg.ignoresSafeArea())
        .overlay {
            if dropTargeted {
                RoundedRectangle(cornerRadius: 12).stroke(colors.accent, lineWidth: 2).padding(6).allowsHitTesting(false)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            for url in urls { save(url) }
            return !urls.isEmpty
        } isTargeted: { dropTargeted = $0 }
        .onAppear { clipboard.start() }
        .background {
            Button("Search") { searchFocused = true }
                .keyboardShortcut("f", modifiers: .command)
                .hidden()
        }
    }

    private var hint: String? {
        #if os(macOS)
        model.preferences.macReader == .fullScreen ? "Opens full screen" : "Opens in a window"
        #else
        "Tap a section to start there"
        #endif
    }

    private func list(_ shelves: LibraryShelves, selected: UUID?) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 16) {
                Wordmark()
                Spacer()
                HStack(spacing: 8) {
                    Glyph.search.view(size: 16).foregroundStyle(colors.mute)
                    TextField("Search", text: $search, prompt: Text("Search  ⌘F").foregroundStyle(colors.mute))
                        .textFieldStyle(.plain)
                        .font(.plexSans(14, relativeTo: .body))
                        .foregroundStyle(colors.ink)
                        .focused($searchFocused)
                        .accessibilityLabel("Search titles and text")
                }
                .padding(.horizontal, 10)
                .frame(maxWidth: 220, minHeight: 34)
                .overlay(RoundedRectangle(cornerRadius: 10).stroke(colors.faint, lineWidth: 1))
                #if os(iOS)
                IconButton(glyph: .sliders, label: "Settings", action: onSettings).padding(.trailing, -12)
                #endif
            }
            .frame(height: 44)

            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    BudgetChips(budget: $budget).padding(.top, 14)
                    if clipboard.hasLink {
                        ClipboardCard { model.add(.url($0, title: nil)) }.padding(.top, 12)
                    }
                    if let current = shelves.current {
                        ContinueCard(item: current)
                            .padding(.top, 20)
                            .overlay(alignment: .bottom) { Rectangle().fill(colors.faint).frame(height: 1) }
                            .selectable(current.id == selected, colors: colors)
                            .onTapGesture(count: 2) { model.open(current.id) }
                            .onTapGesture { selection = current.id }
                            .contextMenu { menu(current) }
                    }
                    if !shelves.upNext.isEmpty {
                        Text("Up next").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute).padding(.top, 18)
                    }
                    ForEach(shelves.upNext) { item in
                        ItemRow(item: item, busy: model.isBusy(item.id), onRetry: { model.retry(item.id) })
                            .overlay(alignment: .bottom) { Rectangle().fill(colors.faint).frame(height: 1) }
                            .selectable(item.id == selected, colors: colors)
                            .onTapGesture(count: 2) { model.open(item.id) }
                            .onTapGesture { selection = item.id }
                            .contextMenu { menu(item) }
                    }
                    if let shortest = shelves.shortestHidden {
                        Text("Nothing saved is that short. The shortest is \(shortest) min.")
                            .font(.plexSans(14, relativeTo: .subheadline)).foregroundStyle(colors.mute).padding(.vertical, 16)
                    }
                    if !shelves.finished.isEmpty {
                        Text("Finished · \(shelves.finished.count)")
                            .font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute).padding(.top, 18)
                        ForEach(shelves.finished.prefix(20)) { item in
                            Text(item.title)
                                .font(.plexSans(14, relativeTo: .body))
                                .foregroundStyle(colors.mute)
                                .lineLimit(1)
                                .padding(.vertical, 8)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .contentShape(.rect)
                                .onTapGesture { selection = item.id }
                                .contextMenu { menu(item) }
                        }
                    }
                }
            }
            .scrollIndicators(.never)

            #if os(macOS)
            Text("Drop a link, PDF or text file anywhere in this window to save it.")
                .font(.plexSans(13, relativeTo: .footnote))
                .foregroundStyle(colors.mute)
                .lineSpacing(5)
                .padding(.top, 14)
            #endif
        }
        .padding(.horizontal, 32)
        .padding(.top, 52)
        .padding(.bottom, 28)
    }

    @ViewBuilder
    private func menu(_ item: QueueItem) -> some View {
        Button(item.state == .done ? "Read again" : "Read now") { model.open(item.id) }
        if item.state == .done {
            Button("Mark as not finished") { model.markFinished(item.id, false) }
        } else {
            Button("Mark as finished") { model.markFinished(item.id) }
        }
        Divider()
        Button("Delete", role: .destructive) { model.remove([item.id]) }
    }

    private func save(_ url: URL) {
        if url.isFileURL {
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            guard let data = try? Data(contentsOf: url) else { return }
            let type = UTType(filenameExtension: url.pathExtension)
            let contentType: String? = if type?.conforms(to: .pdf) == true {
                "application/pdf"
            } else if type?.conforms(to: .html) == true {
                "text/html"
            } else {
                nil
            }
            model.add(.file(data, contentType: contentType, name: url.lastPathComponent))
        } else if url.scheme == "http" || url.scheme == "https" {
            model.add(.url(url, title: nil))
        }
    }
}

private extension View {
    func selectable(_ selected: Bool, colors: Colors) -> some View {
        padding(.horizontal, 8)
            .background(selected ? colors.faint.opacity(0.35) : .clear, in: .rect(cornerRadius: 8))
            .padding(.horizontal, -8)
            .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
