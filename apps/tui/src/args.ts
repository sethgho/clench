export const USAGE = `meatread: speed-read any link yourself, one word at a time.

Usage:
  meatread <url | file | -> [--wpm <n>]

  <url>       an http(s) address to fetch
  <file>      a saved HTML, Markdown, text or PDF file
  -           read the document from standard input

Options:
  --wpm <n>   words per minute (default 300, range 100-1500)
  -h, --help  show this help
  -V, --version
              show the version

Keys while reading: space play/pause, arrows navigate and change speed,
? shows all keys, q quits.`

export type Args =
  | { kind: "help" }
  | { kind: "version" }
  | { kind: "read"; source: string; wpm: number }
  | { kind: "error"; message: string }

export const DEFAULT_WPM = 300

export function parseArgs(argv: readonly string[]): Args {
  let source: string | undefined
  let wpm = DEFAULT_WPM
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
    } else if (arg.startsWith("-") && arg !== "-") {
      return { kind: "error", message: `unknown option ${arg}` }
    } else if (source === undefined) {
      source = arg
    } else {
      return { kind: "error", message: `unexpected argument ${arg}` }
    }
  }
  if (source === undefined) return { kind: "error", message: "nothing to read: give a URL, a file or -" }
  return { kind: "read", source, wpm }
}
