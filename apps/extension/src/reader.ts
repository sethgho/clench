import { ExtractError, extractWith, isEngineCrash, messageOf } from "@clench/host"
import type { Document } from "@clench/host/engine-types"
import { extract, loadEngine, mount, type ReaderHandle } from "@clench/reader-web"
import wasmUrl from "../../../crates/clench-wasm/pkg/clench_bg.wasm?url"
import { HTML_CONTENT_TYPE, isToReader, type FromReader } from "./protocol"

const host = document.getElementById("reader")
if (!host) throw new Error("reader host missing")

const DEFAULT_WPM = 300
let handle: ReaderHandle | undefined

function send(message: FromReader): void {
  window.parent.postMessage(message, "*")
}

function notice(text: string): void {
  host?.replaceChildren()
  const box = document.createElement("div")
  box.className = "notice"
  box.setAttribute("role", "alert")
  const p = document.createElement("p")
  p.textContent = text
  const close = document.createElement("button")
  close.type = "button"
  close.textContent = "Close"
  close.addEventListener("click", () => send({ type: "clench:close" }))
  box.append(p, close)
  host?.append(box)
  close.focus()
}

async function savedWpm(): Promise<number> {
  try {
    const stored = await chrome.storage.local.get("wpm")
    const wpm: unknown = stored.wpm
    return typeof wpm === "number" && wpm >= 50 && wpm <= 1500 ? wpm : DEFAULT_WPM
  } catch {
    return DEFAULT_WPM
  }
}

function saveWpm(wpm: number): void {
  chrome.storage.local.set({ wpm }).catch(() => {})
}

async function open(html: string, url: string): Promise<void> {
  try {
    await loadEngine(chrome.runtime.getURL(new URL(wasmUrl, location.href).pathname))
  } catch (error) {
    notice(`The reading engine could not be loaded: ${messageOf(error)}`)
    return
  }
  let doc: Document
  try {
    doc = extractWith(extract, new TextEncoder().encode(html), HTML_CONTENT_TYPE, url)
  } catch (error) {
    const crashed = isEngineCrash(error)
    notice(`Could not read this page: ${error instanceof ExtractError ? error.message : messageOf(error)}${crashed ? " Reload the page to try again." : ""}`)
    return
  }
  if (doc.word_count === 0) {
    notice("Nothing to read: no readable text was found on this page.")
    return
  }
  host?.replaceChildren()
  handle = mount(host as HTMLElement, doc, {
    wpm: await savedWpm(),
    onClose: () => send({ type: "clench:close" }),
    onWpmChange: saveWpm,
  })
}

window.addEventListener("message", (event) => {
  if (event.source !== window.parent || !isToReader(event.data)) return
  void open(event.data.html, event.data.url)
})

// Esc closes the reader; while the keys help is showing it only closes that.
document.addEventListener(
  "keydown",
  (event) => {
    if (event.key !== "Escape" || handle?.controller.state.view === "help") return
    send({ type: "clench:close" })
  },
  true,
)

send({ type: "clench:ready" })
