import type { Clock } from "./clock"

/** A manual clock: time moves only through `tick`, firing due timers in order. */
export class FakeClock implements Clock {
  t = 0
  /** Extra delay added to every timer, to simulate a busy event loop. */
  lateness: () => number = () => 0
  private timers: Array<{ at: number; fn: () => void; id: number }> = []
  private seq = 0

  now(): number {
    return this.t
  }

  setTimeout(fn: () => void, ms: number): unknown {
    const id = ++this.seq
    this.timers.push({ at: this.t + Math.max(0, ms) + this.lateness(), fn, id })
    return id
  }

  clearTimeout(handle: unknown): void {
    this.timers = this.timers.filter((x) => x.id !== handle)
  }

  get pending(): number {
    return this.timers.length
  }

  tick(ms: number): void {
    const end = this.t + ms
    for (;;) {
      this.timers.sort((a, b) => a.at - b.at || a.id - b.id)
      const next = this.timers[0]
      if (!next || next.at > end) break
      this.timers.shift()
      this.t = next.at
      next.fn()
    }
    this.t = end
  }

  /** Runs timers until none are left; returns the time that took. */
  runUntilIdle(limitMs = 10 * 60 * 60 * 1000): number {
    const start = this.t
    while (this.timers.length > 0 && this.t - start < limitMs) {
      this.timers.sort((a, b) => a.at - b.at || a.id - b.id)
      const next = this.timers.shift()!
      this.t = Math.max(this.t, next.at)
      next.fn()
    }
    return this.t - start
  }
}
