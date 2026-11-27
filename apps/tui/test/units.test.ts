import { describe, expect, test } from "bun:test"
import { mkdtempSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { parseArgs } from "../src/args"
import * as fmt from "../src/format"
import { actionFor } from "../src/keys"
import { describeExtractError, extract, extractDocument, loadEngine } from "../src/engine"
import { describeStatus, forcedContentType, load, LoadError } from "../src/source"
import { clipStart, highlight, width, window, wrap } from "../src/wrap"

const key = (name: string, extra: Partial<{ sequence: string; ctrl: boolean; shift: boolean; meta: boolean }> = {}) => ({
  name,
  sequence: extra.sequence ?? (name.length === 1 ? name : ""),
  ctrl: false,
  shift: false,
  meta: false,
  ...extra,
})

describe("keys", () => {
  test("map to actions", () => {
    expect(actionFor(key("space", { sequence: " " }))).toEqual({ type: "toggle" })
    expect(actionFor(key("left"))).toEqual({ type: "prevSentence" })
    expect(actionFor(key("left", { shift: true }))).toEqual({ type: "prevParagraph" })
    expect(actionFor(key("right", { shift: true }))).toEqual({ type: "nextParagraph" })
    expect(actionFor(key("l"))).toEqual({ type: "nextSentence" })
    expect(actionFor(key("[", { sequence: "[" }))).toEqual({ type: "prevParagraph" })
    expect(actionFor(key("k"))).toEqual({ type: "faster" })
    expect(actionFor(key("down"))).toEqual({ type: "slower" })
    expect(actionFor(key(".", { sequence: "." }))).toEqual({ type: "stepForward" })
    expect(actionFor(key("7"))).toEqual({ type: "jump", tenth: 7 })
    expect(actionFor(key("?", { sequence: "?", shift: true }))).toEqual({ type: "help" })
    expect(actionFor(key("c", { ctrl: true }))).toEqual({ type: "quit" })
    expect(actionFor(key("r", { ctrl: true }))).toBeUndefined()
    expect(actionFor(key("x"))).toBeUndefined()
  })
})

describe("args", () => {
  test("parse", () => {
    expect(parseArgs(["https://a.b/c"])).toEqual({ kind: "read", source: "https://a.b/c", wpm: 300 })
    expect(parseArgs(["-", "--wpm", "450"])).toEqual({ kind: "read", source: "-", wpm: 450 })
    expect(parseArgs(["--wpm=500", "x.html"])).toEqual({ kind: "read", source: "x.html", wpm: 500 })
    expect(parseArgs(["--help"]).kind).toBe("help")
    expect(parseArgs(["-V"]).kind).toBe("version")
    expect(parseArgs([]).kind).toBe("error")
    expect(parseArgs(["a", "--wpm", "fast"]).kind).toBe("error")
    expect(parseArgs(["a", "--bogus"]).kind).toBe("error")
    expect(parseArgs(["a", "b"]).kind).toBe("error")
  })

  test("--type", () => {
    expect(parseArgs(["-", "--type", "md"])).toEqual({ kind: "read", source: "-", wpm: 300, type: "md" })
    expect(parseArgs(["--type=pdf", "x"])).toEqual({ kind: "read", source: "x", wpm: 300, type: "pdf" })
    expect(parseArgs(["x", "--type", "docx"]).kind).toBe("error")
    expect(parseArgs(["x", "--type"]).kind).toBe("error")
  })
})

describe("format", () => {
  test("durations and counts", () => {
    expect(fmt.clock(0)).toBe("0:00")
    expect(fmt.clock(406_400)).toBe("6:46")
    expect(fmt.clock(3_725_000)).toBe("1:02:05")
    expect(fmt.estimate(20_000)).toBe("under a minute")
    expect(fmt.estimate(406_400)).toBe("about 7 min")
    expect(fmt.estimate(3_900_000)).toBe("about 1 h 5 min")
    expect(fmt.count(1815)).toBe("1,815")
  })
})

describe("wrap", () => {
  test("wraps at spaces by display width and keeps offsets", () => {
    const text = "one two three four five six"
    const lines = wrap(text, 9)
    expect(lines.map((l) => l.text)).toEqual(["one two", "three", "four five", "six"])
    for (const l of lines) expect(text.slice(l.start, l.start + l.text.length)).toBe(l.text)
  })

  test("counts wide characters as two columns and breaks overlong words", () => {
    const lines = wrap("日本語の文章です ok", 6)
    for (const l of lines) expect(width(l.text)).toBeLessThanOrEqual(6)
    expect(lines.map((l) => l.text).join("")).toBe("日本語の文章ですok")
  })

  test("keeps hard line breaks", () => {
    expect(wrap("a b\nc", 80).map((l) => l.text)).toEqual(["a b", "c"])
  })

  test("highlight and window", () => {
    const lines = wrap("alpha beta gamma delta", 11)
    const at = "alpha beta gamma delta".indexOf("gamma")
    const shown = window(lines, at, 1)
    expect(shown.lines[0]!.text).toBe("gamma delta")
    expect(highlight(shown.lines[0]!, at, at + 5)).toEqual({ before: "", mark: "gamma", after: " delta" })
  })

  test("clipStart keeps the tail that fits", () => {
    expect(clipStart("abcdef", 3)).toBe("def")
    expect(clipStart("日本語", 4)).toBe("本語")
    expect(clipStart("ab", 5)).toBe("ab")
  })
})

describe("source", () => {
  test("403 explains bot blocking", () => {
    expect(describeStatus(403, "https://x.test/")).toContain("block")
  })

  test("fetch passes raw bytes and the Content-Type through, with a generic User-Agent", async () => {
    let seen: Headers | undefined
    const body = new Uint8Array([0x3c, 0x70, 0x3e, 0xe9, 0x3c, 0x2f, 0x70, 0x3e])
    const fake = (async (_url: string, init?: RequestInit) => {
      seen = new Headers(init?.headers)
      return new Response(body, { headers: { "content-type": "text/html; charset=iso-8859-1" } })
    }) as unknown as typeof fetch
    const loaded = await load("https://example.test/post", { fetcher: fake })
    expect(loaded.bytes).toEqual(body)
    expect(loaded.contentType).toBe("text/html; charset=iso-8859-1")
    expect(seen!.get("user-agent")).toStartWith("Mozilla/5.0")
    expect(seen!.get("user-agent")).not.toContain("clench")
  })

  test("non-2xx and network failures become readable errors", async () => {
    const status = (code: number) => (async () => new Response("no", { status: code })) as unknown as typeof fetch
    expect(load("https://example.test/", { fetcher: status(403) })).rejects.toThrow(/HTTP 403.*refused/)
    expect(load("https://example.test/", { fetcher: status(404) })).rejects.toThrow(/no such page/)
    const down = (async () => {
      throw Object.assign(new Error("Unable to connect"), { code: "ConnectionRefused" })
    }) as unknown as typeof fetch
    expect(load("https://example.test/", { fetcher: down })).rejects.toThrow(/refused the connection/)
    expect(load("/no/such/file.html")).rejects.toThrow(/no such file/)
  })

  const MD = "# Heading\n\nSome *body* text here.\n\n```\nlet x = 1\n```\n\nMore words after."
  const stdin = (text: string) => async () => new TextEncoder().encode(text)

  test("piped Markdown needs --type md and then reads as structure", async () => {
    loadEngine()
    const plain = await load("-", { stdin: stdin(MD) })
    expect(plain.contentType).toBeUndefined()
    const md = await load("-", { stdin: stdin(MD), type: "md" })
    expect(md.contentType).toBe("text/markdown")
    const doc = extract(md.bytes, md.contentType, "")
    expect(doc.blocks.map((b) => b.kind)).toContain("heading")
    expect(doc.blocks.map((b) => b.kind)).toContain("code")
    const text = doc.blocks.map((b) => b.text).join("\n")
    expect(text).not.toContain("#")
    expect(text).not.toContain("```")
    const asText = extract(plain.bytes, "text/plain", "")
    expect(asText.blocks.map((b) => b.kind)).not.toContain("heading")
  })

  test("--type overrides a server's Content-Type and keeps its charset", async () => {
    expect(forcedContentType("md", "text/plain; charset=iso-8859-1")).toBe("text/markdown; charset=iso-8859-1")
    expect(forcedContentType("html", undefined)).toBe("text/html")
    expect(forcedContentType("pdf", "text/html; charset=utf-8")).toBe("application/pdf")
    const fake = (async () => new Response("# x", { headers: { "content-type": "text/plain" } })) as unknown as typeof fetch
    const loaded = await load("https://example.test/a", { fetcher: fake, type: "md" })
    expect(loaded.contentType).toBe("text/markdown")
  })

  test("local files take their type from the extension, and --type wins", async () => {
    const dir = mkdtempSync(join(tmpdir(), "clench-src-"))
    try {
      for (const [name, type] of [["a.md", "text/markdown"], ["a.MARKDOWN", "text/markdown"], ["a.html", "text/html"], ["a.pdf", "application/pdf"], ["a.txt", "text/plain"]] as const) {
        writeFileSync(join(dir, name), "x")
        expect((await load(join(dir, name))).contentType).toBe(type)
      }
      writeFileSync(join(dir, "notes"), "x")
      expect((await load(join(dir, "notes"))).contentType).toBeUndefined()
      expect((await load(join(dir, "notes"), { type: "md" })).contentType).toBe("text/markdown")
      expect((await load(join(dir, "a.txt"), { type: "html" })).contentType).toBe("text/html")
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  test("a body without Content-Length stops at the size cap", async () => {
    let cancelled = false
    let sent = 0
    const body = new ReadableStream<Uint8Array>({
      pull(controller) {
        sent += 1024
        controller.enqueue(new Uint8Array(1024))
      },
      cancel() {
        cancelled = true
      },
    })
    const fake = (async () => new Response(body)) as unknown as typeof fetch
    await expect(load("https://example.test/big", { fetcher: fake, maxBytes: 10 * 1024 })).rejects.toThrow(/too large to read/)
    expect(cancelled).toBe(true)
    expect(sent).toBeLessThan(100 * 1024)
  })

  test("a declared oversize body is refused without reading it", async () => {
    const fake = (async () => new Response("x", { headers: { "content-length": String(40 * 1024 * 1024) } })) as unknown as typeof fetch
    await expect(load("https://example.test/big", { fetcher: fake })).rejects.toThrow(/too large to read \(40 MB; the limit is 32 MB\)/)
  })

  test("a server that never answers, or stalls mid-body, times out with a clear message", async () => {
    const never = ((_url: string, init?: RequestInit) =>
      new Promise((_resolve, reject) => init!.signal!.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError"))))) as unknown as typeof fetch
    await expect(load("https://example.test/a", { fetcher: never, headerTimeoutMs: 30 })).rejects.toThrow(
      "example.test did not answer within 30 ms",
    )
    const stall = ((_url: string, init?: RequestInit) => {
      const body = new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(new Uint8Array(10))
          init!.signal!.addEventListener("abort", () => controller.error(new DOMException("aborted", "AbortError")))
        },
      })
      return Promise.resolve(new Response(body))
    }) as unknown as typeof fetch
    const err = await load("https://example.test/a", { fetcher: stall, totalTimeoutMs: 50 }).catch((e) => e)
    expect(err).toBeInstanceOf(LoadError)
    expect(err.message).toMatch(/downloading https:\/\/example\.test\/a did not finish within/)
  })
})

describe("engine errors", () => {
  test("a wasm trap becomes a plain extraction failure", () => {
    const trap = () => {
      throw new WebAssembly.RuntimeError("unreachable")
    }
    expect(() => extractDocument(new Uint8Array(), undefined, undefined, trap)).toThrow(/extractor failed on this document/)
    expect(describeExtractError(new Error("clench engine crashed: boom"))).toMatch(/extractor failed/)
  })

  test("too deeply nested pages read sensibly", () => {
    loadEngine()
    const html = "<html><body>" + "<div>".repeat(3000) + "words words words" + "</div>".repeat(3000)
    const message = (() => {
      try {
        extractDocument(new TextEncoder().encode(html), "text/html", "")
      } catch (error) {
        return (error as Error).message
      }
    })()
    expect(message).toBe("the page is nested more than 2,500 elements deep, far beyond any real article, so it was not read")
  })
})
