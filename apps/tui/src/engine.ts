import { readFileSync } from "node:fs"
import wasmPath from "../../../crates/clench-wasm/pkg/clench_bg.wasm" with { type: "file" }
import { extract as rawExtract, initSync } from "../../../crates/clench-wasm/pkg/clench.js"
import type { Document } from "../../../crates/clench-wasm/pkg/clench.js"

export { extract, Session, version as engineVersion } from "../../../crates/clench-wasm/pkg/clench.js"
export type { BlockKind, Context, Document, Frame, Omitted, PlayState, Settings } from "../../../crates/clench-wasm/pkg/clench.js"

let loaded = false

/** Instantiates the engine once. The module is embedded in compiled builds. */
export function loadEngine(): void {
  if (loaded) return
  // A `type: "file"` import evaluates to the file's path, but TypeScript
  // types it from the generated `clench_bg.wasm.d.ts` next to it.
  initSync({ module: readFileSync(wasmPath as unknown as string) })
  loaded = true
}

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
 * Extracts a document. Any failure, including a trap inside the engine, is
 * reported as an `ExtractError`. After a trap the instance is unusable, so
 * the caller must not start a session from it; the reader exits instead.
 */
export function extractDocument(
  bytes: Uint8Array,
  contentType: string | undefined,
  url: string | undefined,
  run: typeof rawExtract = rawExtract,
): Document {
  try {
    return run(bytes, contentType, url)
  } catch (error) {
    throw new ExtractError(describeExtractError(error))
  }
}
