/** Messages between the page overlay and the reader frame inside it. */
export type ToReader = { type: "clench:doc"; html: string; url: string }
export type FromReader = { type: "clench:ready" } | { type: "clench:close" }

export function isFromReader(data: unknown): data is FromReader {
  if (typeof data !== "object" || data === null || !("type" in data)) return false
  return data.type === "clench:ready" || data.type === "clench:close"
}

export function isToReader(data: unknown): data is ToReader {
  if (typeof data !== "object" || data === null) return false
  return "type" in data && data.type === "clench:doc" && "html" in data && typeof data.html === "string" && "url" in data && typeof data.url === "string"
}

/** Declared UTF-8 because a JavaScript string re-encodes as UTF-8, whatever the page's own `<meta charset>` says. */
export const HTML_CONTENT_TYPE = "text/html; charset=utf-8"
