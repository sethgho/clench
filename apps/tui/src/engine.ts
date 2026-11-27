import { readFileSync } from "node:fs"
import wasmPath from "../../../crates/clench-wasm/pkg/clench_bg.wasm" with { type: "file" }
import { extract as rawExtract, initSync } from "../../../crates/clench-wasm/pkg/clench.js"
import type { Document } from "../../../crates/clench-wasm/pkg/clench.js"
import { extractWith } from "@clench/host"

export { describeExtractError, ExtractError, isEngineCrash } from "@clench/host"

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
  return extractWith(run, bytes, contentType, url)
}
