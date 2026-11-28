import ClenchKit
import ClenchUI
import SwiftUI

/// The library on a phone (Later, Later states boards): time budget, the
/// item in progress, up next with swipe actions, and the finished list.
struct LibraryView: View {
    let onSettings: () -> Void

    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors
    @State private var budget = TimeBudget.any
    @State private var searching = false
    @State private var search = ""
    @State private var peek: QueueItem?
    @State private var clipboard = ClipboardWatch()
    @State private var showFinished = false
    @FocusState private var searchFocused: Bool

    var body: some View {
        let shelves = model.shelves(budget: searching ? .any : budget, search: search)
        NavigationStack {
            List {
                header
                    .listRowStyle(colors)
                if searching {
                    searchField.listRowStyle(colors)
                } else {
                    BudgetChips(budget: $budget).padding(.top, 12).listRowStyle(colors)
                }
                if clipboard.hasLink, !searching {
                    ClipboardCard { url in model.add(.url(url, title: nil)) }
                        .padding(.top, 12)
                        .listRowStyle(colors)
                }
                if let current = shelves.current {
                    ContinueCard(item: current)
                        .padding(.top, 24)
                        .overlay(alignment: .bottom) { Rectangle().fill(colors.faint).frame(height: 1) }
                        .onTapGesture { model.open(current.id) }
                        .onLongPressGesture { peek = current }
                        .accessibilityAction { model.open(current.id) }
                        .accessibilityAction(named: "Sections") { peek = current }
                        .listRowStyle(colors)
                }
                if !shelves.upNext.isEmpty || shelves.shortestHidden != nil {
                    Text("Up next")
                        .font(.plexMono(12, relativeTo: .caption))
                        .foregroundStyle(colors.mute)
                        .padding(.top, 20)
                        .accessibilityAddTraits(.isHeader)
                        .listRowStyle(colors)
                }
                ForEach(shelves.upNext) { item in
                    ItemRow(item: item, busy: model.isBusy(item.id), onRetry: { model.retry(item.id) })
                        .overlay(alignment: .bottom) { Rectangle().fill(colors.faint).frame(height: 1) }
                        .onTapGesture { if item.failure == nil { model.open(item.id) } }
                        .onLongPressGesture { if item.summary != nil { peek = item } }
                        .accessibilityAction { model.open(item.id) }
                        .accessibilityAction(named: "Sections") { peek = item }
                        .swipeActions(edge: .leading, allowsFullSwipe: true) {
                            Button("Finished") { model.markFinished(item.id) }.tint(colors.ink)
                        }
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                            Button("Delete", role: .destructive) { model.remove([item.id]) }.tint(Color(FixedColor.delete))
                        }
                        .listRowStyle(colors)
                }
                if let shortest = shelves.shortestHidden {
                    Text("Nothing saved is that short. The shortest is \(shortest) min.")
                        .font(.plexSans(14, relativeTo: .subheadline))
                        .foregroundStyle(colors.mute)
                        .lineSpacing(6)
                        .padding(.vertical, 16)
                        .listRowStyle(colors)
                } else if shelves.current == nil, shelves.upNext.isEmpty {
                    emptyState(searching: !search.isEmpty).listRowStyle(colors)
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
            .background(colors.bg.ignoresSafeArea())
            .environment(\.defaultMinListRowHeight, 0)
            .safeAreaInset(edge: .bottom) { finishedLink(count: shelves.finished.count) }
            .hideNavigationBar()
            .navigationDestination(isPresented: $showFinished) {
                FinishedView().clenchTheme(model.preferences)
            }
        }
        .sheet(item: $peek) { item in
            PeekSheet(item: item) { token in
                peek = nil
                model.open(item.id, at: token)
            }
            .clenchTheme(model.preferences)
        }
        .onAppear {
            clipboard.start()
            #if DEBUG
            if Demo.screen == "peek" { peek = model.shelves().current }
            if Demo.screen == "finished" { showFinished = true }
            if Demo.screen == "search" {
                searching = true
                search = "nature"
            }
            #endif
        }
    }

    private var header: some View {
        HStack {
            Wordmark()
            Spacer()
            HStack(spacing: 0) {
                IconButton(glyph: searching ? .close : .search, label: searching ? "Close search" : "Search") {
                    searching.toggle()
                    if !searching { search = "" }
                    searchFocused = searching
                }
                IconButton(glyph: .sliders, label: "Settings", action: onSettings)
            }
            .padding(.trailing, -12)
        }
        .frame(height: 44)
        .padding(.top, 12)
    }

    private var searchField: some View {
        HStack(spacing: 10) {
            Glyph.search.view().foregroundStyle(colors.mute)
            TextField("Search titles and text", text: $search)
                .font(.plexSans(16, relativeTo: .body))
                .foregroundStyle(colors.ink)
                .focused($searchFocused)
                .submitLabel(.search)
                .autocorrectionDisabled()
        }
        .padding(.horizontal, 14)
        .frame(height: 46)
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(colors.mute, lineWidth: 1.5))
        .padding(.top, 8)
    }

