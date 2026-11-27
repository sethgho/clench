import { describe, expect, test } from "bun:test"
import { FakeClock } from "@clench/host/fake-clock"
import { docFrom, Session } from "../../host/test/engine"
import { ReaderController, type ReaderState } from "../src/controller"
import { actionForGesture, actionForKey, classifyGesture } from "../src/input"

const PROSE =
  "The first sentence is short. The second sentence follows it closely.\n\n" +
  "A new paragraph begins here. It has two sentences too.\n\n" +
  "Last paragraph, the end."

function setup(text = PROSE, options: { onClose?: () => void; onWpmChange?: (wpm: number) => void } = {}) {
  const session = new Session(docFrom(text), { wpm: 300 })
  const clock = new FakeClock()
  const controller = new ReaderController(session, { clock, ...options })
  const states: ReaderState[] = []
  controller.subscribe((s) => states.push(s))
  const word = () => {
    const f = controller.state.frame
    return f.before + f.pivot + f.after
  }
  return { session, clock, controller, states, word }
}

const key = (k: string, mods: Partial<{ ctrlKey: boolean; metaKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
})

describe("browser keys", () => {
  test("use the terminal reader's map", () => {
    expect(actionForKey(key(" "))).toEqual({ type: "toggle" })
    expect(actionForKey(key("ArrowLeft"))).toEqual({ type: "prevSentence" })
    expect(actionForKey(key("ArrowRight", { shiftKey: true }))).toEqual({ type: "nextParagraph" })
    expect(actionForKey(key("ArrowUp"))).toEqual({ type: "faster" })
    expect(actionForKey(key("j"))).toEqual({ type: "slower" })
    expect(actionForKey(key("]"))).toEqual({ type: "nextParagraph" })
    expect(actionForKey(key(","))).toEqual({ type: "stepBack" })
    expect(actionForKey(key("4"))).toEqual({ type: "jump", tenth: 4 })
    expect(actionForKey(key("?", { shiftKey: true }))).toEqual({ type: "help" })
    expect(actionForKey(key("Escape"))).toEqual({ type: "close" })
    expect(actionForKey(key("q"))).toEqual({ type: "quit" })
  })

  test("leave modified presses and unbound keys to the browser", () => {
    expect(actionForKey(key("c", { ctrlKey: true }))).toBeUndefined()
    expect(actionForKey(key("z", { ctrlKey: true }))).toBeUndefined()
    expect(actionForKey(key("r", { metaKey: true }))).toBeUndefined()
    expect(actionForKey(key("ArrowLeft", { altKey: true }))).toBeUndefined()
    expect(actionForKey(key("Tab"))).toBeUndefined()
    expect(actionForKey(key("Enter"))).toBeUndefined()
  })
})

describe("gestures", () => {
  test("tap toggles, horizontal swipes move by sentence, scrolls and slow drags do nothing", () => {
    expect(classifyGesture(2, -3, 120)).toBe("tap")
    expect(classifyGesture(2, 2, 2000)).toBeUndefined()
    expect(classifyGesture(-120, 10, 250)).toBe("swipeLeft")
    expect(classifyGesture(90, -20, 300)).toBe("swipeRight")
    expect(classifyGesture(20, 200, 300)).toBeUndefined()
    expect(classifyGesture(-120, 10, 1500)).toBeUndefined()
    expect(classifyGesture(30, 0, 100)).toBeUndefined()
    expect(actionForGesture("tap")).toEqual({ type: "toggle" })
    expect(actionForGesture("swipeLeft")).toEqual({ type: "nextSentence" })
    expect(actionForGesture("swipeRight")).toEqual({ type: "prevSentence" })
  })
})

