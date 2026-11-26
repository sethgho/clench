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

const MAX_BYTES = 32 * 1024 * 1024
const TIMEOUT_MS = 30_000

export function isUrl(source: string): boolean {
  return /^https?:\/\//i.test(source)
}

/** Loads an http(s) URL, a file path, or standard input (`-`). */
export async function load(source: string, fetcher: typeof fetch = fetch): Promise<Loaded> {
  if (isUrl(source)) return fetchUrl(source, fetcher)
  if (source === "-") return { bytes: await readStdin(), contentType: undefined, url: "" }
  return readLocal(source)
}

async function fetchUrl(url: string, fetcher: typeof fetch): Promise<Loaded> {
  let response: Response
  try {
    response = await fetcher(url, {
      redirect: "follow",
      signal: AbortSignal.timeout(TIMEOUT_MS),
      headers: {
        "User-Agent": USER_AGENT,
        Accept: "text/html,application/xhtml+xml,text/markdown,application/pdf;q=0.9,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
      },
    })
  } catch (error) {
    throw new LoadError(describeNetworkError(url, error))
  }
  if (!response.ok) {
    await response.body?.cancel()
    throw new LoadError(describeStatus(response.status, url))
  }
  const length = Number(response.headers.get("content-length") ?? 0)
  if (length > MAX_BYTES) throw new LoadError(`${url} is too large to read (${length} bytes)`)
  let bytes: Uint8Array
  try {
    bytes = new Uint8Array(await response.arrayBuffer())
  } catch (error) {
    throw new LoadError(`reading the response from ${url} failed: ${messageOf(error)}`)
  }
  if (bytes.length > MAX_BYTES) throw new LoadError(`${url} is too large to read (${bytes.length} bytes)`)
  return {
    bytes,
    contentType: response.headers.get("content-type") ?? undefined,
    url: response.url || url,
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function describeNetworkError(url: string, error: unknown): string {
  const err = error as { name?: string; code?: string } | undefined
  const host = new URL(url).host
  if (err?.name === "TimeoutError") return `${host} did not answer within ${TIMEOUT_MS / 1000} seconds`
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
      `programs that are not a full web browser, or need a login; meatread does not try to get around that. ` +
      `If you can open the page in a browser, save it and run meatread on the file.`
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