    private func emptyState(searching: Bool) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(searching ? "Nothing matches" : "Nothing saved yet")
                .font(.plexSans(16, .semibold, relativeTo: .headline)).foregroundStyle(colors.ink)
            Text(searching ? "Try another word from a title or a section." : "Share a page to clench, or copy a link and paste it here.")
                .font(.plexSans(14, relativeTo: .subheadline)).foregroundStyle(colors.mute)
        }
        .padding(.vertical, 24)
    }

    private func finishedLink(count: Int) -> some View {
        Button { showFinished = true } label: {
            HStack {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text("Finished").font(.plexSans(15, .medium, relativeTo: .body)).foregroundStyle(colors.ink)
                    Text("\(count)").font(.plexMono(12, relativeTo: .caption)).foregroundStyle(colors.mute)
                }
                Spacer()
                Glyph.chevronRight.view(size: 18).foregroundStyle(colors.mute)
            }
            .frame(height: 48)
            .padding(.horizontal, 24)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .background(colors.bg)
        .accessibilityLabel("Finished, \(count) items")
    }
}

/// Long-press preview of an item's sections (Peek board).
struct PeekSheet: View {
    let item: QueueItem
    let onOpen: (Int?) -> Void
    @Environment(\.colors) private var colors

    var body: some View {
        ItemDetail(item: item, onOpen: onOpen)
            .padding(.horizontal, 24)
            .padding(.top, 30)
            .padding(.bottom, 12)
            .presentationDetents([.large])
            .presentationDragIndicator(.visible)
            .presentationCornerRadius(28)
            .presentationBackground(colors.sheet)
    }
}

/// Everything finished, most recent first; swipe to read again or delete.
struct FinishedView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.colors) private var colors
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        let finished = model.shelves().finished
        List {
            HStack {
                IconButton(glyph: .chevronLeft, label: "Back to Later") { dismiss() }.padding(.leading, -12)
                Spacer()
            }
            .frame(height: 44)
            .listRowStyle(colors)
            Text("Finished")
                .font(.plexSans(34, .semibold, relativeTo: .largeTitle))
                .tracking(-0.7)
                .foregroundStyle(colors.ink)
                .padding(.vertical, 12)
                .listRowStyle(colors)
            ForEach(finished) { item in
                VStack(alignment: .leading, spacing: 8) {
                    Text(item.title).font(.plexSans(16, .semibold, relativeTo: .headline)).foregroundStyle(colors.mute)
                    if let by = item.attribution {
                        Text(by).font(.plexSans(13, relativeTo: .footnote)).foregroundStyle(colors.mute)
                    }
                    if let summary = item.summary {
                        Barcode(TerrainModel(summary: summary, done: true), height: 12)
                    }
                }
                .padding(.vertical, 14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .overlay(alignment: .bottom) { Rectangle().fill(colors.faint).frame(height: 1) }
                .contentShape(.rect)
                .accessibilityElement(children: .combine)
                .swipeActions(edge: .leading) {
                    Button("Read again") { model.markFinished(item.id, false) }.tint(colors.ink)
                }
                .swipeActions(edge: .trailing) {
                    Button("Delete", role: .destructive) { model.remove([item.id]) }.tint(Color(FixedColor.delete))
                }
                .listRowStyle(colors)
            }
            if finished.isEmpty {
                Text("Swipe an item right to mark it finished.")
                    .font(.plexSans(14, relativeTo: .subheadline)).foregroundStyle(colors.mute)
                    .listRowStyle(colors)
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .background(colors.bg.ignoresSafeArea())
        .environment(\.defaultMinListRowHeight, 0)
        .hideNavigationBar()
        .navigationBarBackButtonHidden()
    }
}

extension View {
    /// A list row with the library's margins and no system chrome.
    func listRowStyle(_ colors: Colors) -> some View {
        listRowInsets(EdgeInsets(top: 0, leading: 24, bottom: 0, trailing: 24))
            .listRowSeparator(.hidden)
            .listRowBackground(colors.bg)
    }

    @ViewBuilder
    func hideNavigationBar() -> some View {
        #if os(iOS)
        toolbar(.hidden, for: .navigationBar)
        #else
        toolbar(.hidden)
        #endif
    }
}
