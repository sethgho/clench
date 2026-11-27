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

export const MAX_BYTES = 32 * 1024 * 1024
/** How long the server may take to start answering. */
export const HEADER_TIMEOUT_MS = 30_000
/** How long the whole download may take. */
export const TOTAL_TIMEOUT_MS = 90_000

export interface FetchOptions {
  fetcher?: typeof fetch
  /** Request headers. Browsers forbid some and preflight others; a page should send none. */
  headers?: Record<string, string>
  /** The reader-facing sentence for a non-2xx answer. */
  describeStatus: (status: number, url: string) => string
  /** The reader-facing sentence for a request that failed before any answer. */
  describeFailure: (url: string, error: unknown) => string
  maxBytes?: number
  headerTimeoutMs?: number
  totalTimeoutMs?: number
}

/**
 * Downloads `url`, bounded in size and time, and returns its bytes with the
 * response's Content-Type and final address. Every failure is a `LoadError`
 * whose message is meant for the reader.
 */
export async function fetchBytes(url: string, options: FetchOptions): Promise<Loaded> {
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
      response = await fetcher(url, { redirect: "follow", signal: abort.signal, headers: options.headers })
    } catch (error) {
      throw new LoadError(timedOut ?? options.describeFailure(url, error))
    }
    clearTimeout(headerTimer)
    if (!response.ok) {
      await response.body?.cancel().catch(() => {})
      throw new LoadError(options.describeStatus(response.status, url))
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

export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
