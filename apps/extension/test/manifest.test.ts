import { describe, expect, test } from "bun:test"
import { buildManifest, EXTENSION_PAGES_CSP } from "../src/manifest.ts"
import { HTML_CONTENT_TYPE, isFromReader, isToReader } from "../src/protocol.ts"

describe("manifest", () => {
  const manifest = buildManifest("1.2.3")

  test("asks only for activeTab, scripting and storage, with no host access", () => {
    expect(manifest.permissions).toEqual(["activeTab", "scripting", "storage"])
    expect(manifest.host_permissions).toBeUndefined()
  })

  test("the shipped manifest has no test-only host permission", () => {
    expect(buildManifest("1.2.3", "e2e").host_permissions).toEqual(["http://127.0.0.1/*"])
    expect(JSON.stringify(manifest)).not.toContain("127.0.0.1")
  })

  test("extension pages may compile WebAssembly and nothing remote", () => {
    expect(manifest.content_security_policy.extension_pages).toBe(EXTENSION_PAGES_CSP)
    expect(EXTENSION_PAGES_CSP).toContain("'wasm-unsafe-eval'")
    expect(EXTENSION_PAGES_CSP).not.toMatch(/https?:|unsafe-eval(?!')|\*/)
  })

  test("Alt+Shift+R runs the toolbar action", () => {
    expect(manifest.commands._execute_action?.suggested_key?.default).toBe("Alt+Shift+R")
  })

  test("only the reader page is web accessible", () => {
    expect(manifest.web_accessible_resources.flatMap((r) => r.resources)).toEqual(["reader.html"])
  })
})

describe("overlay protocol", () => {
  test("recognises reader messages and rejects other data", () => {
    expect(isFromReader({ type: "clench:ready" })).toBe(true)
    expect(isFromReader({ type: "clench:close" })).toBe(true)
    expect(isFromReader({ type: "other" })).toBe(false)
    expect(isFromReader(null)).toBe(false)
    expect(isFromReader("clench:ready")).toBe(false)
  })

  test("a document message needs the html and the page url as strings", () => {
    expect(isToReader({ type: "clench:doc", html: "<p>x</p>", url: "https://a.b/" })).toBe(true)
    expect(isToReader({ type: "clench:doc", html: 1, url: "https://a.b/" })).toBe(false)
    expect(isToReader({ type: "clench:doc", html: "<p>x</p>" })).toBe(false)
  })

  test("page HTML is declared as UTF-8", () => {
    expect(HTML_CONTENT_TYPE).toContain("charset=utf-8")
  })
})
