import { beforeAll, describe, expect, test } from "bun:test"
import { readFileSync } from "node:fs"
import { join } from "node:path"
import { extract, initSync, Session, version, type Document } from "../pkg/meatread.js"

const root = join(import.meta.dir, "../../..")
const fixtures = join(root, "fixtures")
const cli = join(root, "target/debug/meatread")

function fixture(name: string): { bytes: Uint8Array; url: string } {
  const bytes = readFileSync(join(fixtures, name))
  let url = ""
  try {
    url = readFileSync(join(fixtures, name.replace(/\.\w+$/, ".url")), "utf8").trim()
  } catch {}
  return { bytes, url }
}

function native(args: string[]): string {
  const run = Bun.spawnSync([cli, ...args], { stdout: "pipe", stderr: "pipe" })
  if (run.exitCode !== 0) throw new Error(`meatread ${args.join(" ")}: ${run.stderr.toString()}`)
  return run.stdout.toString()
}

const contentType: Record<string, string> = { html: "text/html", pdf: "application/pdf" }

function extractFixture(name: string): Document {
  const { bytes, url } = fixture(name)
  return extract(bytes, contentType[name.split(".").pop()!], url)
}

beforeAll(() => {
  initSync({ module: readFileSync(join(import.meta.dir, "../pkg/meatread_bg.wasm")) })
  const build = Bun.spawnSync(["cargo", "build", "-q", "-p", "meatread-cli"], { cwd: root, stderr: "pipe" })
  if (build.exitCode !== 0) throw new Error(build.stderr.toString())
}, 600_000)

describe("extract", () => {
  test("reads the acceptance page", () => {
    const doc = extractFixture("01-iankduncan.html")
    expect(doc.title).toBe("Smart People Who Give a Shit Use Tools")
    expect(doc.blocks[0]!.text.startsWith("LLMs mostly amplify whoever is sitting at the keyboard.")).toBe(true)
    expect(doc.word_count).toBe(1815)
    expect(doc.partial).toBe(false)
    const session = new Session(doc)
    expect(session.wordCount).toBe(1815)
    const f = session.frame()
    expect(f.before + f.pivot + f.after).toBe("LLMs")
    session.free()
  })

  for (const name of ["01-iankduncan.html", "05-static-matklad.html", "07-rustbook.html", "04-ghost-404media.html", "attention.pdf"]) {
    test(`matches the native extractor byte for byte: ${name}`, () => {
      const { url } = fixture(name)
      const nativeJson = native(["extract", join(fixtures, name), "--json", ...(url ? ["--url", url] : [])])
      expect(JSON.stringify(extractFixture(name))).toBe(JSON.stringify(JSON.parse(nativeJson)))
    })
  }

  test("throws readable errors", () => {
    expect(() => extract(new Uint8Array(), "text/plain", "")).toThrow("no readable text found")
    expect(() => extract(new Uint8Array([0x89, 0x50, 0x4e, 0x47]), "image/png", "")).toThrow("unsupported content type")
    expect(() => extract(new TextEncoder().encode("%PDF-1.4 garbage"), "application/pdf", "")).toThrow(/PDF/)
  })

  test("reports its version", () => {
    expect(version()).toMatch(/^\d+\.\d+\.\d+$/)
  })
})

