import type { BlockKind, Document, Omitted } from "./engine-types"
import * as fmt from "./format"

const NOUNS: Record<BlockKind, { one: string; many: string }> = {
  heading: { one: "heading", many: "headings" },
  paragraph: { one: "paragraph", many: "paragraphs" },
  list_item: { one: "list item", many: "list items" },
  blockquote: { one: "quote", many: "quotes" },
  code: { one: "code block", many: "code blocks" },
  table: { one: "table", many: "tables" },
  figure: { one: "figure", many: "figures" },
  caption: { one: "caption", many: "captions" },
  footnote: { one: "footnote", many: "footnotes" },
  rule: { one: "rule", many: "rules" },
}

/**
 * What the script leaves out, for the ready screen: blocks shown only as a
 * marker are labelled, blocks dropped silently are not. Blocks with no words
 * in them (a horizontal rule) are not worth a mention.
 */
export function notRead(omitted: Omitted[]): string | undefined {
  const noun = (kind: BlockKind, n: number) => `${fmt.count(n)} ${(n === 1 ? NOUNS[kind].one : NOUNS[kind].many)}`
  const parts: string[] = []
  for (const o of omitted) {
    if (o.marked > 0 && o.marked_words > 0) parts.push(`${noun(o.kind, o.marked)} (marked)`)
    if (o.skipped > 0 && o.skipped_words > 0) parts.push(noun(o.kind, o.skipped))
  }
  return parts.length > 0 ? `not read: ${parts.join(", ")}` : undefined
}

/** The finished screen's summary line; a rate only for a straight read. */
export function finishedSummary(words: number, playedMs: number, straight: boolean): string {
  const rate = playedMs > 0 ? Math.round(words / (playedMs / 60_000)) : 0
  if (straight && rate > 0) return `${fmt.count(words)} words in ${fmt.clock(playedMs)} · ${fmt.count(rate)} wpm overall`
  return `${fmt.count(words)} words in the document · ${fmt.clock(playedMs)} spent reading`
}

/** Byline and site name for the ready screen, `·`-separated; empty when neither is known. */
export function siteLabel(doc: Document): string {
  const parts: string[] = []
  if (doc.byline) parts.push(doc.byline)
  let site = doc.site_name ?? ""
  if (!site && doc.url) {
    try {
      site = new URL(doc.url).hostname.replace(/^www\./, "")
    } catch {}
  }
  if (site && site !== doc.byline) parts.push(site)
  return parts.join(" · ")
}

/**
 * Honest warnings about what the document may lack, as lines broken where a
 * terminal reader breaks them; join them with spaces for flowing text.
 */
export function warnings(doc: Document): string[] {
  if (doc.source === "citation_abstract") {
    return ["Abstract only: the full text was not on this page, only the paper's abstract."]
  }
  if (doc.partial) {
    return [
      "Only part of this page could be extracted. The rest may be loaded by",
      "JavaScript or sit behind a paywall, so this may not be the whole article.",
    ]
  }
  if (doc.paywall_hint) return ["The page is marked as paywalled; check that the text is complete."]
  return []
}

