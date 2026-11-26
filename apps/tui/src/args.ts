import { DOC_TYPES, type DocType } from "./source"

export const USAGE = `meatread: speed-read any link yourself, one word at a time.

Usage:
  meatread <url | file | -> [--wpm <n>] [--type <type>]

  <url>       an http(s) address to fetch
  <file>      a saved HTML, Markdown, text or PDF file
  -           read the document from standard input

Options:
  --wpm <n>   words per minute (default 300, range 100-1500)
  --type <t>  read the input as html, md, text or pdf instead of going by
              the server's Content-Type or the file extension; stdin is
              plain text unless told otherwise
  -h, --help  show this help
  -V, --version
              show the version

Keys while reading: space play/pause, arrows navigate and change speed,
? shows all keys, q quits.`

export type Args =
  | { kind: "help" }
  | { kind: "version" }
  | { kind: "read"; source: string; wpm: number; type?: DocType }
  | { kind: "error"; message: string }

export const DEFAULT_WPM = 300

export function parseArgs(argv: readonly string[]): Args {
  let source: string | undefined
  let wpm = DEFAULT_WPM
  let type: DocType | undefined
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!
    if (arg === "-h" || arg === "--help") return { kind: "help" }
    if (arg === "-V" || arg === "--version") return { kind: "version" }
    if (arg === "--wpm" || arg.startsWith("--wpm=")) {
      const value = arg === "--wpm" ? argv[++i] : arg.slice("--wpm=".length)
      if (value === undefined) return { kind: "error", message: "--wpm needs a number" }
      const n = Number(value)
      if (!Number.isInteger(n) || n <= 0) return { kind: "error", message: `--wpm must be a whole number, not "${value}"` }
      wpm = n
    } else if (arg === "--type" || arg.startsWith("--type=")) {
      const value = arg === "--type" ? argv[++i] : arg.slice("--type=".length)
      if (value === undefined) return { kind: "error", message: `--type needs one of ${DOC_TYPES.join(", ")}` }
      if (!(DOC_TYPES as readonly string[]).includes(value)) {
        return { kind: "error", message: `--type must be one of ${DOC_TYPES.join(", ")}, not "${value}"` }
      }
      type = value as DocType
    } else if (arg.startsWith("-") && arg !== "-") {
      return { kind: "error", message: `unknown option ${arg}` }
    } else if (source === undefined) {
      source = arg
    } else {
      return { kind: "error", message: `unexpected argument ${arg}` }
    }
  }
  if (source === undefined) return { kind: "error", message: "nothing to read: give a URL, a file or -" }
  return type ? { kind: "read", source, wpm, type } : { kind: "read", source, wpm }
}
