// Drives the built web reader in the locally installed Google Chrome.
//
//   node apps/web/e2e/check.ts                 serves apps/web/dist at /clench/ and checks it
//   node apps/web/e2e/check.ts --url <site>    checks a deployed copy instead
//   --shots <dir>                              also saves screenshots there
//   --headed                                   shows the browser
//
// It needs network access for the URL checks and is not part of CI.
import { createServer, type Server } from "node:http"
import { mkdirSync, readFileSync, statSync } from "node:fs"
import { extname, join, normalize } from "node:path"
import { chromium, type Browser, type Page } from "playwright-core"

const root = join(import.meta.dirname, "../../..")
const args = process.argv.slice(2)
const option = (name: string) => {
  const i = args.indexOf(name)
  return i >= 0 ? args[i + 1] : undefined
}
const shots = option("--shots")
const headed = args.includes("--headed")
const fixture = readFileSync(join(root, "fixtures/01-iankduncan.html"), "utf8")
const RAW_MD = "https://raw.githubusercontent.com/sethgho/clench/main/README.md"
const CORS_BLOCKED = "https://example.com/"

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".png": "image/png",
  ".webmanifest": "application/manifest+json",
}

/** A static server for the build, mounted at /clench/ like GitHub Pages. */
function serve(dir: string): Promise<{ server: Server; url: string }> {
  const server = createServer((req, res) => {
    const path = new URL(req.url ?? "/", "http://x").pathname
    if (!path.startsWith("/clench/")) {
      res.writeHead(404).end()
      return
    }
    let file = normalize(join(dir, decodeURIComponent(path.slice("/clench/".length))))
    if (!file.startsWith(dir)) {
      res.writeHead(403).end()
      return
    }
    try {
      if (statSync(file).isDirectory()) file = join(file, "index.html")
      res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" })
      res.end(readFileSync(file))
    } catch {
      res.writeHead(404).end()
    }
  })
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => {
    const address = server.address()
    const port = typeof address === "object" && address ? address.port : 0
    resolve({ server, url: `http://127.0.0.1:${port}/clench/` })
  }))
}

const results: Array<{ name: string; ok: boolean; detail?: string }> = []
async function check(name: string, fn: () => Promise<string | void>): Promise<void> {
  try {
    const detail = await fn()
    results.push({ name, ok: true, detail: detail || undefined })
    console.log(`ok   ${name}${detail ? ` (${detail})` : ""}`)
  } catch (error) {
    const detail = error instanceof Error ? error.message.split("\n")[0] : String(error)
    results.push({ name, ok: false, detail })
    console.log(`FAIL ${name}: ${detail}`)
  }
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message)
}

async function shot(page: Page, name: string): Promise<void> {
  if (!shots) return
  mkdirSync(shots, { recursive: true })
  await page.screenshot({ path: join(shots, `${name}.png`) })
}

async function engineReady(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Read", exact: true }).and(page.locator(":enabled")).waitFor({ timeout: 20_000 })
}

async function pasteAndRead(page: Page, text: string): Promise<void> {
  await page.locator("#paste").fill(text)
  await page.getByRole("button", { name: "Read", exact: true }).click()
  await page.locator(".clench-reader .cr-title").waitFor()
}

async function readyText(page: Page): Promise<string> {
  return (await page.locator(".clench-reader .cr-card").innerText()).replace(/\s+/g, " ")
}

async function noHorizontalScroll(page: Page): Promise<string> {
  const { scroll, client } = await page.evaluate(() => ({
    scroll: document.documentElement.scrollWidth,
    client: document.documentElement.clientWidth,
  }))
  assert(scroll <= client, `page is ${scroll}px wide in a ${client}px viewport`)
  return `${scroll}px in ${client}px`
}

async function closeReader(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Close the reader" }).click()
  await page.locator(".clench-reader").waitFor({ state: "detached" })
}

