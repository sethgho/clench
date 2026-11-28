import ClenchKit
import SwiftUI

/// Placeholder reader: one word at a time, pivot letter highlighted at a
/// fixed point, with basic controls. Everything shown comes from the
/// engine's frames; the final design replaces this view.
struct ReaderView: View {
    let reader: ReaderModel

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            WordView(frame: reader.frame)
            if let context = reader.context {
                ContextView(context: context)
            }
            Spacer()
            ProgressView(value: Double(reader.frame.progress))
            controls
        }
        .padding()
        .onDisappear { reader.stop() }
    }

    private var controls: some View {
        HStack(spacing: 20) {
            Button("Previous sentence", systemImage: "backward.end") { reader.prevSentence() }
                .keyboardShortcut(.leftArrow, modifiers: [])
            Button(reader.state == .playing ? "Pause" : "Play",
                   systemImage: reader.state == .playing ? "pause.fill" : "play.fill") { reader.toggle() }
                .keyboardShortcut(.space, modifiers: [])
            Button("Next sentence", systemImage: "forward.end") { reader.nextSentence() }
                .keyboardShortcut(.rightArrow, modifiers: [])
            Divider().frame(height: 20)
            Button("Slower", systemImage: "minus") { reader.nudgeWpm(-25) }
                .keyboardShortcut(.downArrow, modifiers: [])
            Text("\(reader.frame.wpm) wpm").monospacedDigit()
            Button("Faster", systemImage: "plus") { reader.nudgeWpm(25) }
                .keyboardShortcut(.upArrow, modifiers: [])
        }
        .labelStyle(.iconOnly)
        .buttonStyle(.borderless)
    }
}

/// The current word, split around its pivot. With a monospaced font the two
/// halves get equal space either side of the pivot, so the pivot letter
/// stays on the same spot from word to word.
struct WordView: View {
    let frame: Frame

    var body: some View {
        HStack(spacing: 0) {
            Text(frame.before)
                .frame(maxWidth: .infinity, alignment: .trailing)
            Text(frame.pivot)
                .foregroundStyle(.red)
            Text(frame.after)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.system(size: 40, weight: frame.blockKind == .heading ? .bold : .regular, design: .monospaced))
        .foregroundStyle(frame.isMarker ? .secondary : .primary)
        .lineLimit(1)
        .minimumScaleFactor(0.5)
        .overlay(alignment: .top) {
            Rectangle().frame(width: 2, height: 10).offset(y: -16)
        }
        .overlay(alignment: .bottom) {
            Rectangle().frame(width: 2, height: 10).offset(y: 16)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(frame.before + frame.pivot + frame.after)
    }
}

/// The paused word's block with the word highlighted.
struct ContextView: View {
    let context: Context

    var body: some View {
        (Text(context.before) + Text(context.token).bold().foregroundStyle(.red) + Text(context.after))
            .font(.body)
            .foregroundStyle(.secondary)
            .frame(maxWidth: 640)
            .lineLimit(8)
    }
}
