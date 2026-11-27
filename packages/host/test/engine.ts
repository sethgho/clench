import { readFileSync } from "node:fs"
import { extract, initSync } from "../../../crates/clench-wasm/pkg/clench.js"
import type { Document } from "../src/engine-types"

export { extract, Session } from "../../../crates/clench-wasm/pkg/clench.js"

/** Instantiates the engine from the built package (a no-op once loaded). */
export function loadEngine(): void {
  initSync({ module: readFileSync(new URL("../../../crates/clench-wasm/pkg/clench_bg.wasm", import.meta.url)) })
}

export function docFrom(text: string, contentType = "text/plain"): Document {
  loadEngine()
  return extract(new TextEncoder().encode(text), contentType, "")
}
