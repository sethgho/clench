/** @jsxImportSource solid-js */
import { ExtractError, extractWith, isEngineCrash, LoadError, messageOf, type Loaded } from "@clench/host"
import type { Document } from "@clench/host/engine-types"
import { engineVersion, extract, loadEngine, mount, type ReaderHandle } from "@clench/reader-web"
import { createSignal, onCleanup, onMount, Show } from "solid-js"
import { render } from "solid-js/web"
import wasmUrl from "../../../crates/clench-wasm/pkg/clench_bg.wasm?url"
import "./app.css"
import { fromFile, fromText, fromUrl, loadPreferences, parseLaunch, savePreferences, type Preferences, type ThemeChoice } from "./sources"

const storage = () => {
  try {
    return window.localStorage
  } catch {
    return undefined
  }
}

const THEME_LABEL: Record<ThemeChoice, string> = { auto: "Theme: system", light: "Theme: light", dark: "Theme: dark" }
const NEXT_THEME: Record<ThemeChoice, ThemeChoice> = { auto: "light", light: "dark", dark: "auto" }

function applyTheme(theme: ThemeChoice): void {
  if (theme === "auto") document.documentElement.removeAttribute("data-theme")
  else document.documentElement.setAttribute("data-theme", theme)
}

function App() {
  const [prefs, setPrefs] = createSignal<Preferences>(loadPreferences(storage))
  const [ready, setReady] = createSignal(false)
  const [busy, setBusy] = createSignal<string | undefined>()
  const [error, setError] = createSignal<string | undefined>()
  const [reading, setReading] = createSignal(false)
  const [dragging, setDragging] = createSignal(false)
  const [crashed, setCrashed] = createSignal(false)
  let readerHost!: HTMLDivElement
  let pasteBox!: HTMLTextAreaElement
  let typeSelect!: HTMLSelectElement
  let urlBox!: HTMLInputElement
  let fileInput!: HTMLInputElement
  let reader: ReaderHandle | undefined
  let returnFocus: HTMLElement | undefined

  const update = (next: Partial<Preferences>) => {
    const merged = { ...prefs(), ...next }
    setPrefs(merged)
    savePreferences(storage, merged)
  }
  applyTheme(prefs().theme)

  const cycleTheme = () => {
    const theme = NEXT_THEME[prefs().theme]
    update({ theme })
    applyTheme(theme)
    reader?.setTheme(theme)
  }

  const close = () => {
    reader?.destroy()
    reader = undefined
    setReading(false)
    document.title = "clench"
    ;(returnFocus ?? pasteBox)?.focus()
  }

  const open = (loaded: Loaded, fallbackTitle?: string) => {
    setError(undefined)
    if (crashed()) {
      setError("The reading engine crashed earlier. Reload the page to read something else.")
      return
    }
    let doc: Document
    try {
      doc = extractWith(extract, loaded.bytes, loaded.contentType, loaded.url || undefined)
    } catch (e) {
      if (isEngineCrash(e)) setCrashed(true)
      setError(`Could not read this: ${e instanceof ExtractError ? e.message : messageOf(e)}`)
      return
    }
    if (!doc.title && fallbackTitle) doc = { ...doc, title: fallbackTitle }
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : undefined
    reader?.destroy()
    setReading(true)
    try {
      reader = mount(readerHost, doc, {
        wpm: prefs().wpm,
        theme: prefs().theme,
        onClose: close,
        onWpmChange: (wpm) => update({ wpm }),
      })
    } catch (e) {
      if (isEngineCrash(e)) setCrashed(true)
      setReading(false)
      setError(`Could not start reading: ${messageOf(e)}`)
      return
    }
    if (reader.controller.session.wordCount === 0) {
      reader.destroy()
      reader = undefined
      setReading(false)
      setError("Nothing to read: no readable text was found.")
      return
    }
    document.title = `${doc.title || "Untitled"} · clench`
  }

  const readPaste = () => {
    const text = pasteBox.value
    if (!text.trim()) {
      setError("Paste some text, HTML or Markdown first.")
      pasteBox.focus()
      return
    }
    open(fromText(text, typeSelect.value || undefined))
  }

  const readFile = async (file: File | undefined) => {
    if (!file) return
    setBusy(`Reading ${file.name} …`)
    try {
      open(await fromFile(file), file.name.replace(/\.[^.]+$/, ""))
    } catch (e) {
      setError(`Could not open ${file.name}: ${messageOf(e)}`)
    } finally {
      setBusy(undefined)
    }
  }

  const readUrl = async (url: string) => {
    setError(undefined)
    let parsed: URL
    try {
      parsed = new URL(url)
      if (!/^https?:$/.test(parsed.protocol)) throw new Error("not http")
    } catch {
      setError("Enter a full web address starting with https://")
      return
    }
    setBusy(`Fetching ${parsed.host} …`)
    try {
      open(await fromUrl(parsed.href, { online: () => navigator.onLine }))
    } catch (e) {
      setError(e instanceof LoadError ? e.message : messageOf(e))
    } finally {
      setBusy(undefined)
    }
  }

  const onDrop = (e: DragEvent) => {
    e.preventDefault()
    setDragging(false)
    if (reading()) return
    void readFile(e.dataTransfer?.files?.[0])
  }
  const onDragOver = (e: DragEvent) => {
    if (!e.dataTransfer?.types.includes("Files")) return
    e.preventDefault()
    if (!reading()) setDragging(true)
  }
  const onDragLeave = (e: DragEvent) => {
    if (e.relatedTarget === null) setDragging(false)
  }
  onMount(() => {
    window.addEventListener("dragover", onDragOver)
    window.addEventListener("drop", onDrop)
    window.addEventListener("dragleave", onDragLeave)
  })
  onCleanup(() => {
    window.removeEventListener("dragover", onDragOver)
    window.removeEventListener("drop", onDrop)
    window.removeEventListener("dragleave", onDragLeave)
  })

  onMount(async () => {
    try {
      await loadEngine(wasmUrl)
    } catch (e) {
      setError(`The reading engine could not be loaded: ${messageOf(e)}`)
      return
    }
    setReady(true)
    await launch()
    window.addEventListener("hashchange", launch)
  })
  onCleanup(() => window.removeEventListener("hashchange", launch))

  const launch = async () => {
    const request = parseLaunch(location.href)
    if (request.kind === "text") {
      pasteBox.value = request.text
      open(fromText(request.text, undefined), request.title)
    } else if (request.kind === "url") {
      urlBox.value = request.url
      await readUrl(request.url)
    }
  }

  return (
    <>
      <main class="shell" hidden={reading()} aria-hidden={reading() ? "true" : undefined}>
        <header class="masthead">
          <img src={`${import.meta.env.BASE_URL}icons/logo-480.png`} width="240" height="160" alt="" class="logo" />
          <div>
            <h1>clench</h1>
            <p class="tagline">Read the post yourself, one word at a time.</p>
          </div>
          <button type="button" class="theme" onClick={cycleTheme} aria-label={`${THEME_LABEL[prefs().theme]}. Change theme`}>
            {THEME_LABEL[prefs().theme]}
          </button>
        </header>

        <Show when={error()}>
          <p class="error" role="alert">
            {error()}
          </p>
        </Show>
        <p class="busy" role="status" aria-live="polite">
          {busy() ?? (ready() ? "" : "Loading the reading engine …")}
        </p>

        <section aria-labelledby="paste-h" class="panel">
          <h2 id="paste-h">Paste</h2>
          <label for="paste">Text, HTML or Markdown</label>
          <textarea id="paste" ref={pasteBox} rows="8" spellcheck={false} placeholder="Paste an article here" />
          <div class="row">
            <label for="type" class="inline">
              Read as
            </label>
            <select id="type" ref={typeSelect}>
              <option value="">detect automatically</option>
              <option value="text/html">HTML</option>
              <option value="text/markdown">Markdown</option>
              <option value="text/plain">plain text</option>
            </select>
            <button type="button" class="primary" disabled={!ready()} onClick={readPaste}>
              Read
            </button>
          </div>
        </section>

        <section aria-labelledby="file-h" class="panel">
          <h2 id="file-h">Open a file</h2>
          <div class={`drop${dragging() ? " over" : ""}`}>
            <p>Drop an HTML, Markdown, text or PDF file anywhere on this page, or</p>
            <label class="button" for="file">
              Choose a file
            </label>
            <input
              id="file"
              ref={fileInput}
              type="file"
              class="visually-hidden"
              accept=".html,.htm,.xhtml,.md,.markdown,.txt,.pdf,text/html,text/markdown,text/plain,application/pdf"
              disabled={!ready()}
              onChange={() => {
                void readFile(fileInput.files?.[0])
                fileInput.value = ""
              }}
            />
          </div>
        </section>

        <section aria-labelledby="url-h" class="panel">
          <h2 id="url-h">Open a web address</h2>
          <form
            class="row"
            onSubmit={(e) => {
              e.preventDefault()
              void readUrl(urlBox.value.trim())
            }}
          >
            <label for="url" class="visually-hidden">
              Web address
            </label>
            <input id="url" ref={urlBox} type="url" inputmode="url" placeholder="https://…" autocomplete="url" />
            <button type="submit" class="primary" disabled={!ready() || Boolean(busy())}>
              Fetch
            </button>
          </form>
          <p class="note">
            This only works for sites that let other web pages read them, such as raw files on GitHub. Most article pages do not, and
            clench has no server to fetch them for you; paste or drop the page instead.
          </p>
        </section>

        <footer class="colophon">
          <p>
            Everything runs in your browser: nothing you read is sent anywhere, and there is no AI. The text is the author's, never
            rewritten.
          </p>
          <p>
            <a href="https://github.com/sethgho/clench">Source on GitHub</a> · engine {ready() ? engineVersion() : "…"}
          </p>
        </footer>
      </main>
      <div class="reader-host" ref={readerHost} hidden={!reading()} />
      <Show when={dragging()}>
        <div class="drop-overlay" aria-hidden="true">
          Drop to read
        </div>
      </Show>
    </>
  )
}

render(() => <App />, document.getElementById("app")!)

if (import.meta.env.PROD && "serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register(`${import.meta.env.BASE_URL}sw.js`).catch(() => {})
  })
}
