<p align="center"><img src="assets/logo-768.png" alt="clench logo: a red brain gritting its teeth over an open book" width="384"></p>

# clench

[![CI](https://github.com/sethgho/clench/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sethgho/clench/actions/workflows/ci.yml)

Speed-read any link yourself, one word at a time.

Don't have an AI summarize everything: read the post yourself, meat bag.
`clench <url>` fetches the article and shows it one word at a time at a
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
cat notes.md | bun run dev - --type md
```

To build a standalone executable for this machine, with the engine
embedded:

```sh
bun run build          # writes dist/clench
dist/clench https://example.com/some-post
```

Use `bun run <script>`, not plain `bun test` or `bun src/...`: a global Bun
older than 1.3.14 cannot run OpenTUI, and the scripts call the pinned one.

| Script | What it does |
|---|---|
| `bun run build:wasm` | Build the engine into `crates/clench-wasm/pkg` (rerun after Rust changes) |
| `bun run dev <source>` | Run the reader from source |
| `bun run test` | Engine build, `cargo test`, Bun tests and `tsc --noEmit` |
| `bun run build` | Standalone binary in `dist/clench` |
| `bun run build:web` | Engine plus the static web reader in `apps/web/dist` |
| `bun run dev:web` | Web reader dev server |
| `bun run ext:build` | Engine plus the Chrome extension in `apps/extension/dist` |
| `bun run apple:xcframework` | Build the engine for iOS and macOS into `ClenchCore.xcframework` plus Swift bindings |
| `bun run apple:test` | Build the macOS slice and run the Swift package tests |
| `bun run ext:e2e` | Build the extension and test it in Chromium with Playwright (runs in CI) |
| `bun run e2e` | Check `apps/web/dist` in the installed Google Chrome (`--url <site>` checks a deployed copy, `--shots <dir>` saves screenshots); needs network, not run in CI |
| `bun run measure` | Play a few hundred words in a pseudo-terminal and compare wall time and frames with the engine's schedule |

## Chrome extension

`apps/extension` is a Manifest V3 extension that opens the reader over the
page you are on, using the page as your browser rendered it. That covers
JavaScript-rendered pages, pages you are logged in to and sites that block
non-browser fetches. It is fully local, asks only for `activeTab`, `scripting`
and `storage`, and ships the engine inside the package.

```sh
bun run ext:build      # then Load unpacked: apps/extension/dist
```

Click the toolbar button or press Alt+Shift+R. See
[apps/extension/README.md](apps/extension/README.md).

## iOS and macOS

`apps/apple` is a SwiftUI app for iOS 18+ and macOS 15+ with a share
extension: share a page, link or text to clench and choose **Read now** or
**Add to queue**. The engine is linked natively through UniFFI bindings
(`crates/clench-ffi`), so the app runs the same Rust code as every other
host. The interface is a placeholder for now.

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin
bun run apple:xcframework                        # engine + Swift bindings
xcodegen generate --spec apps/apple/project.yml  # then open apps/apple/Clench.xcodeproj
bun run apple:test                               # Swift tests (macOS)
```

It builds unsigned without any setup; to sign with your own team and
bundle ID prefix, copy `apps/apple/Config/Local.xcconfig.example` to
`Local.xcconfig`. See [apps/apple/README.md](apps/apple/README.md).

## Reading

The reader opens on a ready screen with the title, byline, word count, an
estimated reading time, a "not read" line when the script leaves anything
out (for example `not read: 15 code blocks (marked), 12 footnotes`; marked
means a `[code]` or `[table]` marker is shown, unmarked means silently left
out) and a warning when the page could only be partly extracted or is only a
paper's abstract. Space starts. While paused, the surrounding paragraph is shown
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
| ctrl-z | suspend; `fg` resumes, paused on the same word |
| `q`, ctrl-c | quit |

`clench <url | file | -> [--wpm <n>] [--type <html|md|text|pdf>]`;
`--help` and `--version` as usual. The type normally comes from the server's
Content-Type or the file's extension (`.html`, `.md`, `.txt`, `.pdf`);
standard input has neither, so it is read as plain text unless you say
otherwise. `--type` overrides the detection for any input. Headings are drawn
bold, and the guide lines thicken on the first word of a new block. After a
jump, the finished screen shows the time spent reading but no overall rate.
Code blocks and tables are not read word by word; they show as
`[code]` and `[table]` markers, in a different style from the author's
text. Colours come from the terminal's own palette and the background is
never painted; `NO_COLOR` turns colour off.

## Web

**<https://clench.sethgholson.com/>** runs the same engine in your
browser. It is a static page: there is no server behind it, nothing you read
is sent anywhere, and it works offline after the first visit.

- **Paste** text, HTML or Markdown (the type is detected, or choose it), or
  **drop or pick a file** (HTML, Markdown, text, PDF). This always works.
- **A web address** works only when the site lets other pages read it
  ([CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS)),
  for example raw files on `raw.githubusercontent.com`. Most article pages do
  not, and clench has no server or proxy to fetch them, so it says so and
  asks you to paste or drop the page instead. A Chrome extension that reads
  the open tab is planned ([#2](https://github.com/sethgho/clench/issues/2)).
- **Links:** `#text=<text>` opens text carried in the URL fragment, which the
  browser never sends to a server; `?url=<address>` tries a fetch.
- **Share:** installed as an app in Chrome (desktop or Android), it can
  appear as a share target. Firefox and Safari, including every browser on
  iOS, do not support web share targets.

Keys are the terminal reader's (no ctrl-z; `q` closes the reader). On touch
screens, tap the word to play or pause and swipe for the next or previous
sentence. Speed and theme are remembered in this browser.

## Architecture

All reading logic is in Rust so that every host behaves the same:

- **`crates/clench-core`**: the portable, sans-I/O engine (`no_std` +
  `alloc`, integer-only timing). A `Document` of typed blocks becomes a
  `Script` of tokens with pivots and sentence and paragraph membership; a
  host-clocked `Player` handles pacing, ease-in, navigation and estimates.
  Builds for native, `wasm32-unknown-unknown` and `thumbv7em-none-eabihf`.
- **`crates/clench-extract`**: fetched bytes plus Content-Type to a
  `Document` for HTML (a Readability port), Markdown, plain text and PDF,
  with charset detection. No I/O. Features `markdown` and `pdf`.
- **`crates/clench-ffi`**: UniFFI bindings for native hosts with the same
  surface as the wasm crate (`extract()`, a host-clocked `Session`,
  `version()`); `scripts/build-apple.sh` turns them into
  `ClenchCore.xcframework` and Swift bindings for `apps/apple`.
- **`crates/clench-wasm`**: `wasm-bindgen` bindings, `extract()` and a
  `Session` that mirrors the player's controls and returns plain `Frame` and
  `Context` objects; the generated `.d.ts` is the API for any JavaScript host.
  About 2.0 MB optimized (0.9 MB gzipped), 1.3 MB without the `pdf` feature
  (`scripts/build-wasm.sh --no-default-features --features markdown`).
- **Hosts**: `apps/tui`, the terminal reader (Bun, OpenTUI, SolidJS), is a
  thin view: it fetches bytes, owns the clock, forwards keys and draws
  frames. `apps/web` is the web reader (Vite, SolidJS); the reader view it
  mounts is `packages/reader-web`, a self-contained `mount(element, doc)`
  that other browser hosts can reuse. `packages/host` holds what the hosts
  share: the clock driver, key map, input types, bounded fetching and the
  wording of the ready and finished screens. `crates/clench-cli` is a
  native debugging tool (below).

The core API, tokenization rules and pacing formula are documented in
rustdoc (`cargo doc --workspace --open`).

## Known limitations

- Pages whose text is rendered by JavaScript, or that sit behind a paywall,
  do not contain the article in their HTML; they come back as a fragment
  and the ready screen says so.
- Some sites answer 401 or 403 to anything that is not a full browser.
  clench does not try to get around that; save the page from a browser and
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
- Right-to-left scripts (Arabic, Hebrew) are not laid out for reading: the
  text before and after the pivot letter is drawn in logical order, so words
  and their pivot alignment can look wrong.
- Sentence detection is heuristic (abbreviations and initialisms can fool
  it), which affects sentence jumps and pauses.
- Chinese, Japanese and Thai text is not segmented into words: each run
  between spaces is cut into fixed-width pieces.

## Debug command line

```sh
cargo run -p clench-cli -- extract https://example.com/post            # block listing
cargo run -p clench-cli -- extract https://example.com/post --text     # reading text
cargo run -p clench-cli -- extract https://example.com/post --json     # Document as JSON
cargo run -p clench-cli -- script  https://example.com/post --wpm 400  # token schedule
cargo run -p clench-cli -- extract saved-page.html --url https://example.com/post
cat post.md | cargo run -p clench-cli -- extract - --content-type text/markdown
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
cargo build -p clench-core --target thumbv7em-none-eabihf --features serde
cargo build -p clench-core --target wasm32-unknown-unknown --features serde
cargo build -p clench-extract --target wasm32-unknown-unknown --release --no-default-features
```

## License

MIT — see [LICENSE](LICENSE). Page snapshots under `fixtures/` are third-party content and are excluded.