describe("ReaderController", () => {
  test("opens on the ready screen, paused; space starts and words advance on the clock", () => {
    const { controller, clock, states, word } = setup()
    expect(controller.state.view).toBe("ready")
    expect(controller.state.frame.state).toBe("paused")
    expect(word()).toBe("The")
    controller.act({ type: "toggle" })
    expect(controller.state.view).toBe("reading")
    expect(controller.state.frame.state).toBe("playing")
    clock.tick(3000)
    expect(controller.state.frame.index).toBeGreaterThan(3)
    expect(states.length).toBeGreaterThan(3)
  })

  test("pausing publishes the paragraph around the current word", () => {
    const { controller, clock } = setup()
    controller.act({ type: "toggle" })
    controller.act({ type: "nextParagraph" })
    clock.tick(700)
    controller.act({ type: "toggle" })
    const ctx = controller.state.context!
    expect(ctx.before + ctx.token + ctx.after).toBe("A new paragraph begins here. It has two sentences too.")
    expect(ctx.token.length).toBeGreaterThan(0)
  })

  test("pause() stops playback without leaving the reading view and is a no-op when paused", () => {
    const { controller, clock } = setup()
    controller.pause()
    expect(controller.state.view).toBe("ready")
    controller.act({ type: "toggle" })
    clock.tick(500)
    controller.pause()
    expect(controller.state.frame.state).toBe("paused")
    expect(controller.state.view).toBe("reading")
    const index = controller.state.frame.index
    clock.tick(5000)
    expect(controller.state.frame.index).toBe(index)
    expect(clock.pending).toBe(0)
  })

  test("a background stall advances one word, not many", () => {
    const { controller, clock } = setup()
    controller.act({ type: "toggle" })
    const index = controller.state.frame.index
    clock.lateness = () => 60_000
    clock.tick(0)
    controller.act({ type: "faster" })
    clock.t += 60_000
    controller.pause()
    expect(controller.state.frame.index - index).toBeLessThanOrEqual(2)
  })

  test("help pauses and blocks other keys until closed", () => {
    const { controller, clock } = setup()
    controller.act({ type: "toggle" })
    clock.tick(400)
    controller.act({ type: "help" })
    expect(controller.state.view).toBe("help")
    expect(controller.state.frame.state).toBe("paused")
    const index = controller.state.frame.index
    controller.act({ type: "nextParagraph" })
    expect(controller.state.frame.index).toBe(index)
    controller.act({ type: "close" })
    expect(controller.state.view).toBe("reading")
  })

  test("speed changes are reported to the host", () => {
    const seen: number[] = []
    const { controller } = setup(PROSE, { onWpmChange: (w) => seen.push(w) })
    controller.act({ type: "faster" })
    controller.act({ type: "slower" })
    controller.act({ type: "slower" })
    controller.setWpm(450)
    expect(seen).toEqual([325, 300, 275, 450])
    expect(controller.state.view).toBe("ready")
  })

  test("q closes through the host and pauses first", () => {
    let closed = 0
    const { controller, clock } = setup(PROSE, { onClose: () => closed++ })
    controller.act({ type: "toggle" })
    clock.tick(300)
    controller.act({ type: "quit" })
    expect(closed).toBe(1)
    expect(controller.state.frame.state).toBe("paused")
  })

  test("finished after a straight read reports a rate; a jump makes it a non-straight run; restart begins a fresh one", () => {
    const { controller, clock } = setup("Just a few words to read.")
    controller.act({ type: "toggle" })
    clock.runUntilIdle()
    expect(controller.state.view).toBe("finished")
    expect(controller.state.straight).toBe(true)
    expect(controller.state.playedMs).toBeGreaterThan(0)
    controller.act({ type: "restart" })
    expect(controller.state.frame.state).toBe("playing")
    controller.act({ type: "stepForward" })
    clock.runUntilIdle()
    expect(controller.state.view).toBe("finished")
    expect(controller.state.straight).toBe(false)
    controller.act({ type: "toggle" })
    expect(controller.state.straight).toBe(true)
    expect(controller.state.frame.index).toBe(0)
  })

  test("dispose cancels the timer", () => {
    const { controller, clock } = setup()
    controller.act({ type: "toggle" })
    controller.dispose()
    expect(clock.pending).toBe(0)
  })
})
