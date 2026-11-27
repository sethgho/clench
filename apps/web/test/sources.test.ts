import { describe, expect, test } from "bun:test"
import { readFileSync } from "node:fs"
import { extract, loadEngine } from "../../../packages/host/test/engine"
import {
  DEFAULT_PREFERENCES,
  describeFetchFailure,
  fromFile,
  fromText,
  fromUrl,
  loadPreferences,
  parseLaunch,
  savePreferences,
  type StorageLike,
} from "../src/sources"

const BASE = "https://clench.sethgholson.com/"

describe("launch links", () => {
  test("#text= carries text in the fragment", () => {
    expect(parseLaunch(`${BASE}#text=${encodeURIComponent("Hello there, reader.")}`)).toEqual({ kind: "text", text: "Hello there, reader." })
  })

  test("?url= asks for a fetch; non-http addresses are ignored", () => {
    expect(parseLaunch(`${BASE}?url=${encodeURIComponent("https://x.y/a.md")}`)).toEqual({ kind: "url", url: "https://x.y/a.md" })
    expect(parseLaunch(`${BASE}?url=${encodeURIComponent("javascript:alert(1)")}`)).toEqual({ kind: "none" })
  })

  test("share target: url wins, a bare link in text is a url, otherwise text with its title", () => {
    expect(parseLaunch(`${BASE}?title=T&text=see+this&url=https%3A%2F%2Fa.b%2Fc`)).toEqual({ kind: "url", url: "https://a.b/c" })
    expect(parseLaunch(`${BASE}?title=T&text=https%3A%2F%2Fa.b%2Fc`)).toEqual({ kind: "url", url: "https://a.b/c" })
    expect(parseLaunch(`${BASE}?title=Notes&text=Some+shared+words.`)).toEqual({ kind: "text", text: "Some shared words.", title: "Notes" })
    expect(parseLaunch(BASE)).toEqual({ kind: "none" })
  })
})

describe("inputs", () => {
  test("pasted HTML of a saved page reads like the file", () => {
    const html = readFileSync(new URL("../../../fixtures/01-iankduncan.html", import.meta.url), "utf8")
    const loaded = fromText(html, undefined)
    expect(loaded.contentType).toBe("text/html")
    loadEngine()
    const doc = extract(loaded.bytes, loaded.contentType, "")
    expect(doc.title).toBe("Smart People Who Give a Shit Use Tools")
    expect(doc.word_count).toBe(1815)
  })

  test("an explicit type overrides detection", () => {
    expect(fromText("# not a heading", "text/plain").contentType).toBe("text/plain")
    expect(fromText("# Heading\n\n- a\n- b", undefined).contentType).toBe("text/markdown")
  })

  test("files take their type from the name", async () => {
    const file = { name: "notes.md", type: "", arrayBuffer: async () => new TextEncoder().encode("# Hi").buffer as ArrayBuffer }
    const loaded = await fromFile(file)
    expect(loaded.contentType).toBe("text/markdown")
    expect(new TextDecoder().decode(loaded.bytes)).toBe("# Hi")
  })
})

describe("fetching from the page", () => {
  test("sends no custom headers or credentials, and refines a text/plain .md to Markdown", async () => {
    let seen: RequestInit | undefined
    const fetcher = (async (_url: string, init?: RequestInit) => {
      seen = init
      return new Response("# Title\n\nBody.", { headers: { "content-type": "text/plain; charset=utf-8" } })
    }) as unknown as typeof fetch
    const loaded = await fromUrl("https://raw.githubusercontent.com/o/r/main/README.md", { fetcher })
    expect(loaded.contentType).toBe("text/markdown; charset=utf-8")
    expect(seen?.headers).toBeUndefined()
    expect(seen?.credentials).toBe("omit")
    expect(seen?.mode).toBe("cors")
  })

  test("a blocked request explains CORS and the alternatives; offline says offline", async () => {
    const blocked = (async () => {
      throw new TypeError("Failed to fetch")
    }) as unknown as typeof fetch
    const error = await fromUrl("https://news.example/post", { fetcher: blocked }).catch((e: Error) => e.message)
    expect(error).toContain("CORS")
    expect(error).toContain("no server")
    expect(error).toContain("paste")
    expect(describeFetchFailure("https://news.example/post", false)).toContain("offline")
  })

  test("a refusal says so", async () => {
    const fetcher = (async () => new Response("no", { status: 403 })) as unknown as typeof fetch
    await expect(fromUrl("https://news.example/post", { fetcher })).rejects.toThrow(/HTTP 403.*refused/)
  })
})

describe("preferences", () => {
  const memory = (): StorageLike & { data: Map<string, string> } => {
    const data = new Map<string, string>()
    return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) }
  }

  test("round-trip and fall back to defaults on junk", () => {
    const store = memory()
    savePreferences(() => store, { wpm: 425, theme: "dark" })
    expect(loadPreferences(() => store)).toEqual({ wpm: 425, theme: "dark" })
    store.data.set("clench.preferences", "{not json")
    expect(loadPreferences(() => store)).toEqual(DEFAULT_PREFERENCES)
    store.data.set("clench.preferences", JSON.stringify({ wpm: -3, theme: "neon" }))
    expect(loadPreferences(() => store)).toEqual(DEFAULT_PREFERENCES)
  })

  test("storage that throws or is missing is harmless", () => {
    const throwing: StorageLike = {
      getItem: () => {
        throw new Error("SecurityError")
      },
      setItem: () => {
        throw new Error("QuotaExceededError")
      },
    }
    expect(loadPreferences(() => throwing)).toEqual(DEFAULT_PREFERENCES)
    expect(() => savePreferences(() => throwing, DEFAULT_PREFERENCES)).not.toThrow()
    expect(loadPreferences(() => undefined)).toEqual(DEFAULT_PREFERENCES)
    expect(() =>
      loadPreferences(() => {
        throw new Error("blocked")
      }),
    ).not.toThrow()
  })
})
