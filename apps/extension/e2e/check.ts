// Loads the e2e build of the extension into Chromium and drives it against
// local pages. Needs `bun run ext:build:e2e` first (`bun run ext:e2e` does both)
// and Playwright's Chromium (`node_modules/.bin/playwright-core install chromium`).
import { createServer, type Server } from "node:http"
import { mkdtempSync, readFileSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { chromium, type BrowserContext, type Frame, type Page, type Worker } from "playwright-core"

const root = join(import.meta.dirname, "../../..")
const extensionDir = join(import.meta.dirname, "../dist-e2e")
const truth: Record<string, { start: string }> = JSON.parse(readFileSync(join(root, "fixtures/truth.json"), "utf8"))

const ARTICLE = [
  "Quartz clocks keep surprisingly good time because a crystal vibrates at a steady rate.",
  "Engineers count those vibrations and turn them into seconds, minutes and hours without any fuss.",
  "The same trick, repeated a billion times a second, drives the processors inside our phones.",
]
const SPA = `<!doctype html><html><head><meta charset="utf-8"><title>Rendered later</title></head>
<body><div id="app"></div><input id="note" aria-label="note"><div style="height:4000px"></div>
<script>
  setTimeout(() => {
    const main = document.createElement("article")
    const h = document.createElement("h1"); h.textContent = "Clocks and crystals"; main.append(h)
    for (const text of ${JSON.stringify(ARTICLE)}) { const p = document.createElement("p"); p.textContent = text; main.append(p) }
    document.getElementById("app").append(main)
  }, 50)
</script></body></html>`

function serve(): Promise<{ server: Server; origin: string }> {
  const server = createServer((req, res) => {
    const path = new URL(req.url ?? "/", "http://x").pathname
    if (path === "/spa.html") {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" }).end(SPA)
      return
    }
    const match = /^\/fixture\/([\w.-]+\.html)$/.exec(path)
    if (!match?.[1]) {
      res.writeHead(404).end()
      return
    }
    try {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8" }).end(readFileSync(join(root, "fixtures", match[1])))
    } catch {
      res.writeHead(404).end()
    }
  })
  return new Promise((resolve) =>
    server.listen(0, "127.0.0.1", () => {
      const address = server.address()
      resolve({ server, origin: `http://127.0.0.1:${typeof address === "object" && address ? address.port : 0}` })
    }),
  )
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message)
}

const squash = (text: string) => text.replace(/\s+/g, " ").replace(/[‘’]/g, "'").replace(/[“”]/g, '"').toLowerCase()

const results: Array<{ name: string; ok: boolean }> = []
async function check(name: string, fn: () => Promise<string | void>): Promise<void> {
  try {
    const detail = await fn()
    results.push({ name, ok: true })
    console.log(`ok   ${name}${detail ? ` (${detail})` : ""}`)
  } catch (error) {
    results.push({ name, ok: false })
    console.log(`FAIL ${name}: ${error instanceof Error ? error.message.split("\n")[0] : String(error)}`)
  }
}

async function readerFrame(page: Page): Promise<Frame> {
  const deadline = Date.now() + 10_000
  while (Date.now() < deadline) {
    const frame = page.frames().find((f) => f.url().startsWith("chrome-extension://") && f.url().endsWith("/reader.html"))
    if (frame) return frame
    await page.waitForTimeout(50)
  }
  throw new Error("the reader frame never appeared")
}

async function overlayCount(page: Page): Promise<number> {
  return page.evaluate(() => document.documentElement.children.length)
}

async function firstWord(frame: Frame): Promise<string> {
  const handle = await frame.waitForFunction(() => document.querySelector(".cr-word")?.textContent?.trim() || false, null, { polling: "raf" })
  return String(await handle.jsonValue())
}

