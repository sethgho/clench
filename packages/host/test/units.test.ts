import { describe, expect, test } from "bun:test"
import * as fmt from "../src/format"
import { detectTextType, forcedContentType, isUrl, typeForFile, typeForName, typeForUrl } from "../src/input"
import { actionFor } from "../src/keys"
import { finishedSummary, notRead, siteLabel, warnings } from "../src/summary"
import { docFrom } from "./engine"

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

describe("summary", () => {
  test("notRead words kinds, singulars, marked versus silent, and ignores empty blocks", () => {
    const o = (kind: any, marked: number, marked_words: number, skipped: number, skipped_words: number) => ({ kind, marked, marked_words, skipped, skipped_words })
    expect(notRead([])).toBeUndefined()
    expect(notRead([o("rule", 0, 0, 3, 0)])).toBeUndefined()
    expect(notRead([o("code", 15, 300, 0, 0), o("footnote", 0, 0, 12, 90), o("caption", 0, 0, 1, 5)])).toBe(
      "not read: 15 code blocks (marked), 12 footnotes, 1 caption",
    )
  })

  test("finishedSummary gives a rate only for a straight read", () => {
    expect(finishedSummary(600, 120_000, true)).toBe("600 words in 2:00 · 300 wpm overall")
    expect(finishedSummary(600, 120_000, false)).toBe("600 words in the document · 2:00 spent reading")
    expect(finishedSummary(600, 0, true)).toBe("600 words in the document · 0:00 spent reading")
  })

  test("siteLabel and warnings", () => {
    const doc = { ...docFrom("Some words here."), url: "https://www.example.com/a", byline: "A. Writer" }
    expect(siteLabel(doc)).toBe("A. Writer · example.com")
    expect(warnings(doc)).toEqual([])
    expect(warnings({ ...doc, partial: true }).join(" ")).toContain("Only part of this page")
    expect(warnings({ ...doc, source: "citation_abstract" })[0]).toStartWith("Abstract only")
    expect(warnings({ ...doc, paywall_hint: true })[0]).toContain("paywalled")
  })
})

describe("input", () => {
  test("isUrl and forcedContentType", () => {
    expect(isUrl("https://a.b/c")).toBe(true)
    expect(isUrl("HTTP://a.b")).toBe(true)
    expect(isUrl("ftp://a.b")).toBe(false)
    expect(isUrl("notes.md")).toBe(false)
    expect(forcedContentType("md", "text/plain; charset=iso-8859-1")).toBe("text/markdown; charset=iso-8859-1")
    expect(forcedContentType("pdf", "text/html; charset=utf-8")).toBe("application/pdf")
  })

  test("file types come from the extension first, then a reported type the extractor reads", () => {
    expect(typeForName("post.HTML")).toBe("text/html")
    expect(typeForName("notes.markdown")).toBe("text/markdown")
    expect(typeForName("dir.md/notes")).toBeUndefined()
    expect(typeForName(".md")).toBeUndefined()
    expect(typeForFile("paper.pdf", "")).toBe("application/pdf")
    expect(typeForFile("notes.md", "application/octet-stream")).toBe("text/markdown")
    expect(typeForFile("page", "application/xhtml+xml")).toBe("text/html")
    expect(typeForFile("readme", "text/x-markdown")).toBe("text/markdown")
    expect(typeForFile("data.bin", "application/octet-stream")).toBeUndefined()
    expect(typeForFile("x", "text/plain; charset=utf-8")).toBe("text/plain")
  })

  test("typeForUrl lets a path's extension refine a generic Content-Type", () => {
    const raw = "https://raw.githubusercontent.com/o/r/main/README.md"
    expect(typeForUrl(raw, "text/plain; charset=utf-8")).toBe("text/markdown; charset=utf-8")
    expect(typeForUrl(raw, undefined)).toBe("text/markdown")
    expect(typeForUrl("https://x.y/a.pdf", "application/octet-stream")).toBe("application/pdf")
    expect(typeForUrl("https://x.y/post", "text/plain")).toBe("text/plain")
    expect(typeForUrl("https://x.y/post.md", "text/html; charset=utf-8")).toBe("text/html; charset=utf-8")
    expect(typeForUrl("https://x.y/notes.txt?x=1", undefined)).toBe("text/plain")
  })

  test("detectTextType tells HTML, Markdown and prose apart", () => {
    expect(detectTextType("<!DOCTYPE html><html><body><p>Hi</p></body></html>")).toBe("text/html")
    expect(detectTextType("  <html lang=en><p>x")).toBe("text/html")
    expect(detectTextType("<article><h1>T</h1><p>One</p><p>Two</p></article>")).toBe("text/html")
    expect(detectTextType("# Title\n\nSome text with a [link](https://x.y).")).toBe("text/markdown")
    expect(detectTextType("Intro.\n\n```\ncode\n```\n")).toBe("text/markdown")
    expect(detectTextType("Shopping:\n- eggs\n- milk\n\nSee [list](https://x.y).")).toBe("text/markdown")
    expect(detectTextType("Just a paragraph. It mentions <b> once and #hashtags.")).toBe("text/plain")
    expect(detectTextType("Price is 5 * 3 = 15.\n\nAnother line.")).toBe("text/plain")
  })
})
