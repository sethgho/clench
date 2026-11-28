import Foundation

/// The part of a reading session the driver needs.
public protocol Playback: AnyObject {
    func advance(ms: UInt32) -> Bool
    func nextChangeIn() -> UInt32?
}

extension Session: Playback {}

/// The time source and timer the driver uses; swapped for a fake in tests.
public protocol DriverClock: Sendable {
    /// Monotonic time, ms.
    func now() -> Double
    /// Suspends for `ms`; throws when the task is cancelled.
    func sleep(ms: UInt32) async throws
}

/// The real clock: monotonic uptime and `Task.sleep`.
public struct SystemClock: DriverClock {
    public init() {}

    public func now() -> Double {
        Double(DispatchTime.now().uptimeNanoseconds) / 1_000_000
    }

    public func sleep(ms: UInt32) async throws {
        try await Task.sleep(for: .milliseconds(Int(ms)), tolerance: .milliseconds(1))
    }
}

/// Drives a session from the host's clock, the same way the JavaScript
/// hosts do (`packages/host/src/clock.ts`). It sleeps until the session's
/// next change and on every wake reports the real time that passed, so a
/// late timer shortens the next word instead of delaying everything after
/// it. Sub-millisecond remainders are carried so no time is lost to
/// rounding.
@MainActor
public final class ReaderDriver {
    private let playback: Playback
    private let clock: DriverClock
    private let onChange: @MainActor () -> Void

    private var timer: Task<Void, Never>?
    private var running = false
    private var last = 0.0
    private var carry = 0.0

    /// Wall-clock time spent playing, ms.
    public private(set) var playedMs = 0.0

    public init(playback: Playback, clock: DriverClock = SystemClock(), onChange: @escaping @MainActor () -> Void) {
        self.playback = playback
        self.clock = clock
        self.onChange = onChange
    }

    /// Applies a user control: credits the time that passed since the last
    /// wake, runs `control`, then reschedules and reports a change.
    public func control(_ control: () -> Void) {
        if running { _ = credit() }
        control()
        schedule()
        onChange()
    }

    public func resetPlayed() {
        playedMs = 0
    }

    public func stop() {
        cancel()
        running = false
    }

    /// Handles a timer firing. Internal so tests can fire it by hand.
    func wake() {
        timer = nil
        let changed = credit()
        schedule()
        if changed { onChange() }
    }

    /// Reports the time since the last wake to the session, never more than
    /// the reader could have watched. If the process was suspended or
    /// starved, the excess is dropped: the current word completes and the
    /// next gets at most half its own time, so no word is skipped unseen.
    /// Only the time actually credited counts as time spent playing.
    private func credit() -> Bool {
        let now = clock.now()
        let real = now - last
        let elapsed = real + carry
        let whole = max(0, elapsed.rounded(.down))
        last = now
        let wholeMs = UInt32(min(whole, Double(UInt32.max)))
        let due = playback.nextChangeIn()
        guard let due, wholeMs > due else {
            carry = elapsed - whole
            playedMs += real
            return playback.advance(ms: wholeMs)
        }
        var changed = playback.advance(ms: due)
        var credited = due
        let next = playback.nextChangeIn()
        let late = next.map { min(wholeMs - due, $0 / 2) } ?? 0
        if late > 0 {
            changed = playback.advance(ms: late) || changed
            credited += late
        }
        let dropped = wholeMs - due > late
        carry = dropped ? 0 : elapsed - whole
        playedMs += dropped ? Double(credited) : real
        return changed
    }

    private func schedule() {
        cancel()
        guard let wait = playback.nextChangeIn() else {
            running = false
            return
        }
        if !running {
            running = true
            last = clock.now()
            carry = 0
        }
        let clock = self.clock
        timer = Task { [weak self] in
            do {
                try await clock.sleep(ms: wait)
            } catch {
                return
            }
            guard !Task.isCancelled else { return }
            self?.wake()
        }
    }

    private func cancel() {
        timer?.cancel()
        timer = nil
    }
}