async function main(): Promise<void> {
  const { server, origin } = await serve()
  const profile = mkdtempSync(join(tmpdir(), "clench-ext-"))
  let context: BrowserContext | undefined
  try {
    context = await chromium.launchPersistentContext(profile, {
      channel: "chromium",
      headless: true,
      args: [`--disable-extensions-except=${extensionDir}`, `--load-extension=${extensionDir}`],
    })
    const worker: Worker = context.serviceWorkers()[0] ?? (await context.waitForEvent("serviceworker"))
    const extensionOrigin = new URL(worker.url()).origin

    const foreign: string[] = []
    context.on("request", (request) => {
      const frameUrl = (() => {
        try {
          return request.frame().url()
        } catch {
          return ""
        }
      })()
      const fromExtension = request.serviceWorker() !== null || frameUrl.startsWith(extensionOrigin)
      if (fromExtension && !request.url().startsWith(extensionOrigin) && !request.url().startsWith("data:") && !request.url().startsWith("blob:")) foreign.push(request.url())
    })
    // Pages in the fixtures reference third-party hosts; nothing leaves the machine.
    await context.route(/^https?:\/\/(?!127\.0\.0\.1)/, (route) => route.abort())

    const open = async (path: string): Promise<Page> => {
      const page = await context!.newPage()
      await page.goto(`${origin}${path}`)
      await page.bringToFront()
      return page
    }
    const trigger = (): Promise<void> => worker.evaluate(() => (globalThis as unknown as { clenchOpenActiveTab(): Promise<void> }).clenchOpenActiveTab())

    await check("reads a page whose text is rendered by JavaScript", async () => {
      const page = await open("/spa.html")
      await page.waitForFunction(() => document.querySelector("article"))
      const before = await overlayCount(page)
      await trigger()
      const frame = await readerFrame(page)
      await frame.waitForSelector(".cr-title")
      const title = await frame.textContent(".cr-title"); assert(title === "Rendered later", `title was ${JSON.stringify(title)}`)
      await page.keyboard.press("Space")
      const word = await firstWord(frame)
      assert(word === "Clocks", `first word was ${JSON.stringify(word)}`)
      assert((await overlayCount(page)) === before + 1, "exactly one overlay element expected")
      await page.close()
      return `first word ${word}`
    })

    await check("Esc removes the overlay and restores focus and scroll", async () => {
      const page = await open("/spa.html")
      await page.waitForFunction(() => document.querySelector("article"))
      await page.focus("#note")
      await page.evaluate(() => window.scrollTo(0, 700))
      const before = await overlayCount(page)
      await trigger()
      const frame = await readerFrame(page)
      await frame.waitForSelector(".cr-title")
      await page.evaluate(() => window.scrollTo(0, 0))
      await page.keyboard.press("Escape")
      await page.waitForFunction((n) => document.documentElement.children.length === n, before)
      assert(page.frames().every((f) => !f.url().startsWith("chrome-extension://")), "reader frame still present")
      const state = await page.evaluate(() => ({ focus: document.activeElement?.id, y: Math.round(window.scrollY) }))
      assert(state.focus === "note", `focus was ${state.focus}`)
      assert(state.y === 700, `scrollY was ${state.y}`)
      await page.close()
    })

    await check("Esc closes the keys help first, then the reader", async () => {
      const page = await open("/spa.html")
      await page.waitForFunction(() => document.querySelector("article"))
      const before = await overlayCount(page)
      await trigger()
      const frame = await readerFrame(page)
      await frame.waitForSelector(".cr-title")
      await page.keyboard.press("?")
      await frame.waitForSelector("[role=dialog]")
      await page.keyboard.press("Escape")
      await frame.waitForSelector("[role=dialog]", { state: "detached" })
      assert((await overlayCount(page)) === before + 1, "reader closed with the help")
      await page.keyboard.press("Escape")
      await page.waitForFunction((n) => document.documentElement.children.length === n, before)
      await page.close()
    })

    await check("the toolbar action toggles: a second press closes the reader", async () => {
      const page = await open("/spa.html")
      await page.waitForFunction(() => document.querySelector("article"))
      const before = await overlayCount(page)
      await trigger()
      await readerFrame(page)
      await trigger()
      await page.waitForFunction((n) => document.documentElement.children.length === n, before)
      await page.close()
    })

    await check("a saved article page reads from its first sentence", async () => {
      const page = await open("/fixture/05-static-matklad.html")
      await trigger()
      const frame = await readerFrame(page)
      await frame.waitForSelector(".cr-title")
      await page.keyboard.press("Space")
      await firstWord(frame)
      await page.keyboard.press("Space")
      await frame.waitForSelector(".cr-context")
      const context = squash((await frame.textContent(".cr-context")) ?? "")
      const start = squash(truth["05-static-matklad"]?.start ?? "")
      assert(start && context.includes(start), `paragraph did not contain the article's first sentence: ${context.slice(0, 120)}`)
      await page.keyboard.press("Escape")
      await page.close()
    })

    await check("a page with no article text shows the partial warning", async () => {
      const page = await open("/fixture/15-spa-bsky.html")
      await trigger()
      const frame = await readerFrame(page)
      await frame.waitForSelector(".cr-warn")
      const warning = squash((await frame.textContent(".cr-warn")) ?? "")
      assert(warning.includes("only part of this page could be extracted"), `warning was: ${warning}`)
      await page.keyboard.press("Escape")
      await page.close()
    })

    await check("the extension and its reader make no network requests", async () => {
      assert(foreign.length === 0, `requests outside the extension: ${foreign.join(", ")}`)
    })

    await check("a page that cannot be injected marks the toolbar button", async () => {
      const page = await context!.newPage()
      await page.goto("chrome://version")
      await page.bringToFront()
      await trigger()
      const badge = await worker.evaluate(async () => {
        const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true })
        return tab?.id === undefined ? "" : chrome.action.getBadgeText({ tabId: tab.id })
      })
      assert(badge === "!", `badge was ${JSON.stringify(badge)}`)
      await page.close()
    })
  } finally {
    await context?.close()
    server.close()
    rmSync(profile, { recursive: true, force: true })
  }
  const failed = results.filter((r) => !r.ok).length
  console.log(`\n${results.length - failed} passed, ${failed} failed`)
  process.exit(failed ? 1 : 0)
}

await main()
