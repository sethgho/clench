import { testRender } from "@opentui/solid"
import type { CapturedSpan } from "@opentui/core"
import { readFileSync } from "node:fs"
import { FakeClock } from "@clench/host/fake-clock"
import { extract, loadEngine, Session, type Document, type Settings } from "../src/engine"
import { makeTheme, type Theme } from "../src/theme"
import { App } from "../src/ui/app"

export { FakeClock }

export function docFrom(text: string, contentType = "text/plain"): Document {
  loadEngine()
  return extract(new TextEncoder().encode(text), contentType, "")
}

export function fixtureDoc(name: string): Document {
  loadEngine()
  const dir = `${import.meta.dir}/../../../fixtures`
  const bytes = readFileSync(`${dir}/${name}.html`)
  const url = readFileSync(`${dir}/${name}.url`, "utf8").trim()
  return extract(bytes, "text/html", url)
}

export interface Harness {
  t: Awaited<ReturnType<typeof testRender>>
  session: Session
  clock: FakeClock
  quits: () => number
  /** Renders and returns the screen as text. */
  screen: () => Promise<string>
  /** Presses a key, lets the input parser run, and renders. */
  press: (key: string, modifiers?: { shift?: boolean; ctrl?: boolean }) => Promise<string>
  arrow: (dir: "up" | "down" | "left" | "right", modifiers?: { shift?: boolean }) => Promise<string>
  word: () => string
}

export async function mount(
  doc: Document,
  opts: { width?: number; height?: number; settings?: Settings; theme?: Theme; stopProcess?: () => void; terminal?: { suspend(): Promise<void>; resume(): Promise<void> }; session?: Session } = {},
): Promise<Harness> {
  const session = opts.session ?? new Session(doc, opts.settings ?? { wpm: 300 })
  const clock = new FakeClock()
  let quits = 0
  const t = await testRender(
    () => <App doc={doc} session={session} theme={opts.theme ?? makeTheme(true)} clock={clock} onQuit={() => quits++} stopProcess={opts.stopProcess} terminal={opts.terminal} />,
    { width: opts.width ?? 80, height: opts.height ?? 24 },
  )
  const screen = async () => {
    await t.renderOnce()
    return t.captureCharFrame()
  }
  const settle = async () => {
    await Bun.sleep(5)
    return screen()
  }
  return {
    t,
    session,
    clock,
    quits: () => quits,
    screen,
    press: async (key, modifiers) => {
      t.mockInput.pressKey(key, modifiers)
      return settle()
    },
    arrow: async (dir, modifiers) => {
      t.mockInput.pressArrow(dir, modifiers)
      return settle()
    },
    word: () => {
      const f = session.frame()
      return f.before + f.pivot + f.after
    },
  }
}

/** The display column where each span of `row` starts. */
export function columns(spans: CapturedSpan[]): Array<{ span: CapturedSpan; col: number }> {
  let col = 0
  return spans.map((span) => {
    const at = col
    col += span.width
    return { span, col: at }
  })
}

export function sameColor(a: { toInts(): number[] }, b: { toInts(): number[] }): boolean {
  return a.toInts().join() === b.toInts().join()
}
