import ClenchKit
import SwiftUI

/// The engine's document map ready to draw: ticks with their relative size,
/// and which one is current. Ticks before the current one are read.
public struct TerrainModel: Hashable, Sendable {
    public struct Mark: Hashable, Sendable {
        public var heading: Bool
        /// 0 to 1, relative to the largest non-heading tick.
        public var weight: Double

        public init(heading: Bool, weight: Double) {
            self.heading = heading
            self.weight = weight
        }
    }

    public enum State: Sendable { case read, current, unread }

    public var marks: [Mark]
    /// The current tick, if one is shown.
    public var current: Int?
    /// Every tick is read (the item is finished).
    public var finished: Bool

    public init(marks: [Mark], current: Int?, finished: Bool = false) {
        self.marks = marks
        self.current = current
        self.finished = finished
    }

    /// The live map of a reader.
    public init(ticks: [TerrainTick], position: ReadingPosition) {
        self.init(marks: ticks.map { Mark(heading: $0.heading, weight: Double($0.weight) / 1000) },
                  current: position.tick.map(Int.init), finished: position.tick == nil)
    }

    /// The stored map of a library item. Items not started show no current
    /// tick, as in a list of things to read.
    public init(summary: ItemSummary, done: Bool = false) {
        let marks = summary.ticks.map { Mark(heading: $0.heading, weight: Double($0.weight) / 1000) }
        if done {
            self.init(marks: marks, current: nil, finished: true)
        } else {
            self.init(marks: marks, current: summary.started ? summary.mark.tick : nil,
                      finished: summary.mark.tick == nil && summary.started)
        }
    }

    public func state(_ i: Int) -> State {
        if finished { return .read }
        guard let current else { return .unread }
        if i == current { return .current }
        return i < current ? .read : .unread
    }

    public func color(_ i: Int, _ c: Colors) -> Color {
        switch state(i) {
        case .current: c.accent
        case .read: c.faint
        case .unread: marks[i].heading ? c.ink : c.mute
        }
    }
}

/// The document map drawn across, as on library rows: one bar per tick,
/// headings full height, others by size.
public struct Barcode: View {
    let model: TerrainModel
    let height: CGFloat
    /// Space between bars; nil spreads the bars over the width.
    let gap: CGFloat?
    let headingWidth: CGFloat
    @Environment(\.colors) private var colors

    public init(_ model: TerrainModel, height: CGFloat, gap: CGFloat? = 2, headingWidth: CGFloat = 2) {
        self.model = model
        self.height = height
        self.gap = gap
        self.headingWidth = headingWidth
    }

    public var body: some View {
        Canvas { ctx, size in
            let widths = model.marks.indices.map(width)
            let total = widths.reduce(0, +)
            let n = CGFloat(max(1, model.marks.count - 1))
            let space = gap.map { min($0, max(0, (size.width - total) / n)) } ?? max(0, (size.width - total) / n)
            var x: CGFloat = 0
            for i in model.marks.indices {
                let h = barHeight(i)
                ctx.fill(Path(CGRect(x: x, y: size.height - h, width: widths[i], height: h)),
                         with: .color(model.color(i, colors)))
                x += widths[i] + space
            }
        }
        .frame(height: height)
        .accessibilityHidden(true)
    }

    private func width(_ i: Int) -> CGFloat {
        if model.state(i) == .current { return 3 }
        return model.marks[i].heading ? headingWidth : 1
    }

    private func barHeight(_ i: Int) -> CGFloat {
        let m = model.marks[i]
        if m.heading || model.state(i) == .current { return height }
        return (height * (0.3 + 0.45 * m.weight)).rounded()
    }
}

/// Sizes of the vertical map's marks.
public struct EdgeMapStyle: Sendable {
    public var currentWidth: CGFloat
    public var currentHeight: CGFloat
    public var headingWidth: CGFloat
    public var minWidth: CGFloat
    public var maxWidth: CGFloat

    public static let phone = EdgeMapStyle(currentWidth: 34, currentHeight: 3, headingWidth: 22, minWidth: 5, maxWidth: 14)
    public static let mac = EdgeMapStyle(currentWidth: 54, currentHeight: 4, headingWidth: 36, minWidth: 8, maxWidth: 22)
}

/// The document map drawn down the right edge of the reader: one row per
/// tick, marks right-aligned, sized by the engine's weights. The current
/// mark glides to its new row (200 ms).
public struct EdgeMap: View {
    let model: TerrainModel
    let style: EdgeMapStyle
    @Environment(\.colors) private var colors
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(_ model: TerrainModel, style: EdgeMapStyle = .phone) {
        self.model = model
        self.style = style
    }

    public var body: some View {
        GeometryReader { geo in
            let row = geo.size.height / CGFloat(max(1, model.marks.count))
            ZStack(alignment: .topTrailing) {
                Canvas { ctx, size in
                    for i in model.marks.indices where model.state(i) != .current {
                        let m = model.marks[i]
                        let w = m.heading ? style.headingWidth : style.minWidth + (style.maxWidth - style.minWidth) * m.weight
                        let h: CGFloat = m.heading ? 2 : 1
                        let y = (CGFloat(i) + 0.5) * row - h / 2
                        ctx.fill(Path(CGRect(x: size.width - w, y: y, width: w, height: h)),
                                 with: .color(model.color(i, colors)))
                    }
                }
                if let current = model.current, !model.finished, model.marks.indices.contains(current) {
                    Rectangle()
                        .fill(colors.accent)
                        .frame(width: style.currentWidth, height: style.currentHeight)
                        .offset(y: (CGFloat(current) + 0.5) * row - style.currentHeight / 2)
                        .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: current)
                }
            }
        }
    }

    /// The tick under a point `y` points from the top of a map `height` tall.
    public static func tick(at y: CGFloat, height: CGFloat, count: Int) -> Int? {
        guard count > 0, height > 0 else { return nil }
        let f = min(max(y / height, 0), 0.999_999)
        return Int(f * CGFloat(count))
    }
}

/// "clench" with the accent on the e.
public struct Wordmark: View {
    let size: CGFloat
    @Environment(\.colors) private var colors

    public init(size: CGFloat = 15) {
        self.size = size
    }

    public var body: some View {
        (Text("cl") + Text("e").foregroundStyle(colors.accent) + Text("nch"))
            .font(.plexMono(size, .medium, relativeTo: .subheadline))
            .tracking(size * 0.02)
            .foregroundStyle(colors.ink)
            .accessibilityLabel("clench")
    }
}
