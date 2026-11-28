import ClenchKit
import SwiftUI

/// The area that reads while touched: turns touches into ``ReadingGesture``
/// effects on the reader, and offers the same controls to VoiceOver.
struct ReadingSurface: View {
    let reader: ReaderModel
    @Binding var gesture: ReadingGesture
    var label = "Hold to read, or tap to keep reading. Drag up or down to change speed, sideways to move by sentence."
    var onWpm: () -> Void = {}
    var onStep: () -> Void = {}
    var onLift: () -> Void = {}

    var body: some View {
        Rectangle()
            .fill(Color.clear)
            .contentShape(.rect)
            .gesture(
                DragGesture(minimumDistance: 0, coordinateSpace: .local)
                    .onChanged { value in
                        if gesture.pressed {
                            apply(gesture.move(x: value.location.x, y: value.location.y))
                        } else {
                            apply(gesture.down(x: value.startLocation.x, y: value.startLocation.y, time: Self.now,
                                               playing: reader.isPlaying, wpm: Int(reader.frame.wpm)))
                        }
                    }
                    .onEnded { _ in
                        apply(gesture.up(time: Self.now))
                        onLift()
                    }
            )
            .accessibilityElement()
            .accessibilityLabel(label)
            .accessibilityValue(reader.sentence.map { $0.before + $0.token + $0.after } ?? "")
            .accessibilityAddTraits(.allowsDirectInteraction)
            .accessibilityAction { reader.toggle() }
            .accessibilityAdjustableAction { direction in
                switch direction {
                case .increment: reader.setWpm(Int(reader.frame.wpm) + 25)
                case .decrement: reader.setWpm(Int(reader.frame.wpm) - 25)
                @unknown default: break
                }
            }
            .accessibilityAction(named: "Next sentence") { reader.nextSentence() }
            .accessibilityAction(named: "Previous sentence") { reader.prevSentence() }
    }

    static var now: TimeInterval { ProcessInfo.processInfo.systemUptime }

    private func apply(_ effects: [ReadingGesture.Effect]) {
        for effect in effects {
            switch effect {
            case .play: reader.play()
            case .pause: reader.pause()
            case let .setWpm(wpm):
                reader.setWpm(wpm)
                onWpm()
            case let .stepSentence(delta):
                if delta > 0 { reader.nextSentence() } else { reader.prevSentence() }
                onStep()
            }
        }
    }
}
