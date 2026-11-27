/** Whether `error` means the engine instance crashed and must not be reused. */
export function isEngineCrash(error: unknown): boolean {
  if (error instanceof WebAssembly.RuntimeError) return true
  return error instanceof Error && error.message.startsWith("clench engine crashed:")
}

/** The extractor could not read the document; the message is for the reader. */
export class ExtractError extends Error {
  override name = "ExtractError"
}

/** Rewrites an engine error as a sentence for the reader. */
export function describeExtractError(error: unknown): string {
  if (isEngineCrash(error)) {
    return "the extractor failed on this document (an internal error), so it cannot be read"
  }
  const message = error instanceof Error ? error.message : String(error)
  const nested = message.match(/nests elements more than (\d+) levels deep/)
  if (nested) {
    const limit = Number(nested[1]).toLocaleString("en-US")
    return `the page is nested more than ${limit} elements deep, far beyond any real article, so it was not read`
  }
  return message
}

/**
 * Runs the engine's `extract`. Any failure, including a trap inside the
 * engine, is reported as an `ExtractError`. After a trap the instance is
 * unusable, so the caller must not start a session from it.
 */
export function extractWith<D>(
  run: (bytes: Uint8Array, contentType?: string | null, url?: string | null) => D,
  bytes: Uint8Array,
  contentType: string | undefined,
  url: string | undefined,
): D {
  try {
    return run(bytes, contentType, url)
  } catch (error) {
    throw new ExtractError(describeExtractError(error))
  }
}
