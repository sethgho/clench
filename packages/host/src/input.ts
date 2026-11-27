/** A document type a reader can be told to use instead of detecting it. */
export type DocType = "html" | "md" | "text" | "pdf"

export const DOC_TYPES: readonly DocType[] = ["html", "md", "text", "pdf"]

export const MEDIA_TYPES: Record<DocType, string> = {
  html: "text/html",
  md: "text/markdown",
  text: "text/plain",
  pdf: "application/pdf",
}

export function isUrl(source: string): boolean {
  return /^https?:\/\//i.test(source)
}

/**
 * The Content-Type to hand the extractor: the forced type's media type,
 * keeping any charset the source declared.
 */
export function forcedContentType(type: DocType, declared: string | undefined): string {
  const charset = declared?.match(/;\s*charset\s*=\s*("[^"]*"|[^;\s]+)/i)?.[0]
  return MEDIA_TYPES[type] + (charset && type !== "pdf" ? charset : "")
}

const EXTENSIONS: Record<string, string> = {
  ".html": "text/html",
  ".htm": "text/html",
  ".xhtml": "text/html",
  ".md": "text/markdown",
  ".markdown": "text/markdown",
  ".pdf": "application/pdf",
  ".txt": "text/plain",
}

/** The media type implied by a file name's extension, if it is one the extractor reads. */
export function typeForName(name: string): string | undefined {
  const dot = name.lastIndexOf(".")
  if (dot <= 0 || dot < name.lastIndexOf("/")) return undefined
  return EXTENSIONS[name.slice(dot).toLowerCase()]
}

/**
 * The media type for a picked or dropped file: the extension first, since
 * browsers report Markdown inconsistently (often as nothing at all), then
 * the type the platform reports if the extractor reads it.
 */
export function typeForFile(name: string, reported: string | undefined): string | undefined {
  const byName = typeForName(name)
  if (byName) return byName
  const media = reported?.split(";")[0]?.trim().toLowerCase()
  if (!media) return undefined
  if (media === "application/xhtml+xml") return "text/html"
  if (media === "text/x-markdown") return "text/markdown"
  return Object.values(MEDIA_TYPES).includes(media) ? media : undefined
}

/**
 * Guesses the type of pasted text. HTML is recognised by a document
 * preamble or by markup that wraps most of the text; Markdown by structure
 * that plain prose rarely has (ATX headings, fenced code, list runs, links).
 * Anything else is plain text, which reads every word as written.
 */
export function detectTextType(text: string): string {
  const head = text.trimStart().slice(0, 2048)
  if (/^(<!doctype html|<html[\s>]|<\?xml[^>]*>\s*<html)/i.test(head)) return MEDIA_TYPES.html
  const tags = text.match(/<\/?(p|div|article|section|h[1-6]|ul|ol|li|br|span|a|em|strong|blockquote|pre|table|body|head|main)\b[^>]*>/gi)
  if (tags && tags.length >= 3 && head.startsWith("<")) return MEDIA_TYPES.html
  let signals = 0
  if (/^#{1,6} \S/m.test(text)) signals += 2
  if (/^(```|~~~)/m.test(text)) signals += 2
  if (/\[[^\]\n]+\]\([^)\s]+\)/.test(text)) signals += 1
  if ((text.match(/^\s{0,3}([-*+]|\d+\.) \S/gm)?.length ?? 0) >= 2) signals += 1
  if (/^> \S/m.test(text)) signals += 1
  if (/(\*\*|__)\S[^*\n]*\S\1/.test(text)) signals += 1
  return signals >= 2 ? MEDIA_TYPES.md : MEDIA_TYPES.text
}

/**
 * The Content-Type to extract a downloaded file with. Static file hosts often
 * serve Markdown as `text/plain` or bytes as `application/octet-stream`; for
 * those (or none at all), a known extension in the URL's path decides, and
 * any declared charset is kept.
 */
export function typeForUrl(url: string, declared: string | undefined): string | undefined {
  const media = declared?.split(";")[0]?.trim().toLowerCase()
  if (media && media !== "text/plain" && media !== "application/octet-stream") return declared
  let path: string
  try {
    path = decodeURIComponent(new URL(url).pathname)
  } catch {
    return declared
  }
  const byName = typeForName(path)
  if (!byName) return declared
  const charset = declared?.match(/;\s*charset\s*=\s*("[^"]*"|[^;\s]+)/i)?.[0]
  return byName + (charset && byName !== MEDIA_TYPES.pdf ? charset : "")
}
