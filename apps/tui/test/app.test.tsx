import { afterEach, describe, expect, test } from "bun:test"
import { RGBA, TextAttributes } from "@opentui/core"
import { testRender } from "@opentui/solid"
import { Session } from "../src/engine"
import { makeTheme } from "../src/theme"
import { pivotColumn, Reticle } from "../src/ui/reticle"
import { columns, docFrom, fixtureDoc, mount, sameColor, type Harness } from "./helpers"

const theme = makeTheme(true)
const PROSE =
  "The first sentence is short. The second sentence follows it closely.\n\n" +
  "A new paragraph begins here. It has two sentences too.\n\n" +
  "Last paragraph, the end."

let open: Harness | undefined
afterEach(() => {
  open?.t.renderer.destroy()
  open = undefined
})

async function app(...args: Parameters<typeof mount>): Promise<Harness> {
  open = await mount(...args)
  return open
}

/** Where the pivot span of row `row` starts, and its text. */
function pivotAt(t: Harness["t"], row: number, style = theme.pivot) {
  const line = t.captureSpans().lines[row]!
  const hit = columns(line.spans).find(({ span }) => sameColor(span.fg, style.fg) && span.text.trim() !== "")
  return hit && { col: hit.col, text: hit.span.text, attributes: hit.span.attributes }
}

describe("reticle", () => {
  const words = ["a", "reading", "naïve", "Größe", "日本語の文章", "👩‍👩‍👧‍👦family", "😀😀😀😀wide", "Supercalifragilisticexpialidocious"]
  for (const width of [80, 41, 20]) {
    test(`pivot stays on the centre column at width ${width}`, async () => {
      const session = new Session(docFrom(words.join(" ")))
      const [frame, setFrame] = (await import("solid-js")).createSignal(session.frame())
      const t = await testRender(() => <Reticle frame={frame()} width={width} top={0} theme={theme} />, { width, height: 3 })
      try {
        for (let i = 0; i < session.tokenCount; i++) {
          session.seek(i)
          const f = session.frame()
          setFrame(f)
          await t.renderOnce()
          const hit = pivotAt(t, 1)!
          expect(hit.text).toBe(f.pivot)
          expect(hit.col).toBe(pivotColumn(width))
          // The guide ticks sit on the same column.
          const top = columns(t.captureSpans().lines[0]!.spans)
          const tick = top.find(({ span }) => span.text.includes("┬"))!
          expect(tick.col + Bun.stringWidth(tick.span.text.slice(0, tick.span.text.indexOf("┬")))).toBe(pivotColumn(width))
          // Text before the pivot ends right against it (clipped from the left if needed).
          const row = t.captureCharFrame().split("\n")[1]!
          if (f.before && Bun.stringWidth(f.before) <= pivotColumn(width)) {
            const lastBefore = [...f.before].at(-1)!
            expect(row).toContain(lastBefore + f.pivot)
          }
        }
      } finally {
        t.renderer.destroy()
      }
    })
  }

  test("markers look different from author text", async () => {
    const doc = docFrom("Intro words.\n\n```\nlet x = 1;\n```\n\nOutro.", "text/markdown")
    const session = new Session(doc)
    session.nextParagraph()
    const f = session.frame()
    expect(f.is_marker).toBe(true)
    const t = await testRender(() => <Reticle frame={f} width={40} top={0} theme={theme} />, { width: 40, height: 3 })
    await t.renderOnce()
    const spans = t.captureSpans().lines[1]!.spans.filter((s) => s.text.trim() !== "")
    expect(spans.map((s) => s.text).join("")).toBe("[code]")
    for (const s of spans) {
      expect(sameColor(s.fg, theme.marker.fg)).toBe(true)
      expect(s.attributes & TextAttributes.ITALIC).toBeTruthy()
    }
    t.renderer.destroy()
  })

  for (const color of [true, false]) {
    test(`heading words are bold and body words are not (${color ? "colour" : "NO_COLOR"})`, async () => {
      const h = await app(docFrom("# Big Heading\n\nBody words follow here.", "text/markdown"), { theme: makeTheme(color) })
      await h.press(" ")
      const row = (await h.screen()).split("\n").findIndex((l) => l.includes("┬")) + 1
      expect(h.session.frame().block_kind).toBe("heading")
      expect(h.word()).toBe("Big")
      const headSpans = h.t.captureSpans().lines[row]!.spans.filter((x) => x.text.trim() !== "")
      const nonPivot = headSpans.filter((x) => x.text !== h.session.frame().pivot)
      expect(nonPivot.length).toBeGreaterThan(0)
      for (const x of headSpans) expect(x.attributes & TextAttributes.BOLD).toBeTruthy()
      await h.press("]")
      expect(h.session.frame().block_kind).toBe("paragraph")
      const bodySpans = h.t.captureSpans().lines[row]!.spans.filter((x) => x.text.trim() !== "" && x.text !== h.session.frame().pivot)
      for (const x of bodySpans) expect(x.attributes & TextAttributes.BOLD).toBeFalsy()
    })
  }

  test("the first word of a new block gets heavier guide lines; later words do not", async () => {
    const h = await app(docFrom(PROSE))
    await h.press(" ")
    expect(await h.screen()).not.toContain("━━━")
    await h.press("]")
    expect((await h.screen()).split("\n").some((l) => l.includes("━━━┬━━━"))).toBe(true)
    await h.press(".")
    expect((await h.screen()).split("\n").some((l) => l.includes("━━━┬"))).toBe(false)
  })

  test("NO_COLOR keeps the pivot distinguishable by attributes alone", async () => {
    const plain = makeTheme(false)
    const f = new Session(docFrom("reading")).frame()
    const t = await testRender(() => <Reticle frame={f} width={40} top={0} theme={plain} />, { width: 40, height: 3 })
    await t.renderOnce()
    const pivot = t.captureSpans().lines[1]!.spans.find((s) => s.text === f.pivot)!
    expect(sameColor(pivot.fg, RGBA.defaultForeground())).toBe(true)
    expect(pivot.attributes & TextAttributes.BOLD).toBeTruthy()
    expect(pivot.attributes & TextAttributes.UNDERLINE).toBeTruthy()
    t.renderer.destroy()
  })
})