describe("Session", () => {
  test("total duration matches the native engine and is reached by advance()", () => {
    const name = "01-iankduncan.html"
    const schedule = native(["script", join(fixtures, name), "--wpm", "400"])
    const nativeTotal = schedule
      .split("\n")
      .map((line) => /(\d+) ms {2}\S+$/.exec(line))
      .reduce((sum, m) => sum + (m ? Number(m[1]) : 0), 0)

    const doc = extractFixture(name)
    const exact = new Session(doc, { wpm: 400, pacing: { ease_in_words: 0 } })
    expect(exact.totalMs).toBe(nativeTotal)

    exact.play()
    let played = 0
    for (let wait = exact.nextChangeIn(); wait !== undefined; wait = exact.nextChangeIn()) {
      exact.advance(wait)
      played += wait
    }
    expect(exact.state).toBe("finished")
    expect(played).toBe(nativeTotal)

    const chunked = new Session(doc, { wpm: 400, pacing: { ease_in_words: 0 } })
    chunked.play()
    let elapsed = 0
    while (chunked.state === "playing") {
      chunked.advance(37)
      elapsed += 37
    }
    expect(elapsed).toBeGreaterThanOrEqual(nativeTotal)
    expect(elapsed).toBeLessThan(nativeTotal + 37)
    exact.free()
    chunked.free()
  })

  const text = "First sentence here. Second one follows.\n\nNew paragraph starts. It ends."
  const plain = () => extract(new TextEncoder().encode(text), "text/plain", "")

  test("navigation controls", () => {
    const s = new Session(plain())
    const word = () => {
      const f = s.frame()
      return f.before + f.pivot + f.after
    }
    expect(s.state).toBe("paused")
    expect(word()).toBe("First")
    s.nextSentence()
    expect(word()).toBe("Second")
    s.step(1)
    expect(word()).toBe("one")
    s.prevSentence()
    expect(word()).toBe("Second")
    s.prevSentence()
    expect(word()).toBe("First")
    s.nextParagraph()
    expect(word()).toBe("New")
    s.prevParagraph()
    s.prevParagraph()
    expect(word()).toBe("First")
    s.seek(999)
    expect(s.index).toBe(s.tokenCount - 1)
    s.seekFraction(0.5)
    expect(s.index).toBe(Math.floor(0.5 * s.tokenCount))
    s.step(-100)
    expect(s.index).toBe(0)
    s.toggle()
    expect(s.state).toBe("playing")
    s.restart()
    expect(s.state).toBe("playing")
    s.toggle()
    expect(s.state).toBe("paused")
  })

  test("wpm changes retime the script", () => {
    const s = new Session(plain(), { wpm: 300 })
    const at300 = s.totalMs
    s.setWpm(600)
    expect(s.wpm).toBe(600)
    expect(Math.abs(s.totalMs - at300 / 2)).toBeLessThan(s.tokenCount)
    s.nudgeWpm(-25)
    expect(s.wpm).toBe(575)
    s.setWpm(5)
    expect(s.wpm).toBe(100)
    s.nudgeWpm(100_000)
    expect(s.wpm).toBe(1500)
    expect(s.frame().wpm).toBe(1500)
  })

  test("context splits the block around the current word", () => {
    const s = new Session(plain())
    s.seek(5)
    const ctx = s.context()!
    expect(ctx.kind).toBe("paragraph")
    expect(ctx.token).toBe("follows.")
    expect(ctx.before + ctx.token + ctx.after).toBe("First sentence here. Second one follows.")
  })

  test("markers are flagged and carry the skipped block as context", () => {
    const md = "Intro words.\n\n```\nlet x = 1;\n```\n\nOutro."
    const doc = extract(new TextEncoder().encode(md), "text/markdown", "")
    const s = new Session(doc)
    s.nextParagraph()
    const f = s.frame()
    expect(f.is_marker).toBe(true)
    expect(f.before + f.pivot + f.after).toBe("[code]")
    expect(s.context()!.token).toContain("let x = 1;")
    expect(s.wordCount).toBe(3)
  })

  test("non-ASCII context survives the UTF-16 boundary", () => {
    const doc = extract(new TextEncoder().encode("Größe 😀 naïve café 日本語です."), "text/plain", "")
    const s = new Session(doc)
    s.seek(3)
    const ctx = s.context()!
    expect(ctx.before).toBe("Größe 😀 naïve ")
    expect(ctx.token).toBe("café")
  })

  test("rejects malformed settings and documents", () => {
    expect(() => new Session(plain(), { wmp: 300 } as never)).toThrow(/invalid settings/)
    expect(() => new Session({ blocks: "nope" } as never)).toThrow(/invalid document/)
  })

  test("an empty document is finished at once", () => {
    const s = new Session({ blocks: [] } as unknown as Document)
    expect(s.state).toBe("finished")
    expect(s.context()).toBeUndefined()
    expect(s.nextChangeIn()).toBeUndefined()
  })
})
