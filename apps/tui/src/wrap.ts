const graphemes = new Intl.Segmenter(undefined, { granularity: "grapheme" })

/** Terminal columns `text` occupies. */
export function width(text: string): number {
  return Bun.stringWidth(text)
}

/** The longest tail of `text` that fits in `columns`, cut between grapheme clusters. */
export function clipStart(text: string, columns: number): string {
  if (width(text) <= columns) return text
  const parts = Array.from(graphemes.segment(text), (g) => g.segment)
  let used = 0
  let i = parts.length
  while (i > 0) {
    const w = width(parts[i - 1]!)
    if (used + w > columns) break
    used += w
    i--
  }
  return parts.slice(i).join("")
}

/** One wrapped line: its text and where it starts in the source string. */
export interface Line {
  text: string
  start: number
}

/**
 * Wraps `text` to `columns` display columns at whitespace, breaking a word
 * that is wider than a whole line between grapheme clusters. Line breaks in
 * the text are kept. Offsets are JavaScript string indices into `text`.
 */
export function wrap(text: string, columns: number): Line[] {
  const cols = Math.max(1, columns)
  const lines: Line[] = []
  let offset = 0
  for (const para of text.split("\n")) {
    wrapParagraph(para, offset, cols, lines)
    offset += para.length + 1
  }
  return lines
}

function wrapParagraph(text: string, base: number, cols: number, out: Line[]): void {
  let line = ""
  let lineStart = base
  let lineWidth = 0
  const push = () => {
    out.push({ text: line.trimEnd(), start: lineStart })
    line = ""
    lineWidth = 0
  }
  const words = text.matchAll(/(\S+)(\s*)/g)
  let any = false
  for (const m of words) {
    any = true
    const word = m[1]!
    const space = m[2]!
    const at = base + m.index
    const w = width(word)
    if (lineWidth > 0 && lineWidth + w > cols) push()
    if (lineWidth === 0) lineStart = at
    if (w > cols) {
      let piece = ""
      let pieceWidth = 0
      let pieceStart = at
      for (const { segment, index } of graphemes.segment(word)) {
        const gw = width(segment)
        if (pieceWidth > 0 && pieceWidth + gw > cols) {
          out.push({ text: piece, start: pieceStart })
          piece = ""
          pieceWidth = 0
          pieceStart = at + index
        }
        piece += segment
        pieceWidth += gw
      }
      line = piece
      lineWidth = pieceWidth
      lineStart = pieceStart
    } else {
      line += word
      lineWidth += w
    }
    line += space
    lineWidth += width(space)
  }
  if (line.length > 0 || !any) {
    if (!any) lineStart = base
    push()
  }
}

/** A line cut into the parts before, inside and after a highlighted range. */
export interface Highlighted {
  before: string
  mark: string
  after: string
}

/** Splits `line` around the source range `[from, to)`. */
export function highlight(line: Line, from: number, to: number): Highlighted {
  const a = Math.min(Math.max(from - line.start, 0), line.text.length)
  const b = Math.min(Math.max(to - line.start, 0), line.text.length)
  return { before: line.text.slice(0, a), mark: line.text.slice(a, b), after: line.text.slice(b) }
}

/**
 * The slice of `lines` to show in at most `rows` rows so the line holding
 * source offset `at` is visible, roughly a third of the way down.
 */
export function window(lines: Line[], at: number, rows: number): { first: number; lines: Line[] } {
  if (rows <= 0) return { first: 0, lines: [] }
  let target = 0
  for (let i = 0; i < lines.length; i++) if (lines[i]!.start <= at) target = i
  const first = Math.max(0, Math.min(target - Math.floor(rows / 3), lines.length - rows))
  return { first, lines: lines.slice(first, first + rows) }
}
