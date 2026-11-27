import { describe, expect, test } from "bun:test"
import { Driver, realClock } from "../src/clock"
import { FakeClock } from "../src/fake-clock"
import { docFrom, Session } from "./engine"

const TEXT = Array.from({ length: 40 }, (_, i) => `Sentence number ${i} has a few words, then ends.`).join(" ")

function setup(lateness?: () => number) {
  const session = new Session(docFrom(TEXT), { wpm: 600 })
  const clock = new FakeClock()
  if (lateness) clock.lateness = lateness
  let changes = 0
  const driver = new Driver(session, clock, () => changes++)
  return { session, clock, driver, changes: () => changes }
}

/** The engine's own playing time for the whole script, with ease-in. */
function engineTotal(): number {
  const s = new Session(docFrom(TEXT), { wpm: 600 })
  s.play()
  let total = 0
  for (let w = s.nextChangeIn(); w !== undefined; w = s.nextChangeIn()) {
    s.advance(w)
    total += w
  }
  return total
}

describe("Driver", () => {
  test("plays to the end in exactly the engine's time with punctual timers", () => {
    const { session, clock, driver, changes } = setup()
    driver.control(() => session.play())
    const took = clock.runUntilIdle()
    expect(session.state).toBe("finished")
    expect(took).toBe(engineTotal())
    expect(changes()).toBe(session.tokenCount + 1)
    expect(driver.playedMs).toBe(took)
  })

  test("late timers do not accumulate drift", () => {
    let seed = 7
    const random = () => ((seed = (seed * 1103515245 + 12345) % 2 ** 31) / 2 ** 31) * 9
    const { session, clock, driver } = setup(random)
    driver.control(() => session.play())
    const took = clock.runUntilIdle()
    expect(session.state).toBe("finished")
    const ideal = engineTotal()
    // Each wake is up to 9 ms late, over hundreds of words; only the last
    // wake's lateness (plus rounding) may show in the total.
    expect(took).toBeGreaterThanOrEqual(ideal)
    expect(took - ideal).toBeLessThan(10)
  })

  test("fractional clock readings are carried, not dropped", () => {
    const { session, clock, driver } = setup(() => 0.4)
    driver.control(() => session.play())
    const took = clock.runUntilIdle()
    // Dropping the 0.4 ms remainders would add 0.4 ms per word.
    expect(session.tokenCount).toBeGreaterThan(100)
    expect(took - engineTotal()).toBeLessThan(2)
  })

  test("pausing stops the timer and stops counting played time", () => {
    const { session, clock, driver } = setup()
    driver.control(() => session.play())
    clock.tick(1000)
    driver.control(() => session.pause())
    expect(clock.pending).toBe(0)
    const index = session.index
    clock.tick(5000)
    expect(session.index).toBe(index)
    expect(driver.playedMs).toBe(1000)
  })

  test("a control credits the time that passed before it", () => {
    const { session, clock, driver } = setup()
    driver.control(() => session.play())
    const first = session.nextChangeIn()!
    clock.tick(first - 1)
    expect(session.index).toBe(0)
    clock.t += 5 // time passes without a timer firing (a busy loop)
    driver.control(() => session.setWpm(600))
    expect(session.index).toBe(1)
  })

  test("a huge stall advances exactly one word and counts only credited time", () => {
    const { session, clock, driver } = setup()
    driver.control(() => session.play())
    const first = session.nextChangeIn()!
    clock.lateness = () => 20_000
    driver.control(() => session.pause())
    driver.control(() => session.play())
    clock.tick(first + 20_000)
    expect(session.index).toBe(1)
    expect(driver.playedMs).toBeLessThan(first + 1000)
  })

  test("a stall before a key press does not skip words either", () => {
    const { session, clock, driver } = setup()
    driver.control(() => session.play())
    clock.t += 20_000
    driver.control(() => session.pause())
    expect(session.index).toBe(1)
    expect(driver.playedMs).toBeLessThan(1000)
  })

  test("dispose cancels the pending wake", () => {
    const { session, clock, driver } = setup()
    driver.control(() => session.play())
    driver.dispose()
    expect(clock.pending).toBe(0)
  })
})

test("on the real clock, wall time matches the engine's schedule", async () => {
  const plan = new Session(docFrom(TEXT.slice(0, 250)), { wpm: 1000 })
  plan.play()
  let expected = 0
  for (let w = plan.nextChangeIn(); w !== undefined; w = plan.nextChangeIn()) {
    plan.advance(w)
    expected += w
  }
  expect(plan.tokenCount).toBeGreaterThan(40)
  // A loaded machine can delay one run by an arbitrary amount, but a Driver
  // that accumulates drift (a millisecond or more per word over 50+ words)
  // is late on every run, so the best of a few attempts must be close.
  const excess: number[] = []
  for (let attempt = 0; attempt < 4; attempt++) {
    const s = new Session(docFrom(TEXT.slice(0, 250)), { wpm: 1000 })
    const { promise, resolve } = Promise.withResolvers<void>()
    const driver = new Driver(s, realClock, () => {
      if (s.state === "finished") resolve()
    })
    const start = performance.now()
    driver.control(() => s.play())
    await promise
    const wall = performance.now() - start
    expect(wall).toBeGreaterThanOrEqual(expected - 1)
    excess.push(wall - expected)
    if (wall - expected < 30) break
  }
  expect(Math.min(...excess)).toBeLessThan(30)
}, 30_000)
