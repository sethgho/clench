export type ManifestMode = "production" | "e2e"

export interface Manifest {
  manifest_version: 3
  name: string
  version: string
  description: string
  minimum_chrome_version: string
  icons: Record<string, string>
  action: { default_title: string; default_icon: Record<string, string> }
  background: { service_worker: string; type: "module" }
  commands: Record<string, { suggested_key?: { default: string }; description?: string }>
  permissions: string[]
  host_permissions?: string[]
  web_accessible_resources: Array<{ resources: string[]; matches: string[] }>
  content_security_policy: { extension_pages: string }
}

/** Where the engine runs: the extension page may compile WebAssembly, and nothing else is loosened. */
export const EXTENSION_PAGES_CSP = "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'; base-uri 'none'"

/**
 * The extension manifest. The `e2e` mode adds a host permission for the loopback
 * test server, since a browser test cannot click the toolbar button that grants
 * `activeTab`; it is never shipped.
 */
export function buildManifest(version: string, mode: ManifestMode = "production"): Manifest {
  const icons = { "32": "icons/icon-32.png", "48": "icons/icon-48.png", "128": "icons/icon-128.png" }
  return {
    manifest_version: 3,
    name: "clench",
    version,
    description: "Read the page you are on one word at a time. Runs entirely in your browser; nothing is sent anywhere.",
    minimum_chrome_version: "114",
    icons,
    action: { default_title: "Read this page with clench", default_icon: icons },
    background: { service_worker: "background.js", type: "module" },
    commands: {
      _execute_action: { suggested_key: { default: "Alt+Shift+R" } },
    },
    permissions: ["activeTab", "scripting", "storage"],
    ...(mode === "e2e" ? { host_permissions: ["http://127.0.0.1/*"] } : {}),
    web_accessible_resources: [{ resources: ["reader.html"], matches: ["<all_urls>"] }],
    content_security_policy: { extension_pages: EXTENSION_PAGES_CSP },
  }
}
