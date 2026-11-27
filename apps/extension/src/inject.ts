/**
 * Runs inside the tab (via `chrome.scripting.executeScript({ func })`), so it
 * must not reference anything outside its own body: no imports, no closures.
 * Opens the reader over the page, or removes it when it is already open.
 */
export function toggleOverlay(readerUrl: string): void {
  const KEY = "__clenchOverlayClose"
  const registry = globalThis as unknown as Record<string, unknown>
  const existing = registry[KEY]
  if (typeof existing === "function") {
    existing()
    return
  }

  const html = document.documentElement.outerHTML
  const pageUrl = location.href
  const readerOrigin = new URL(readerUrl).origin
  const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
  const scrollX = window.scrollX
  const scrollY = window.scrollY

  const host = document.createElement("div")
  host.setAttribute("style", "all: initial; position: fixed; inset: 0; z-index: 2147483647; display: block;")
  const shadow = host.attachShadow({ mode: "closed" })
  const frame = document.createElement("iframe")
  frame.setAttribute("title", "clench reader")
  frame.setAttribute("style", "position: fixed; inset: 0; width: 100vw; height: 100vh; border: 0; margin: 0; padding: 0; color-scheme: normal; background: transparent;")
  frame.src = readerUrl
  shadow.appendChild(frame)

  const onMessage = (event: MessageEvent): void => {
    if (event.source !== frame.contentWindow || event.origin !== readerOrigin) return
    const data: unknown = event.data
    if (typeof data !== "object" || data === null || !("type" in data)) return
    if (data.type === "clench:ready") {
      frame.contentWindow?.postMessage({ type: "clench:doc", html, url: pageUrl }, readerOrigin)
      frame.focus()
    } else if (data.type === "clench:close") {
      close()
    }
  }
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") close()
  }

  function close(): void {
    window.removeEventListener("message", onMessage)
    window.removeEventListener("keydown", onKeyDown, true)
    host.remove()
    delete registry[KEY]
    window.scrollTo(scrollX, scrollY)
    if (previousFocus && previousFocus.isConnected) previousFocus.focus({ preventScroll: true })
  }

  window.addEventListener("message", onMessage)
  window.addEventListener("keydown", onKeyDown, true)
  registry[KEY] = close
  document.documentElement.appendChild(host)
}