describe("screens", () => {
  test("ready screen shows title, byline, word count and estimate, and starts paused", async () => {
    const h = await app(fixtureDoc("01-iankduncan"))
    const s = await h.screen()
    expect(s).toContain("Smart People Who Give a Shit Use Tools")
    expect(s).toContain("Ian Duncan")
    expect(s).toContain("1,815 words · about 7 min at 300 wpm")
    expect(s).toContain("space start")
    expect(s).not.toContain("incomplete")
    expect(h.session.state).toBe("paused")
    h.clock.tick(5000)
    expect(h.session.index).toBe(0)
  })

  test("ready screen warns about partial documents", async () => {
    const doc = fixtureDoc("15-spa-bsky")
    expect(doc.partial).toBe(true)
    const h = await app(doc)
    const s = await h.screen()
    expect(s).toContain("Only part of this page could be extracted")
    expect(pivotAt(h.t, s.split("\n").findIndex((l) => l.includes("Only part")), theme.warn)).toBeDefined()
  })

  test("ready screen says what is not read, only when something is", async () => {
    const plain = await app(docFrom(PROSE))
    expect(await plain.screen()).not.toContain("not read")
    plain.t.renderer.destroy()
    open = undefined
    const md = "# Title\n\nIntro words here.\n\n```\nlet x = 1;\n```\n\n```\nlet y = 2;\n```\n\nOutro."
    const h = await app(docFrom(md, "text/markdown"))
    expect(await h.screen()).toContain("not read: 2 code blocks (marked)")
  })

  test("ready screen says when a document is only an abstract", async () => {
    const doc = fixtureDoc("10-arxiv-abs")
    expect(doc.source).toBe("citation_abstract")
    const h = await app(doc)
    const s = await h.screen()
    expect(s).toContain("Abstract only")
    expect(s).not.toContain("JavaScript")
  })

  test("ready screen notes a paywall hint on a complete page", async () => {
    const doc = fixtureDoc("14-nonenglish-spiegel")
    expect(doc.paywall_hint && !doc.partial).toBe(true)
    const h = await app(doc)
    expect(await h.screen()).toContain("marked as paywalled")
  })

  test("space plays; words advance on the clock; space pauses and shows the paragraph", async () => {
    const h = await app(docFrom(PROSE))
    let s = await h.press(" ")
    expect(h.session.state).toBe("playing")
    expect(s).toContain("0%  300 wpm")
    const row = s.split("\n").findIndex((l) => l.includes("┬")) + 1
    expect(pivotAt(h.t, row)!.col).toBe(pivotColumn(80))
    h.clock.tick(2000)
    s = await h.screen()
    expect(h.session.index).toBeGreaterThan(2)
    expect(s).toContain(h.word().slice(1))
    const current = h.word()
    s = await h.press(" ")
    expect(h.session.state).toBe("paused")
    expect(s).toContain("paused · space resume")
    expect(s).toContain("The first sentence is short. The second sentence follows it closely.")
    // The current word is highlighted inside the paragraph.
    const ctxRow = s.split("\n").findIndex((l) => l.includes("The first sentence"))
    const mark = t(h).lines[ctxRow]!.spans.find((sp) => sameColor(sp.fg, theme.mark.fg) && sp.text.trim() !== "")!
    expect(mark.text).toBe(current)
    expect(mark.attributes & TextAttributes.UNDERLINE).toBeTruthy()
    h.clock.tick(5000)
    expect(h.word()).toBe(current)
  })

  test("paused paragraph lines share one left edge and the footer starts at column 0", async () => {
    const long = Array.from({ length: 30 }, (_, i) => `word${i}`).join(" ")
    const h = await app(docFrom(long), { width: 40, height: 20 })
    await h.press(" ")
    await h.press(" ")
    await h.press("5")
    const s = await h.screen()
    const rows = s.split("\n").filter((l) => /word\d/.test(l) && !l.includes("┬"))
    expect(rows.length).toBeGreaterThan(2)
    const edges = new Set(rows.slice(1).map((l) => l.search(/\S/)))
    expect(edges.size).toBe(1)
    const footer = s.split("\n")[19]!
    expect(footer.search(/\S/)).toBe(0)
  })

  test("navigation keys", async () => {
    const h = await app(docFrom(PROSE))
    await h.press(" ")
    await h.press(" ")
    await h.arrow("right")
    expect(h.word()).toBe("The")
    expect(h.session.index).toBe(5)
    await h.press("l")
    expect(h.word()).toBe("A")
    await h.press("h")
    expect(h.session.index).toBe(5)
    await h.arrow("left")
    expect(h.session.index).toBe(0)
    await h.press("]")
    expect(h.word()).toBe("A")
    await h.arrow("right", { shift: true })
    expect(h.word()).toBe("Last")
    await h.press("[")
    expect(h.word()).toBe("A")
    await h.arrow("left", { shift: true })
    await h.arrow("left", { shift: true })
    expect(h.session.index).toBe(0)
    await h.press(".")
    await h.press(".")
    expect(h.session.index).toBe(2)
    await h.press(",")
    expect(h.session.index).toBe(1)
    await h.press("5")
    expect(h.session.index).toBe(Math.floor(h.session.tokenCount / 2))
    await h.press("0")
    expect(h.session.index).toBe(0)
    await h.press("9")
    const s = await h.press("r")
    expect(h.session.index).toBe(0)
    expect(h.session.state).toBe("paused")
    expect(s).toContain("paused")
  })

  test("speed keys change WPM by 25 and update the footer", async () => {
    const h = await app(docFrom(PROSE))
    await h.press(" ")
    await h.arrow("up")
    let s = await h.press("k")
    expect(h.session.wpm).toBe(350)
    expect(s).toContain("350 wpm")
    await h.arrow("down")
    await h.press("j")
    s = await h.press("j")
    expect(h.session.wpm).toBe(275)
    expect(s).toContain("275 wpm")
  })

  test("speed keys on the ready screen update the estimate", async () => {
    const h = await app(fixtureDoc("01-iankduncan"))
    for (let i = 0; i < 4; i++) await h.arrow("up")
    expect(await h.screen()).toContain("1,815 words · about 5 min at 400 wpm")
  })

  test("? toggles help and pauses; esc closes it", async () => {
    const h = await app(docFrom(PROSE))
    await h.press(" ")
    let s = await h.press("?")
    expect(s).toContain("Keys")
    expect(s).toContain("previous / next sentence")
    expect(h.session.state).toBe("paused")
    await h.press(" ")
    expect(h.session.state).toBe("paused")
    s = await h.press("?")
    expect(s).not.toContain("previous / next sentence")
    await h.press("?")
    h.t.mockInput.pressEscape()
    await Bun.sleep(60)
    expect(await h.screen()).not.toContain("previous / next sentence")
  })

  test("finished screen after the last word; r reads again", async () => {
    const h = await app(docFrom("Just four short words."))
    await h.press(" ")
    h.clock.runUntilIdle()
    let s = await h.screen()
    expect(h.session.state).toBe("finished")
    expect(s).toContain("Finished.")
    expect(s).toMatch(/4 words in 0:0\d/)
    expect(s).toContain("r read again")
    s = await h.press("r")
    expect(h.session.state).toBe("playing")
    expect(h.session.index).toBe(0)
  })

  test("finished after a straight read reports a rate; after a jump it does not", async () => {
    const straight = await app(docFrom(PROSE))
    await straight.press(" ")
    straight.clock.runUntilIdle()
    const s1 = await straight.screen()
    expect(s1).toMatch(/\d+ words in 0:\d\d · [\d,]+ wpm overall/)
    expect(s1).toContain("space or r read again")
    straight.t.renderer.destroy()
    open = undefined

    const jumped = await app(docFrom(PROSE))
    await jumped.press(" ")
    jumped.clock.tick(500)
    await jumped.press("9")
    jumped.clock.runUntilIdle()
    const s2 = await jumped.screen()
    expect(jumped.session.state).toBe("finished")
    expect(s2).toContain("words in the document")
    expect(s2).toContain("spent reading")
    expect(s2).not.toContain("wpm overall")
  })

  test("space on the finished screen reads again as a fresh straight run", async () => {
    const h = await app(docFrom("Just four short words."))
    await h.press(" ")
    await h.press("9")
    h.clock.runUntilIdle()
    expect(await h.screen()).not.toContain("wpm overall")
    await h.press(" ")
    expect(h.session.state).toBe("playing")
    expect(h.session.index).toBe(0)
    h.clock.runUntilIdle()
    expect(await h.screen()).toContain("wpm overall")
  })

  test("ctrl-z pauses, leaves the terminal, stops the process; continuing re-enters paused at the same word", async () => {
    const events: string[] = []
    const terminal = {
      suspend: async () => void events.push("suspend"),
      resume: async () => void events.push("resume"),
    }
    const h = await mount(docFrom(PROSE), { stopProcess: () => events.push("stop"), terminal })
    open = h
    await h.press(" ")
    h.clock.tick(400)
    const index = h.session.index
    expect(index).toBeGreaterThan(0)
    h.t.mockInput.pressKey("z", { ctrl: true })
    await Bun.sleep(30)
    expect(h.session.state).toBe("paused")
    expect(events).toEqual(["suspend", "stop"])
    h.clock.tick(10_000)
    expect(h.session.index).toBe(index)
    process.emit("SIGCONT")
    await Bun.sleep(30)
    expect(events).toEqual(["suspend", "stop", "resume"])
    expect(h.session.state).toBe("paused")
    expect(h.session.index).toBe(index)
    await h.press(" ")
    expect(h.session.state).toBe("playing")
  })

  test("q and ctrl-c quit", async () => {
    const h = await app(docFrom(PROSE))
    h.t.mockInput.pressKey("q")
    await Bun.sleep(5)
    expect(h.quits()).toBe(1)
    expect(h.t.renderer.isDestroyed).toBe(true)
    open = undefined
    const h2 = await app(docFrom(PROSE))
    h2.t.mockInput.pressCtrlC()
    await Bun.sleep(5)
    expect(h2.quits()).toBe(1)
  })

  test("resizing keeps the pivot centred", async () => {
    const h = await app(docFrom(PROSE))
    await h.press(" ")
    for (const [w, ht] of [[50, 12], [24, 8], [120, 40]] as const) {
      h.t.resize(w, ht)
      const s = await h.screen()
      const row = s.split("\n").findIndex((l) => l.includes("┬")) + 1
      expect(pivotAt(h.t, row)!.col).toBe(pivotColumn(w))
    }
  })
})

function t(h: Harness) {
  return h.t.captureSpans()
}
