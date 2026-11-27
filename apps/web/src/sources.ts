import { detectTextType, fetchBytes, isUrl, typeForFile, typeForUrl, type Loaded } from "@clench/host"

/** What the page was opened with: a deep link, a share, or nothing. */
export type Launch =
  | { kind: "text"; text: string; title?: string }
  | { kind: "url"; url: string }
  | { kind: "none" }

/**
 * Reads a launch request from the page address. `#text=` carries text in the
 * fragment, which browsers never send to the server. `?url=` asks for a fetch.
 * A Web Share Target request arrives as `?title=&text=&url=`; many Android
 * apps put the link in `text`, so a `text` that is only a URL is treated as one.
 */
export function parseLaunch(href: string): Launch {
  const address = new URL(href)
  const fragment = new URLSearchParams(address.hash.replace(/^#/, ""))
  const fragmentText = fragment.get("text")
  if (fragmentText) return { kind: "text", text: fragmentText }
  const query = address.searchParams
  const url = query.get("url")?.trim()
  if (url && isUrl(url)) return { kind: "url", url }
  const text = query.get("text")?.trim()
  if (text) {
    if (isUrl(text) && !/\s/.test(text)) return { kind: "url", url: text }
    const title = query.get("title")?.trim()
    return { kind: "text", text, ...(title ? { title } : {}) }
  }
  return { kind: "none" }
}

/** Pasted or shared text as extractor input, typed by `type` or by detection. */
export function fromText(text: string, type: string | undefined): Loaded {
  return { bytes: new TextEncoder().encode(text), contentType: type || detectTextType(text), url: "" }
}

/** A picked or dropped file as extractor input. */
export async function fromFile(file: Pick<File, "name" | "type" | "arrayBuffer">): Promise<Loaded> {
  return { bytes: new Uint8Array(await file.arrayBuffer()), contentType: typeForFile(file.name, file.type), url: "" }
}

const NO_SERVER =
  "clench runs entirely in your browser and has no server to fetch pages for it, so it can only read sites that allow other web pages to read them."
const INSTEAD =
  "Open the page yourself, then paste its text here, or save it (Ctrl+S or ⌘S) and drop the file here. A browser extension that reads the open tab is planned."

/** The reader-facing sentence for a fetch that failed before any answer. */
export function describeFetchFailure(url: string, online: boolean): string {
  const host = new URL(url).host
  if (!online) return `You appear to be offline, so ${host} could not be reached.`
  return `The browser did not let clench read ${host}. Most sites block reading from other web pages (CORS), and some are simply unreachable; the browser does not say which. ${NO_SERVER} ${INSTEAD}`
}

/** The reader-facing sentence for a non-2xx answer. */
export function describeHttpStatus(status: number, url: string): string {
  if (status === 401 || status === 403) {
    return `${url} answered HTTP ${status}: the site refused the request. ${INSTEAD}`
  }
  if (status === 404 || status === 410) return `${url} answered HTTP ${status}: no such page.`
  if (status === 429) return `${url} answered HTTP 429: too many requests, try again later.`
  if (status >= 500) return `${url} answered HTTP ${status}: the server had an error.`
  return `${url} answered HTTP ${status}.`
}

/**
 * Fetches `url` directly from the browser, with no custom headers (they
 * would trigger a CORS preflight most servers refuse) and no credentials.
 */
export async function fromUrl(
  url: string,
  options: { fetcher?: typeof fetch; online?: () => boolean } = {},
): Promise<Loaded> {
  const base = options.fetcher ?? ((input, init) => fetch(input, init))
  const fetcher = ((input: RequestInfo | URL, init?: RequestInit) =>
    base(input, { ...init, mode: "cors", credentials: "omit", referrerPolicy: "no-referrer" })) as typeof fetch
  const loaded = await fetchBytes(url, {
    fetcher,
    describeStatus: describeHttpStatus,
    describeFailure: (u) => describeFetchFailure(u, options.online?.() ?? true),
  })
  return { ...loaded, contentType: typeForUrl(loaded.url || url, loaded.contentType) }
}

/** A storage that may be missing or throw (private windows, blocked site data). */
export interface StorageLike {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

export type ThemeChoice = "auto" | "light" | "dark"

export interface Preferences {
  wpm: number
  theme: ThemeChoice
}

export const DEFAULT_PREFERENCES: Preferences = { wpm: 300, theme: "auto" }
const KEY = "clench.preferences"

export function loadPreferences(storage: () => StorageLike | undefined): Preferences {
  try {
    const raw = storage()?.getItem(KEY)
    if (!raw) return { ...DEFAULT_PREFERENCES }
    const parsed = JSON.parse(raw) as Partial<Preferences>
    const wpm = Number.isFinite(parsed.wpm) && parsed.wpm! >= 50 && parsed.wpm! <= 2000 ? Math.round(parsed.wpm!) : DEFAULT_PREFERENCES.wpm
    const theme = parsed.theme === "light" || parsed.theme === "dark" ? parsed.theme : "auto"
    return { wpm, theme }
  } catch {
    return { ...DEFAULT_PREFERENCES }
  }
}

export function savePreferences(storage: () => StorageLike | undefined, preferences: Preferences): void {
  try {
    storage()?.setItem(KEY, JSON.stringify(preferences))
  } catch {}
}
