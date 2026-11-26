import { readFileSync } from "node:fs"
import wasmPath from "../../../crates/meatread-wasm/pkg/meatread_bg.wasm" with { type: "file" }
import { initSync } from "../../../crates/meatread-wasm/pkg/meatread.js"

export { extract, Session, version as engineVersion } from "../../../crates/meatread-wasm/pkg/meatread.js"
export type { Context, Document, Frame, PlayState, Settings } from "../../../crates/meatread-wasm/pkg/meatread.js"

let loaded = false

/** Instantiates the engine once. The module is embedded in compiled builds. */
export function loadEngine(): void {
  if (loaded) return
  // A `type: "file"` import evaluates to the file's path, but TypeScript
  // types it from the generated `meatread_bg.wasm.d.ts` next to it.
  initSync({ module: readFileSync(wasmPath as unknown as string) })
  loaded = true
}

/** Whether `error` means the engine instance crashed and must not be reused. */
export function isEngineCrash(error: unknown): boolean {
  if (error instanceof WebAssembly.RuntimeError) return true
  return error instanceof Error && error.message.startsWith("meatread engine crashed:")
}
