import { extname } from "node:path"

/** Raw input for the extractor. Text is never decoded here. */
export interface Loaded {
  bytes: Uint8Array
  contentType: string | undefined
  url: string
}

/** A failure to load, with a message meant for the reader. */
export class LoadError extends Error {
  override name = "LoadError"
}

/**
 * Sent with every request. Generic on purpose: it names no person, machine
 * or program version.
 */
export const USER_AGENT =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"

export const MAX_BYTES = 32 * 1024 * 1024
/** How long the server may take to start answering. */
const HEADER_TIMEOUT_MS = 30_000
/** How long the whole download may take. */
const TOTAL_TIMEOUT_MS = 90_000

/** A document type named on the command line. */
export type DocType = "html" | "md" | "text" | "pdf"

export const DOC_TYPES: readonly DocType[] = ["html", "md", "text", "pdf"]

const MEDIA_TYPES: Record<DocType, string> = {
  html: "text/html",
  md: "text/markdown",
  text: "text/plain",
  pdf: "application/pdf",
}

export interface LoadOptions {
  /** Overrides the type the source or the server declares. */
  type?: DocType
  fetcher?: typeof fetch
  /** Reads standard input; replaced in tests. */
  stdin?: () => Promise<Uint8Array>
  maxBytes?: number
  headerTimeoutMs?: number
  totalTimeoutMs?: number
}

export function isUrl(source: string): boolean {
  return /^https?:\/\//i.test(source)
}

/**
 * The Content-Type to hand the extractor: the forced type's media type,
 * keeping any charset the source declared.
 */
