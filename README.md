# meatread

Speed-read any link yourself, one word at a time.

Don't have an AI summarize everything: read the post yourself, meat bag.
`meatread <url>` fetches the article and shows it one word at a time at a
fixed point, each word aligned on its pivot letter (RSVP, rapid serial
visual presentation), with timing that slows at punctuation and paragraph
breaks. There is no AI anywhere in it, and the engine never alters the
author's words: it splits, times, skips and annotates, but never rewrites,
normalizes or summarizes.

## Quick start

Requirements: [Bun](https://bun.sh) (any recent version, used only to
install; the scripts run a pinned Bun from `node_modules`), Rust with the
`wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.130 and `wasm-opt`
([binaryen](https://github.com/WebAssembly/binaryen)).

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
brew install binaryen                       # or your package manager's binaryen

bun install
bun run dev https://example.com/some-post   # builds the engine on first run
bun run dev saved-page.html --wpm 450
cat notes.md | bun run dev -
```

To build a standalone executable for this machine, with the engine
embedded:

```sh
bun run build          # writes dist/meatread
dist/meatread https://example.com/some-post
```

Use `bun run <script>`, not plain `bun test` or `bun src/...`: a global Bun
older than 1.3.14 cannot run OpenTUI, and the scripts call the pinned one.

| Script | What it does |
|---|---|
| `bun run build:wasm` | Build the engine into `crates/meatread-wasm/pkg` (rerun after Rust changes) |
| `bun run dev <source>` | Run the reader from source |
| `bun run test` | Engine build, `cargo test`, Bun tests and `tsc --noEmit` |
| `bun run build` | Standalone binary in `dist/meatread` |
| `bun run measure` | Play a few hundred words in a pseudo-terminal and compare wall time and frames with the engine's schedule |

## Reading

The reader opens on a ready screen with the title, byline, word count, an
estimated reading time and a warning when the page could only be partly
extracted. Space starts. While paused, the surrounding paragraph is shown
with the current word highlighted.

| Key | Action |
|---|---|
| space | play / pause |
| ← `h`, → `l` | previous / next sentence |
| `[` `]`, shift+← shift+→ | previous / next paragraph |
| ↑ `k`, ↓ `j` | 25 WPM faster / slower |
| `,` `.` | back / forward one word |
| `0`–`9` | jump to that tenth of the document |
| `r` | restart |
| `?` | help |
| `q`, ctrl-c | quit |

`meatread <url | file | -> [--wpm <n>]`; `--help` and `--version` as
usual. Code blocks and tables are not read word by word; they show as
`[code]` and `[table]` markers, in a different style from the author's
text. Colours come from the terminal's own palette and the background is
never painted; `NO_COLOR` turns colour off.

## Architecture

All reading logic is in Rust so that every host behaves the same:

- **`crates/meatread-core`**: the portable, sans-I/O engine (`no_std` +
  `alloc`, integer-only timing). A `Document` of typed blocks becomes a
  `Script` of tokens with pivots and sentence and paragraph membership; a
  host-clocked `Player` handles pacing, ease-in, navigation and estimates.
  Builds for native, `wasm32-unknown-unknown` and `thumbv7em-none-eabihf`.
- **`crates/meatread-extract`**: fetched bytes plus Content-Type to a
  `Document` for HTML (a Readability port), Markdown, plain text and PDF,
  with charset detection. No I/O. Features `markdown` and `pdf`.
- **`crates/meatread-wasm`**: `wasm-bindgen` bindings, `extract()` and a
  `Session` that mirrors the player's controls and returns plain `Frame` and
  `Context` objects; the generated `.d.ts` is the API for any JavaScript host.
  About 2.0 MB optimized (0.9 MB gzipped), 1.3 MB without the `pdf` feature
  (`scripts/build-wasm.sh --no-default-features --features markdown`).
- **Hosts**: `apps/tui`, the terminal reader (Bun, OpenTUI, SolidJS), is a
  thin view: it fetches bytes, owns the clock, forwards keys and draws
  frames. `crates/meatread-cli` is a native debugging tool (below).

The core API, tokenization rules and pacing formula are documented in
rustdoc (`cargo doc --workspace --open`).

## Known limitations

- Pages whose text is rendered by JavaScript, or that sit behind a paywall,
  do not contain the article in their HTML; they come back as a fragment
  and the ready screen says so.
- Some sites answer 401 or 403 to anything that is not a full browser.
  meatread does not try to get around that; save the page from a browser and
  read the file instead.
- PDF is best-effort: the text is whatever the file's text layer holds, so
  math and multi-column layouts can be garbled and end-of-line hyphens are
  kept as written. A PDF that crashes the extractor is reported as
  unreadable.
- Pages nested more than 2,500 elements deep are refused rather than read;
  real pages stay a few hundred levels deep.
- Pages that declare no charset and are not valid UTF-8 are read as
  windows-1252, the web's default; a page in another legacy encoding must
  declare it.
- Sentence detection is heuristic (abbreviations and initialisms can fool
  it), which affects sentence jumps and pauses.
- Chinese, Japanese and Thai text is not segmented into words: each run
  between spaces is cut into fixed-width pieces.

## Debug command line

```sh
cargo run -p meatread-cli -- extract https://example.com/post            # block listing
cargo run -p meatread-cli -- extract https://example.com/post --text     # reading text
cargo run -p meatread-cli -- extract https://example.com/post --json     # Document as JSON
cargo run -p meatread-cli -- script  https://example.com/post --wpm 400  # token schedule
cargo run -p meatread-cli -- extract saved-page.html --url https://example.com/post
cat post.md | cargo run -p meatread-cli -- extract - --content-type text/markdown
```

It fetches the same way as the reader (generic browser User-Agent, redirects
followed, the response's Content-Type passed to the extractor). `fixtures/`
holds saved third-party pages and a sample PDF with ground truth for local
regression tests; see `fixtures/README.md`.

## Development

```sh
bun run test
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check

# Portability checks
cargo build -p meatread-core --target thumbv7em-none-eabihf --features serde
cargo build -p meatread-core --target wasm32-unknown-unknown --features serde
cargo build -p meatread-extract --target wasm32-unknown-unknown --release --no-default-features
```