async function run(browser: Browser, base: string): Promise<void> {
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: "light" })
  const page = await context.newPage()
  const errors: string[] = []
  page.on("pageerror", (e) => errors.push(e.message))
  const thirdParty: string[] = []
  const origin = new URL(base).origin
  page.on("request", (r) => {
    const u = new URL(r.url())
    if (u.origin !== origin && !r.url().startsWith(RAW_MD) && !r.url().startsWith(CORS_BLOCKED) && u.protocol.startsWith("http")) thirdParty.push(r.url())
  })

  await check("loads and the engine starts", async () => {
    await page.goto(base)
    await engineReady(page)
    await shot(page, "01-input")
  })

  await check("pasted HTML: ready screen with title, word count and not-read line", async () => {
    await pasteAndRead(page, fixture)
    const text = await readyText(page)
    assert(text.includes("Smart People Who Give a Shit Use Tools"), `title missing: ${text.slice(0, 120)}`)
    assert(/1,815 words/.test(text), `word count missing: ${text}`)
    assert(text.includes("not read: 1 caption"), `not-read line missing: ${text}`)
    await shot(page, "02-ready")
    return text.match(/1,815 words[^·]*· [^n]*/)?.[0]?.trim()
  })

  let pivotSpread = 0
  await check("plays: words advance and pivot letters share an x-coordinate", async () => {
    await page.getByRole("button", { name: "Start reading" }).click()
    const samples: Array<{ index: number; x: number; word: string }> = []
    const end = Date.now() + 3000
    while (Date.now() < end) {
      samples.push(
        await page.evaluate(() => {
          const pivot = document.querySelector(".clench-reader .cr-pivot")!.getBoundingClientRect()
          const word = document.querySelector<HTMLElement>(".clench-reader .cr-word")!
          return { index: Number(word.dataset.index), x: pivot.left + pivot.width / 2, word: word.textContent ?? "" }
        }),
      )
      await page.waitForTimeout(90)
    }
    const indices = new Set(samples.map((s) => s.index))
    assert(indices.size >= 8, `only ${indices.size} distinct words in 3 s`)
    const xs = samples.map((s) => s.x)
    pivotSpread = Math.max(...xs) - Math.min(...xs)
    assert(pivotSpread <= 1, `pivot centres spread over ${pivotSpread.toFixed(2)}px`)
    await shot(page, "03-reading")
    return `${indices.size} words, last index ${Math.max(...indices)}, pivot spread ${pivotSpread.toFixed(2)}px, words like "${samples.at(-1)!.word}"`
  })

  await check("space pauses and shows the paragraph with the word marked", async () => {
    await page.keyboard.press(" ")
    const mark = page.locator(".clench-reader .cr-context mark")
    await mark.waitFor()
    const marked = (await mark.innerText()).trim()
    const word = ((await page.locator(".clench-reader .cr-word").textContent()) ?? "").trim()
    assert(marked === word, `marked "${marked}" but showing "${word}"`)
    const paragraph = await page.locator(".clench-reader .cr-context p").innerText()
    assert(paragraph.length > marked.length, "no surrounding paragraph")
    await shot(page, "04-paused")
    return `"${marked}" in a ${paragraph.length}-character paragraph`
  })

  await check("space on a focused button toggles once", async () => {
    await page.getByRole("button", { name: "Play" }).focus()
    await page.keyboard.press(" ")
    await page.waitForTimeout(150)
    const playing = await page.getByRole("button", { name: "Pause" }).count()
    assert(playing === 1, "expected playback after one press")
    await page.keyboard.press(" ")
  })

  await check("hiding the tab pauses", async () => {
    await page.keyboard.press(" ")
    await page.getByRole("button", { name: "Pause" }).waitFor()
    await page.evaluate(() => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" })
      document.dispatchEvent(new Event("visibilitychange"))
    })
    await page.getByRole("button", { name: "Play" }).waitFor({ timeout: 2000 })
    await page.evaluate(() => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "visible" })
    })
  })

  await check("q closes the reader", async () => {
    await page.locator(".clench-reader").focus()
    await page.keyboard.press("q")
    await page.locator(".clench-reader").waitFor({ state: "detached" })
  })

  await check("picking the file reads it the same way", async () => {
    await page.locator("#file").setInputFiles({ name: "01-iankduncan.html", mimeType: "text/html", buffer: Buffer.from(fixture) })
    await page.locator(".clench-reader .cr-title").waitFor()
    const text = await readyText(page)
    assert(text.includes("Smart People Who Give a Shit Use Tools") && text.includes("1,815 words"), text.slice(0, 160))
    await closeReader(page)
  })

  await check("dropping the file reads it the same way", async () => {
    await page.evaluate((html) => {
      const data = new DataTransfer()
      data.items.add(new File([html], "01-iankduncan.html", { type: "text/html" }))
      window.dispatchEvent(new DragEvent("dragover", { dataTransfer: data, cancelable: true }))
      window.dispatchEvent(new DragEvent("drop", { dataTransfer: data, cancelable: true }))
    }, fixture)
    await page.locator(".clench-reader .cr-title").waitFor()
    const text = await readyText(page)
    assert(text.includes("not read: 1 caption") && text.includes("1,815 words"), text.slice(0, 160))
    await closeReader(page)
  })

  await check("a URL that blocks cross-site reading gets the honest message", async () => {
    await page.locator("#url").fill(CORS_BLOCKED)
    await page.getByRole("button", { name: "Fetch" }).click()
    const alert = page.getByRole("alert")
    await alert.waitFor({ timeout: 20_000 })
    const text = await alert.innerText()
    assert(text.includes("CORS") && text.includes("paste"), text)
    await shot(page, "05-cors")
    return text.slice(0, 90) + "…"
  })

  await check("raw.githubusercontent.com Markdown loads via URL", async () => {
    await page.locator("#url").fill(RAW_MD)
    await page.getByRole("button", { name: "Fetch" }).click()
    await page.locator(".clench-reader .cr-title").waitFor({ timeout: 20_000 })
    const text = await readyText(page)
    assert(/clench/.test(text) && /words/.test(text), text.slice(0, 160))
    await closeReader(page)
    return text.slice(0, 80)
  })

  await check("#text= deep link opens the ready screen", async () => {
    await page.goto(`${base}#text=${encodeURIComponent("Read the post yourself. It is short and honest.")}`)
    await page.locator(".clench-reader .cr-title").waitFor({ timeout: 20_000 })
    const text = await readyText(page)
    assert(text.includes("9 words"), text)
  })

  await check("dark theme toggle", async () => {
    await page.goto(base)
    await engineReady(page)
    await page.getByRole("button", { name: /Change theme/ }).click()
    await page.getByRole("button", { name: /Change theme/ }).click()
    const theme = await page.evaluate(() => document.documentElement.dataset.theme)
    assert(theme === "dark", `theme is ${theme}`)
    await pasteAndRead(page, fixture)
    await shot(page, "06-dark-ready")
    await closeReader(page)
    await page.getByRole("button", { name: /Change theme/ }).click()
  })

  await check("offline reload works after the first visit", async () => {
    await page.goto(base)
    await engineReady(page)
    await page.evaluate(async () => {
      const registration = await navigator.serviceWorker.ready
      if (!navigator.serviceWorker.controller) {
        await new Promise((resolve) => navigator.serviceWorker.addEventListener("controllerchange", resolve, { once: true }))
      }
      return registration.active?.state
    })
    await context.setOffline(true)
    await page.reload()
    await engineReady(page)
    await pasteAndRead(page, fixture)
    const text = await readyText(page)
    await context.setOffline(false)
    assert(text.includes("1,815 words"), text.slice(0, 120))
    await closeReader(page)
  })

  await check("no requests to third parties and no page errors", async () => {
    assert(thirdParty.length === 0, `third-party requests: ${thirdParty.join(", ")}`)
    assert(errors.length === 0, `page errors: ${errors.join("; ")}`)
  })
  await context.close()

  for (const width of [375, 320]) {
    const mobile = await browser.newContext({ viewport: { width, height: 812 }, hasTouch: true, isMobile: true, colorScheme: "dark" })
    const m = await mobile.newPage()
    await check(`mobile ${width}×812: no horizontal scroll on input, ready, reading and paused`, async () => {
      await m.goto(base)
      await engineReady(m)
      const a = await noHorizontalScroll(m)
      if (width === 375) await shot(m, "07-mobile-input")
      await pasteAndRead(m, fixture)
      await noHorizontalScroll(m)
      if (width === 375) await shot(m, "08-mobile-ready")
      await m.getByRole("button", { name: "Start reading" }).tap()
      await m.waitForTimeout(1500)
      await noHorizontalScroll(m)
      await m.locator(".clench-reader .cr-stage").tap()
      await m.locator(".clench-reader .cr-context mark").waitFor()
      await noHorizontalScroll(m)
      if (width === 375) await shot(m, "09-mobile-paused")
      return a
    })
    await mobile.close()
  }
}

const target = option("--url")
const local = target ? undefined : await serve(join(root, "apps/web/dist"))
const base = target ?? local!.url
console.log(`checking ${base}`)
const browser = await chromium.launch({ channel: "chrome", headless: !headed })
try {
  await run(browser, base)
} finally {
  await browser.close()
  local?.server.close()
}
const failed = results.filter((r) => !r.ok)
console.log(`\n${results.length - failed.length}/${results.length} checks passed`)
process.exit(failed.length > 0 ? 1 : 0)
