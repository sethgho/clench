/** The time source and timers the driver uses; swapped for a fake in tests. */
export interface Clock {
  now(): number
  setTimeout(callback: () => void, ms: number): unknown
  clearTimeout(handle: unknown): void
}

export const realClock: Clock = {
  now: () => performance.now(),
  setTimeout: (callback, ms) => setTimeout(callback, ms),
  clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
}

/** The part of a reading session the driver needs. */
export interface Playback {
  advance(ms: number): boolean
  nextChangeIn(): number | undefined
}

/**
 * Drives a session from the wall clock. It sleeps until the session's next
 * change and on every wake reports the real time that passed, so a late
 * timer shortens the next word instead of delaying everything after it.
 * Sub-millisecond remainders are carried so no time is lost to rounding.
 */
export class Driver {
  private timer: unknown = undefined
  private running = false
  private last = 0
  private carry = 0
  private played = 0

  constructor(
    private readonly session: Playback,
    private readonly clock: Clock,
    private readonly onChange: () => void,
  ) {}

  /**
   * Applies a user control: credits the time that passed since the last
   * wake, runs `control`, then reschedules and reports a change.
   */
  control(control: () => void): void {
    if (this.running) this.credit()
    control()
    this.schedule()
    this.onChange()
  }

  /** Wall-clock time spent playing, ms. */
  get playedMs(): number {
    return this.played
  }

  /** Starts counting played time again from zero. */
  resetPlayed(): void {
    this.played = 0
  }

  dispose(): void {
    this.cancel()
    this.running = false
  }

  /**
   * Reports the time since the last wake to the session, never more than the
   * reader could have watched. If the process was suspended or starved, the
   * excess is dropped: the current word completes and the next gets at most
   * half its own time, so no word is skipped unseen and playback resumes
   * where it left off. Ordinary timer lateness is far below that bound, so
   * it still shortens the following word and drift does not accumulate.
   * Only the time actually credited counts as time spent playing.
   */
  private credit(): boolean {
    const now = this.clock.now()
    const real = now - this.last
    const elapsed = real + this.carry
    const whole = Math.max(0, Math.floor(elapsed))
    this.last = now
    const due = this.session.nextChangeIn()
    if (due === undefined || whole <= due) {
      this.carry = elapsed - whole
      this.played += real
      return this.session.advance(whole)
    }
    let changed = this.session.advance(due)
    let credited = due
    const next = this.session.nextChangeIn()
    const late = next === undefined ? 0 : Math.min(whole - due, Math.floor(next / 2))
    if (late > 0) {
      changed = this.session.advance(late) || changed
      credited += late
    }
    const dropped = whole - due > late
    this.carry = dropped ? 0 : elapsed - whole
    this.played += dropped ? credited : real
    return changed
  }

  private schedule(): void {
    this.cancel()
    const wait = this.session.nextChangeIn()
    if (wait === undefined) {
      this.running = false
      return
    }
    if (!this.running) {
      this.running = true
      this.last = this.clock.now()
      this.carry = 0
    }
    this.timer = this.clock.setTimeout(this.wake, wait)
  }

  private cancel(): void {
    if (this.timer !== undefined) this.clock.clearTimeout(this.timer)
    this.timer = undefined
  }

  private readonly wake = (): void => {
    this.timer = undefined
    const changed = this.credit()
    this.schedule()
    if (changed) this.onChange()
  }
}