export function forcedContentType(type: DocType, declared: string | undefined): string {
  const charset = declared?.match(/;\s*charset\s*=\s*("[^"]*"|[^;\s]+)/i)?.[0]
  return MEDIA_TYPES[type] + (charset && type !== "pdf" ? charset : "")
}

/** Loads an http(s) URL, a file path, or standard input (`-`). */
export async function load(source: string, options: LoadOptions = {}): Promise<Loaded> {
  const loaded = isUrl(source)
    ? await fetchUrl(source, options)
    : source === "-"
      ? { bytes: await (options.stdin ?? readStdin)(), contentType: undefined, url: "" }
      : await readLocal(source)
  if (options.type) loaded.contentType = forcedContentType(options.type, loaded.contentType)
  return loaded
}

async function fetchUrl(url: string, options: LoadOptions): Promise<Loaded> {
  const fetcher = options.fetcher ?? fetch
  const max = options.maxBytes ?? MAX_BYTES
  const headerMs = options.headerTimeoutMs ?? HEADER_TIMEOUT_MS
  const totalMs = options.totalTimeoutMs ?? TOTAL_TIMEOUT_MS
  const host = new URL(url).host
  const abort = new AbortController()
  let timedOut: string | undefined
  const expire = (message: string) => () => {
    timedOut = message
    abort.abort()
  }
  const headerTimer = setTimeout(expire(`${host} did not answer within ${seconds(headerMs)}`), headerMs)
  const totalTimer = setTimeout(
    expire(`downloading ${url} did not finish within ${seconds(totalMs)}`),
    totalMs,
  )
  try {
    let response: Response
    try {
      response = await fetcher(url, {
        redirect: "follow",
        signal: abort.signal,
        headers: {
          "User-Agent": USER_AGENT,
          Accept: "text/html,application/xhtml+xml,text/markdown,application/pdf;q=0.9,*/*;q=0.8",
          "Accept-Language": "en-US,en;q=0.9",
        },
      })
    } catch (error) {
      throw new LoadError(timedOut ?? describeNetworkError(url, error))
    }
    clearTimeout(headerTimer)
    if (!response.ok) {
      await response.body?.cancel().catch(() => {})
      throw new LoadError(describeStatus(response.status, url))
    }
    const tooLarge = (size: string) => new LoadError(`${url} is too large to read (${size}; the limit is ${megabytes(max)})`)
    const declared = Number(response.headers.get("content-length") ?? 0)
    if (declared > max) {
      await response.body?.cancel().catch(() => {})
      throw tooLarge(megabytes(declared))
    }
    const chunks: Uint8Array[] = []
    let total = 0
    try {
      const reader = response.body?.getReader()
      for (;;) {
        const part = reader ? await reader.read() : { done: true as const, value: undefined }
        if (part.done) break
        total += part.value.length
        if (total > max) {
          await reader!.cancel().catch(() => {})
          throw tooLarge(`more than ${megabytes(max)}`)
        }
        chunks.push(part.value)
      }
    } catch (error) {
      if (error instanceof LoadError) throw error
      throw new LoadError(timedOut ?? `reading the response from ${url} failed: ${messageOf(error)}`)
    }
    const bytes = new Uint8Array(total)
    let at = 0
    for (const chunk of chunks) {
      bytes.set(chunk, at)
      at += chunk.length
    }
    return {
      bytes,
      contentType: response.headers.get("content-type") ?? undefined,
      url: response.url || url,
    }
  } finally {
    clearTimeout(headerTimer)
    clearTimeout(totalTimer)
  }
}

function seconds(ms: number): string {
  if (ms < 1000) return `${ms} ms`
  const s = ms / 1000
  return `${Number.isInteger(s) ? s : s.toFixed(1)} seconds`
}

function megabytes(bytes: number): string {
  return `${Math.round(bytes / (1024 * 1024))} MB`
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function describeNetworkError(url: string, error: unknown): string {
  const err = error as { name?: string; code?: string } | undefined
  const host = new URL(url).host
  switch (err?.code) {
    case "ConnectionRefused":
    case "ECONNREFUSED":
      return `${host} refused the connection`
    case "FailedToOpenSocket":
    case "ENOTFOUND":
    case "EAI_NONAME":
      return `could not reach ${host}: check the address and your connection`
  }
  if (/certificate|TLS|SSL/i.test(messageOf(error))) {
    return `the secure connection to ${host} failed: ${messageOf(error)}`
  }
  return `could not fetch ${url}: ${messageOf(error)}`
}

export function describeStatus(status: number, url: string): string {
  if (status === 401 || status === 403) {
    return (
      `${url} answered HTTP ${status}: the site refused the request. Many sites block ` +
      `programs that are not a full web browser, or need a login; clench does not try to get around that. ` +
      `If you can open the page in a browser, save it and run clench on the file.`
    )
  }
  if (status === 404 || status === 410) return `${url} answered HTTP ${status}: no such page`
  if (status === 429) return `${url} answered HTTP 429: too many requests, try again later`
  if (status >= 500) return `${url} answered HTTP ${status}: the server had an error`
  return `${url} answered HTTP ${status}`
}

const TYPES: Record<string, string> = {
  ".html": "text/html",
  ".htm": "text/html",
  ".xhtml": "text/html",
  ".md": "text/markdown",
  ".markdown": "text/markdown",
  ".pdf": "application/pdf",
  ".txt": "text/plain",
}

async function readLocal(path: string): Promise<Loaded> {
  const file = Bun.file(path)
  let bytes: Uint8Array
  try {
    bytes = await file.bytes()
  } catch (error) {
    const code = (error as { code?: string }).code
    if (code === "ENOENT") throw new LoadError(`no such file: ${path}`)
    if (code === "EISDIR") throw new LoadError(`${path} is a directory`)
    if (code === "EACCES") throw new LoadError(`permission denied: ${path}`)
    throw new LoadError(`reading ${path} failed: ${messageOf(error)}`)
  }
  return { bytes, contentType: TYPES[extname(path).toLowerCase()], url: "" }
}

async function readStdin(): Promise<Uint8Array> {
  try {
    return await Bun.stdin.bytes()
  } catch (error) {
    throw new LoadError(`reading standard input failed: ${messageOf(error)}`)
  }
}
