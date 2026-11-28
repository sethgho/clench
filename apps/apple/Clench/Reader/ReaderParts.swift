import ClenchKit
import ClenchUI
import SwiftUI

/// The current word, split where the engine put its pivot: the halves get
/// equal room on either side, so the pivot letter sits on the centre line
/// from word to word. Two faint ticks mark the fixation point.
struct WordView: View {
    let frame: Frame
    let size: CGFloat
    @Environment(\.colors) private var colors

    var body: some View {
        let lineHeight = size * 1.5
        HStack(spacing: 0) {
            Text(frame.before)
                .frame(maxWidth: .infinity, alignment: .trailing)
            Text(frame.pivot)
                .font(.plexMonoFixed(size, .medium))
                .foregroundStyle(colors.accent)
            Text(frame.after)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.plexMonoFixed(size, frame.blockKind == .heading ? .semibold : .regular))
        .foregroundStyle(frame.isMarker ? colors.mute : colors.ink)
        .lineLimit(1)
        .minimumScaleFactor(0.4)
        .frame(height: lineHeight)
        .overlay(alignment: .top) { tick.offset(y: -(lineHeight * 0.27 + tickLength)) }
        .overlay(alignment: .bottom) { tick.offset(y: lineHeight * 0.27 + tickLength) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(frame.before + frame.pivot + frame.after)
    }

    private var tickLength: CGFloat { size >= 60 ? 14 : 10 }

    private var tick: some View {
        Rectangle().fill(colors.faint).frame(width: 2, height: tickLength)
    }
}

/// The sentence around the current word, from the engine: the word in ink,
/// the rest of the sentence muted.
struct SentenceText: View {
    let sentence: SentenceView
    let size: CGFloat
    @Environment(\.colors) private var colors

    var body: some View {
        (Text(sentence.before).foregroundStyle(colors.mute)
            + Text(sentence.token).foregroundStyle(colors.ink)
            + Text(sentence.after).foregroundStyle(colors.mute))
            .font(.plexSans(size, relativeTo: .body))
            .lineSpacing(size * 0.5)
            .multilineTextAlignment(.center)
    }
}

/// The paused word's whole block for the page view: read text muted, the
/// word in the accent, the rest in ink.
struct PageText: View {
    let context: Context
    @Environment(\.colors) private var colors

    var body: some View {
        (Text(context.before).foregroundStyle(colors.mute)
            + Text(context.token).foregroundStyle(colors.accent)
            + Text(context.after).foregroundStyle(colors.ink))
            .font(.plexSans(17, relativeTo: .body))
            .lineSpacing(9)
    }
}

/// The speed tape: ticks every 50 wpm over the engine's range, a major tick
/// every 100, and the accent marker with the speed beside it.
struct SpeedDial: View {
    let wpm: Int
    let height: CGFloat
    var majorWidth: CGFloat = 18
    var flipped = false
    @Environment(\.colors) private var colors
    private let scale = SpeedScale()

    var body: some View {
        let ticks = scale.ticks
        let row = height / CGFloat(ticks.count)
        ZStack(alignment: flipped ? .topTrailing : .topLeading) {
            VStack(alignment: flipped ? .trailing : .leading, spacing: 0) {
                ForEach(ticks, id: \.wpm) { t in
                    Rectangle()
                        .fill(t.major ? colors.ink : colors.mute)
                        .frame(width: t.major ? majorWidth : majorWidth / 2, height: t.major ? 2 : 1)
                        .frame(height: row)
                }
            }
            Text(verbatim: "\(scale.range.upperBound)")
                .font(.plexMono(11, relativeTo: .caption2))
                .foregroundStyle(colors.mute)
                .padding(.horizontal, 8)
                .offset(y: -20)
            Text(verbatim: "\(scale.range.lowerBound)")
                .font(.plexMono(11, relativeTo: .caption2))
                .foregroundStyle(colors.mute)
                .padding(.horizontal, 8)
                .offset(y: height + 6)
            HStack(spacing: 10) {
                if flipped { label }
                Rectangle().fill(colors.accent).frame(width: majorWidth * 2, height: 3)
                if !flipped { label }
            }
            .frame(height: 40)
            .offset(y: row / 2 + scale.fraction(wpm) * (height - row) - 20)
        }
        .frame(width: 150, height: height, alignment: flipped ? .topTrailing : .topLeading)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Speed dial, \(wpm) words per minute on a scale from \(scale.range.lowerBound) to \(scale.range.upperBound)")
    }

