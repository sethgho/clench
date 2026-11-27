import type { Clock } from "@clench/host"
import type { Document as ClenchDocument, Settings } from "@clench/host/engine-types"
import { render } from "solid-js/web"
import { createComponent } from "solid-js"
import init, { Session } from "../../../crates/clench-wasm/pkg/clench.js"
import { ReaderController, type ReaderSession } from "./controller"
import { actionForKey } from "./input"
import { READER_CSS } from "./styles"
import { ReaderView } from "./view"

export { ReaderController, type ReaderSession, type ReaderState, type View } from "./controller"
export { actionForGesture, actionForKey, classifyGesture, type Gesture } from "./input"
export { READER_CSS } from "./styles"
export { extract, version as engineVersion } from "../../../crates/clench-wasm/pkg/clench.js"

export type Theme = "auto" | "light" | "dark"

/**
 * Loads the engine from `wasm` (a URL or the module's bytes). Must finish
 * before `mount` or `extract` is called; later calls are no-ops.
 */
export async function loadEngine(wasm: string | URL | BufferSource): Promise<void> {
  await init({ module_or_path: wasm })
}

export interface MountOptions {
  wpm?: number
  /** Engine settings beyond the speed; `wpm` above wins over `settings.wpm`. */
  settings?: Settings
  theme?: Theme
  /** Shows a close control and binds `q`; the host removes the reader. */
  onClose?: () => void
  onWpmChange?: (wpm: number) => void
  /** Focus the reader so keys work at once (default true). */
  autoFocus?: boolean
  clock?: Clock
  /** Builds the session; the engine's `Session` by default. */
  createSession?: (doc: ClenchDocument, settings: Settings) => ReaderSession & { free?(): void }
}

export interface ReaderHandle {
  readonly controller: ReaderController
  setTheme(theme: Theme): void
  /** Stops playback, removes the reader and frees its session. */
  destroy(): void
}

const STYLE_ID = "clench-reader-style"

/** Adds the reader's stylesheet once to the document or shadow root holding `element`. */
function injectStyles(element: HTMLElement): void {
  const root = element.getRootNode()
  const owner = element.ownerDocument
  const holder: HTMLElement | ShadowRoot = root instanceof ShadowRoot ? root : owner.head
  if (holder.querySelector(`#${STYLE_ID}`)) return
  const style = owner.createElement("style")
  style.id = STYLE_ID
  style.textContent = READER_CSS
  holder.appendChild(style)
}

/**
 * Mounts a reader for `doc` into `element`, which it fills. The reader makes
 * no assumptions about the page around it: its styles are scoped, its keys
 * are bound to its own root, and it pauses when the page is hidden.
 */
export function mount(element: HTMLElement, doc: ClenchDocument, options: MountOptions = {}): ReaderHandle {
  injectStyles(element)
  const settings: Settings = { ...options.settings, ...(options.wpm ? { wpm: options.wpm } : {}) }
  const session = options.createSession ? options.createSession(doc, settings) : new Session(doc, settings)
  const controller = new ReaderController(session, {
    clock: options.clock,
    onClose: options.onClose,
    onWpmChange: options.onWpmChange,
  })
  const ownerDocument = element.ownerDocument
  const root = ownerDocument.createElement("div")
  root.className = "clench-reader"
  root.tabIndex = 0
  root.setAttribute("role", "region")
  root.setAttribute("aria-label", `clench reader: ${doc.title || "Untitled"}`)
  root.setAttribute("aria-roledescription", "speed reader")
  const setTheme = (theme: Theme) => {
    if (theme === "auto") root.removeAttribute("data-theme")
    else root.setAttribute("data-theme", theme)
  }
  setTheme(options.theme ?? "auto")
  element.appendChild(root)

  const onKey = (event: KeyboardEvent) => {
    const target = event.target as HTMLElement | null
    if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return
    const action = actionForKey(event)
    if (!action) return
    if (action.type === "quit" && !options.onClose) return
    event.preventDefault()
    controller.act(action)
  }
  root.addEventListener("keydown", onKey)
  const onVisibility = () => {
    if (ownerDocument.visibilityState === "hidden") controller.pause()
  }
  ownerDocument.addEventListener("visibilitychange", onVisibility)

  // A screen change can remove the focused control; keep keys working by
  // returning focus to the reader rather than leaving it on the page body.
  let lastView = controller.state.view
  const unsubscribe = controller.subscribe((state) => {
    if (state.view === lastView) return
    lastView = state.view
    queueMicrotask(() => {
      const active = ownerDocument.activeElement
      if (!active || active === ownerDocument.body || (!root.contains(active) && !active.isConnected)) root.focus({ preventScroll: true })
    })
  })

  const dispose = render(() => createComponent(ReaderView, { doc, controller, closable: Boolean(options.onClose) }), root)
  if (options.autoFocus !== false) root.focus({ preventScroll: true })

  let destroyed = false
  return {
    controller,
    setTheme,
    destroy() {
      if (destroyed) return
      destroyed = true
      unsubscribe()
      controller.dispose()
      ownerDocument.removeEventListener("visibilitychange", onVisibility)
      root.removeEventListener("keydown", onKey)
      dispose()
      root.remove()
      ;(session as { free?(): void }).free?.()
    },
  }
}
