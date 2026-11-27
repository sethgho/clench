import { basename } from "node:path"
import { fetchBytes, forcedContentType, isUrl, LoadError, messageOf, typeForName, type DocType, type Loaded } from "@clench/host"

export { DOC_TYPES, forcedContentType, isUrl, LoadError, MAX_BYTES, type DocType, type Loaded } from "@clench/host"

/**
 * Sent with every request. Generic on purpose: it names no person, machine
 * or program version.
 */
export const USER_AGENT =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"

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
  return fetchBytes(url, {
    fetcher: options.fetcher,
    maxBytes: options.maxBytes,
    headerTimeoutMs: options.headerTimeoutMs,
    totalTimeoutMs: options.totalTimeoutMs,
    describeStatus,
    describeFailure: describeNetworkError,
    headers: {
      "User-Agent": USER_AGENT,
      Accept: "text/html,application/xhtml+xml,text/markdown,application/pdf;q=0.9,*/*;q=0.8",
      "Accept-Language": "en-US,en;q=0.9",
    },
  })
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
  return { bytes, contentType: typeForName(basename(path)), url: "" }
}

async function readStdin(): Promise<Uint8Array> {
  try {
    return await Bun.stdin.bytes()
  } catch (error) {
    throw new LoadError(`reading standard input failed: ${messageOf(error)}`)
  }
}
