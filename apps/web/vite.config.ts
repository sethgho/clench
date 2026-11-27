import { createHash } from "node:crypto"
import { readdirSync, readFileSync, statSync } from "node:fs"
import { join, relative } from "node:path"
import { defineConfig, type Plugin } from "vite"
import solid from "vite-plugin-solid"

const here = import.meta.dirname

function publicFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name)
    return statSync(path).isDirectory() ? publicFiles(path) : [path]
  })
}

/**
 * Emits `sw.js` with every built file and public asset in its precache list
 * and a cache name derived from their contents, so each deploy that changes
 * anything gets a fresh cache and old ones are dropped.
 */
function serviceWorker(): Plugin {
  return {
    name: "clench-service-worker",
    apply: "build",
    generateBundle(_options, bundle) {
      const hash = createHash("sha256")
      const files: string[] = []
      for (const [name, item] of Object.entries(bundle).sort(([a], [b]) => a.localeCompare(b))) {
        files.push(name)
        hash.update(name).update(item.type === "chunk" ? item.code : item.source)
      }
      const pub = join(here, "public")
      for (const path of publicFiles(pub).sort()) {
        const name = relative(pub, path).split("\\").join("/")
        files.push(name)
        hash.update(name).update(readFileSync(path))
      }
      const template = readFileSync(join(here, "sw.template.js"), "utf8")
      const list = ["./", ...files.filter((f) => f !== "index.html" && !f.endsWith(".map"))].map((f) => (f === "./" ? f : `./${f}`))
      const source = template
        .replace("__CACHE__", `clench-${hash.digest("hex").slice(0, 16)}`)
        .replace("__FILES__", JSON.stringify(list))
      this.emitFile({ type: "asset", fileName: "sw.js", source })
    },
  }
}

export default defineConfig({
  root: here,
  base: process.env.CLENCH_BASE ?? "/clench/",
  plugins: [solid(), serviceWorker()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    assetsInlineLimit: 0,
  },
})
