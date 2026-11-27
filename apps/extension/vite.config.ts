import { readFileSync } from "node:fs"
import { join } from "node:path"
import { defineConfig, type Plugin } from "vite"
import solid from "vite-plugin-solid"
import { buildManifest } from "./src/manifest.ts"

const here = import.meta.dirname

function manifest(mode: "production" | "e2e"): Plugin {
  return {
    name: "clench-manifest",
    apply: "build",
    generateBundle() {
      const { version } = JSON.parse(readFileSync(join(here, "package.json"), "utf8")) as { version: string }
      this.emitFile({ type: "asset", fileName: "manifest.json", source: `${JSON.stringify(buildManifest(version, mode), null, 2)}\n` })
    },
  }
}

export default defineConfig(({ mode }) => ({
  root: here,
  base: "./",
  mode,
  plugins: [solid(), manifest(mode === "e2e" ? "e2e" : "production")],
  build: {
    outDir: mode === "e2e" ? "dist-e2e" : "dist",
    emptyOutDir: true,
    target: "es2022",
    assetsInlineLimit: 0,
    modulePreload: false,
    rollupOptions: {
      input: { reader: join(here, "reader.html"), background: join(here, "src/background.ts") },
      output: { entryFileNames: "[name].js", chunkFileNames: "assets/[name]-[hash].js" },
    },
  },
}))
