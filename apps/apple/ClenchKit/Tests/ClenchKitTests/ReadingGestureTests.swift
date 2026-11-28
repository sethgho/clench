import Testing
@testable import ClenchKit

@Suite struct ReadingGestureTests {
    @Test func holdPlaysAndLiftPauses() {
        var g = ReadingGesture()
        #expect(g.down(x: 100, y: 400, time: 0, playing: false, wpm: 320) == [.play])
        #expect(g.up(time: 1.0) == [.pause])
        #expect(!g.latched)
    }

    @Test func aQuickTapLatchesAndTheNextTapPauses() {
        var g = ReadingGesture()
        #expect(g.down(x: 100, y: 400, time: 0, playing: false, wpm: 320) == [.play])
        #expect(g.up(time: 0.1) == [])
        #expect(g.latched)
        #expect(g.down(x: 100, y: 400, time: 2, playing: true, wpm: 320) == [])
        #expect(g.up(time: 2.05) == [.pause])
        #expect(!g.latched)
    }

    @Test func aSlowTapIsAHold() {
        var g = ReadingGesture()
        _ = g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320)
        #expect(g.up(time: 0.3) == [.pause])
    }

    @Test func verticalDragSetsSpeedSnappedToFive() {
        var g = ReadingGesture()
        _ = g.down(x: 100, y: 400, time: 0, playing: false, wpm: 320)
        #expect(g.move(x: 102, y: 395) == [], "below the axis lock")
        #expect(g.move(x: 101, y: 340) == [.setWpm(440)])
        #expect(g.axis == .vertical)
        #expect(g.move(x: 160, y: 341) == [.setWpm(440)], "the axis stays locked")
        #expect(g.move(x: 100, y: 398) == [.setWpm(325)])
        #expect(g.up(time: 3) == [.pause])
    }

    @Test func speedIsClampedToTheEngineRange() {
        var g = ReadingGesture()
        _ = g.down(x: 0, y: 1000, time: 0, playing: false, wpm: 1400)
        #expect(g.move(x: 0, y: 0) == [.setWpm(1500)])
        #expect(g.move(x: 0, y: 1250) == [.setWpm(900)])
        #expect(g.move(x: 0, y: 9000) == [.setWpm(100)])
    }

    @Test func horizontalDragStepsOneSentencePer36Points() {
        var g = ReadingGesture()
        _ = g.down(x: 100, y: 400, time: 0, playing: false, wpm: 320)
        #expect(g.move(x: 130, y: 401) == [])
        #expect(g.axis == .horizontal)
        #expect(g.move(x: 137, y: 400) == [.stepSentence(1)])
        #expect(g.move(x: 160, y: 400) == [])
        #expect(g.move(x: 173, y: 400) == [.stepSentence(1)])
        #expect(g.move(x: 120, y: 400) == [.stepSentence(-1)])
    }

    @Test func draggingDuringALatchedReadKeepsReading() {
        var g = ReadingGesture()
        _ = g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320)
        _ = g.up(time: 0.1)
        #expect(g.down(x: 0, y: 0, time: 1, playing: true, wpm: 320) == [])
        #expect(g.move(x: 0, y: -50) == [.setWpm(420)])
        #expect(g.up(time: 2) == [])
        #expect(g.latched)
    }

    @Test func aLatchEndsWhenReadingStoppedElsewhere() {
        var g = ReadingGesture()
        _ = g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320)
        _ = g.up(time: 0.1)
        // The engine finished, so the session is no longer playing.
        #expect(g.down(x: 0, y: 0, time: 5, playing: false, wpm: 320) == [.play])
        #expect(g.up(time: 6) == [.pause])
    }

    @Test func holdOnlyModeNeverLatches() {
        var config = ReadingGesture.Config()
        config.touch = .hold
        var g = ReadingGesture(config: config)
        _ = g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320)
        #expect(g.up(time: 0.05) == [.pause])
        #expect(!g.latched)
    }

    @Test func tapOnlyModeTogglesOnTap() {
        var config = ReadingGesture.Config()
        config.touch = .tap
        var g = ReadingGesture(config: config)
        #expect(g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320) == [])
        #expect(g.up(time: 0.05) == [.play])
        #expect(g.down(x: 0, y: 0, time: 1, playing: true, wpm: 320) == [])
        #expect(g.up(time: 1.05) == [.pause])
        #expect(g.down(x: 0, y: 0, time: 2, playing: false, wpm: 320) == [])
        #expect(g.up(time: 3) == [], "a long press does nothing")
    }

    @Test func cancelStopsReading() {
        var g = ReadingGesture()
        _ = g.down(x: 0, y: 0, time: 0, playing: false, wpm: 320)
        #expect(g.cancel() == [.pause])
        #expect(g.cancel() == [])
        #expect(g.up(time: 1) == [])
    }

    @Test func speedScaleCoversTheEngineRange() {
        let scale = SpeedScale()
        let ticks = scale.ticks
        #expect(ticks.count == 29)
        #expect(ticks.first == .init(wpm: 1500, major: true))
        #expect(ticks[1] == .init(wpm: 1450, major: false))
        #expect(ticks.last == .init(wpm: 100, major: true))
        #expect(scale.fraction(1500) == 0)
        #expect(scale.fraction(100) == 1)
        #expect(scale.fraction(800) == 0.5)
        #expect(scale.fraction(5000) == 0)
    }
}