    private var label: some View {
        HStack(alignment: .firstTextBaseline, spacing: 5) {
            Text(verbatim: "\(wpm)").font(.plexMonoFixed(28, .medium)).foregroundStyle(colors.ink)
            Text("wpm").font(.plexMonoFixed(11)).foregroundStyle(colors.mute)
        }
        .monospacedDigit()
    }
}

/// The strip that rises from the bottom edge on a sentence step: which
/// sentence of the block this is, and a bar per sentence sized by its words.
struct SentenceStrip: View {
    let sentence: SentenceView
    var width: CGFloat = 260
    @Environment(\.colors) private var colors

    var body: some View {
        let total = max(1, sentence.words.reduce(0, +))
        let current = Int(sentence.indexInBlock)
        VStack(spacing: 10) {
            Text("Sentence \(current + 1) of \(sentence.words.count)")
                .font(.plexMono(12, relativeTo: .caption))
                .foregroundStyle(colors.ink)
            HStack(alignment: .bottom, spacing: 6) {
                ForEach(Array(sentence.words.enumerated()), id: \.offset) { i, words in
                    Rectangle()
                        .fill(i == current ? colors.accent : (i < current ? colors.faint : colors.mute))
                        .frame(width: max(2, (CGFloat(words) / CGFloat(total) * width).rounded()), height: i == current ? 3 : 2)
                        .frame(height: 14, alignment: .bottom)
                }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Sentence \(current + 1) of \(sentence.words.count)")
    }
}

/// The label beside the finger while scrubbing the map.
struct ScrubBubble: View {
    let section: String
    let percent: Int
    var fontSize: CGFloat = 12
    @Environment(\.colors) private var colors

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(section).font(.plexMono(fontSize, .medium, relativeTo: .caption)).foregroundStyle(colors.ink)
            Text("\(percent)%").font(.plexMono(fontSize, relativeTo: .caption)).foregroundStyle(colors.mute)
        }
        .lineLimit(1)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(colors.bg, in: .rect(cornerRadius: 8))
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(colors.faint, lineWidth: 1))
    }
}

/// Small icon buttons from the design, drawn as strokes.
enum Glyph {
    case chevronDown, chevronLeft, chevronRight, sliders, search, close

    @ViewBuilder
    func view(size: CGFloat = 20) -> some View {
        GlyphShape(glyph: self)
            .stroke(style: StrokeStyle(lineWidth: 1.6, lineCap: .round, lineJoin: .round))
            .frame(width: size, height: size)
    }
}

struct GlyphShape: Shape {
    let glyph: Glyph

    func path(in rect: CGRect) -> Path {
        let s = rect.width / 20
        func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: rect.minX + x * s, y: rect.minY + y * s) }
        var path = Path()
        switch glyph {
        case .chevronDown:
            path.move(to: p(5, 8)); path.addLine(to: p(10, 13)); path.addLine(to: p(15, 8))
        case .chevronLeft:
            path.move(to: p(12, 5)); path.addLine(to: p(7, 10)); path.addLine(to: p(12, 15))
        case .chevronRight:
            path.move(to: p(8, 5)); path.addLine(to: p(13, 10)); path.addLine(to: p(8, 15))
        case .sliders:
            path.move(to: p(3, 6)); path.addLine(to: p(10, 6))
            path.move(to: p(14, 6)); path.addLine(to: p(17, 6))
            path.move(to: p(3, 14)); path.addLine(to: p(6, 14))
            path.move(to: p(10, 14)); path.addLine(to: p(17, 14))
            path.addEllipse(in: CGRect(origin: p(10, 4), size: CGSize(width: 4 * s, height: 4 * s)))
            path.addEllipse(in: CGRect(origin: p(6, 12), size: CGSize(width: 4 * s, height: 4 * s)))
        case .search:
            path.addEllipse(in: CGRect(origin: p(3.5, 3.5), size: CGSize(width: 11 * s, height: 11 * s)))
            path.move(to: p(13.2, 13.2)); path.addLine(to: p(17, 17))
        case .close:
            path.move(to: p(4.4, 4.4)); path.addLine(to: p(15.6, 15.6))
            path.move(to: p(15.6, 4.4)); path.addLine(to: p(4.4, 15.6))
        }
        return path
    }
}

/// A 44-point icon button.
struct IconButton: View {
    let glyph: Glyph
    let label: String
    let action: () -> Void
    @Environment(\.colors) private var colors

    var body: some View {
        Button(action: action) {
            glyph.view()
                .foregroundStyle(colors.mute)
                .frame(width: 44, height: 44)
                .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
    }
}

/// "320 wpm · 39 min left", from the engine's exact remaining time.
func speedLine(wpm: UInt32, remainingMs: UInt64) -> String {
    "\(wpm) wpm · \(ReadingTime.minutes(remainingMs)) min left"
}
