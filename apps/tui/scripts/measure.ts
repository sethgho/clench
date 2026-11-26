// Plays a few hundred words at 600, 1000 and 1500 WPM through the real `meatread` command inside a
// pseudo-terminal and compares what reached the terminal with the engine's
// own schedule: total wall-clock time, frames drawn, and the worst lag of
// any word behind its scheduled time.
//
//   bun apps/tui/scripts/measure.ts [--binary dist/meatread] [--words 300]
//
// Exits non-zero if a word is late by more than LAG_LIMIT_MS or frames are
// missing.
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { extract, loadEngine, Session } from "../src/engine"

const LAG_LIMIT_MS = 50
const root = join(import.meta.dir, "../../..")

function option(name: string): string | undefined {
  const i = process.argv.indexOf(name)
  return i >= 0 ? process.argv[i + 1] : undefined
}

const binary = option("--binary")
const wordTarget = Number(option("--words") ?? 300)

/** A plain-text excerpt of the acceptance fixture with about `words` words. */
function excerpt(words: number): string {
  const html = readFileSync(join(root, "fixtures/01-iankduncan.html"))
  const doc = extract(html, "text/html", "")
  const out: string[] = []
  let n = 0
  for (const b of doc.blocks) {
    if (n >= words) break
    if (b.kind !== "paragraph") continue
    out.push(b.text)
    n += b.text.split(/\s+/).length
  }
  return out.join("\n\n")
}

/** The engine's display time of each token from play to finish, ms. */
function schedule(text: string, wpm: number): number[] {
  const s = new Session(extract(new TextEncoder().encode(text), "text/plain", ""), { wpm })
  s.play()
  const out: number[] = []
  for (let w = s.nextChangeIn(); w !== undefined; w = s.nextChangeIn()) {
    s.advance(w)
    out.push(w)
  }
  s.free()
  return out
}

interface Chunk {
  t: number
  text: string
}

async function run(file: string, wpm: number) {
  const chunks: Chunk[] = []
  const decoder = new TextDecoder()
  const cmd = binary ? [join(root, binary)] : [process.execPath, join(root, "apps/tui/src/main.tsx")]
  const proc = Bun.spawn([...cmd, file, "--wpm", String(wpm)], {
    cwd: root,
    env: { ...process.env, TERM: "xterm-256color" },
    terminal: {
      cols: 100,
      rows: 30,
      data: (_term, data) => chunks.push({ t: performance.now(), text: decoder.decode(data, { stream: true }) }),
    },
  })
  const seen = (s: string) => chunks.some((c) => c.text.includes(s))
  const waitFor = async (s: string, timeoutMs: number) => {
    const end = performance.now() + timeoutMs
    while (!seen(s)) {
      if (performance.now() > end) throw new Error(`timed out waiting for ${JSON.stringify(s)}`)
      await Bun.sleep(5)
    }
  }
  await waitFor("space start", 10_000)
  await Bun.sleep(300)
  const before = chunks.length
  const t0 = performance.now()
  proc.terminal!.write(" ")
  await waitFor("Finished.", 10 * 60_000)
  const playing = chunks.slice(before)
  proc.terminal!.write("q")
  const code = await proc.exited
  const all = chunks.map((c) => c.text).join("")
  return { t0, playing, code, restored: all.lastIndexOf("\x1b[?1049l") > all.lastIndexOf("\x1b[?1049h") }
}

loadEngine()
const dir = mkdtempSync(join(tmpdir(), "meatread-measure-"))
const file = join(dir, "excerpt.txt")
const text = excerpt(wordTarget)
writeFileSync(file, text)
let failed = false
try {
  for (const wpm of [600, 1000, 1500]) {
    const plan = schedule(text, wpm)
    const expected = plan.reduce((a, b) => a + b, 0)
    const { t0, playing, code, restored } = await run(file, wpm)
    // Every frame is wrapped in synchronized-output markers; one frame per
    // word change is expected, the first being the play key's own frame.
    const frames: number[] = []
    for (const c of playing) for (const _ of c.text.matchAll(/\x1b\[\?2026l/g)) frames.push(c.t)
    const finished = playing.find((c) => c.text.includes("Finished."))!.t
    const wall = finished - t0
    // Frame k + 1 shows token k + 1, due after the first k + 1 durations.
    let due = 0
    let worst = 0
    const n = Math.min(plan.length, frames.length - 1)
    for (let k = 0; k < n; k++) {
      due += plan[k]!
      worst = Math.max(worst, frames[k + 1]! - frames[0]! - due)
    }
    const seconds = expected / 1000
    console.log(
      `${wpm} wpm: ${plan.length} tokens, engine ${expected} ms, wall ${wall.toFixed(0)} ms ` +
        `(${(wall - expected).toFixed(0)} ms; key to first frame ${(frames[0]! - t0).toFixed(0)} ms), ${frames.length} frames ` +
        `(${(frames.length / seconds).toFixed(1)}/s vs ${(plan.length / seconds).toFixed(1)} words/s), ` +
        `worst word lag ${worst.toFixed(1)} ms, exit ${code}, terminal ${restored ? "restored" : "NOT restored"}`,
    )
    if (frames.length < plan.length || worst > LAG_LIMIT_MS || Math.abs(wall - expected) > LAG_LIMIT_MS + 50 || code !== 0 || !restored) {
      failed = true
    }
  }
} finally {
  rmSync(dir, { recursive: true, force: true })
}
process.exit(failed ? 1 : 0)
