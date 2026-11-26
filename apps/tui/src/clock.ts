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

  dispose(): void {
    this.cancel()
    this.running = false
  }

  private credit(): boolean {
    const now = this.clock.now()
    const elapsed = now - this.last + this.carry
    const whole = Math.max(0, Math.floor(elapsed))
    this.carry = elapsed - whole
    this.played += now - this.last
    this.last = now
    return this.session.advance(whole)
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
