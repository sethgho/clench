// Builds the standalone `meatread` executable for this machine into
// dist/meatread, with the WebAssembly engine embedded.
import solidPlugin from "@opentui/solid/bun-plugin"
import { existsSync } from "node:fs"
import { join } from "node:path"

const root = join(import.meta.dir, "../../..")
const wasm = join(root, "crates/meatread-wasm/pkg/meatread_bg.wasm")
if (!existsSync(wasm)) {
  console.error("build: the engine is not built; run `bun run build:wasm` first")
  process.exit(1)
}

const outfile = join(root, "dist/meatread")
const result = await Bun.build({
  entrypoints: [join(import.meta.dir, "../src/main.tsx")],
  target: "bun",
  minify: true,
  plugins: [solidPlugin],
  // The binary must behave the same wherever it runs, so it ignores any
  // bunfig.toml, .env, tsconfig.json or package.json in the working directory.
  compile: { outfile, autoloadBunfig: false, autoloadDotenv: false, autoloadTsconfig: false, autoloadPackageJson: false },
})
if (!result.success) {
  for (const log of result.logs) console.error(log)
  process.exit(1)
}
console.log(`build: ${outfile}`)
