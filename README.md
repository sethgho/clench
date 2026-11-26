# meatread

Speed-read any link yourself, one word at a time.

meatread is an RSVP (rapid serial visual presentation) reader: words appear
one by one at a fixed point, each aligned on its pivot letter, with timing
that slows at punctuation and paragraph breaks. Rather than having an AI
summarize a post, you read the post. The engine never alters the author's
words: it splits, times, skips and annotates, but never rewrites, normalizes
or summarizes.

## Status

Built so far:

- **`crates/meatread-core`**: the portable, sans-I/O engine (`no_std` +
  `alloc`, integer-only timing). It takes a `Document` of typed blocks,
  builds a `Script` of tokens (words, long-word pieces, and markers for
  skipped code and tables) with pivot positions and sentence and paragraph
  membership, and plays it with a host-driven `Player` that handles pacing,
  ease-in, navigation and time estimates. Optional `serde` feature for the
  data types. Builds for native, `wasm32-unknown-unknown` and
  `thumbv7em-none-eabihf`.
- **`crates/meatread-extract`**: turns fetched bytes into a core `Document`.
  `extract(bytes, content_type, url)` sniffs PDF, HTML, Markdown and plain
  text, decodes the character encoding (header, byte order mark, `<meta>`),
  and for HTML locates the main content with a Readability port
  (`dom_smoothie`), walks it into typed blocks and removes leftover page
  furniture. It does no I/O and builds for `wasm32-unknown-unknown`. Cargo
  features `markdown` and `pdf` (both on by default) gate those formats.
  Extraction only keeps or drops text and collapses HTML white space; it never
  rewrites words. Content that came from a fallback (JSON-LD, the meta
  description), is implausibly short or looks paywalled is flagged `partial`.
- **`crates/meatread-cli`**: a native `meatread` binary that fetches a URL (or
  reads a file or standard input) and runs it through the extractor.
- **`fixtures/`**: saved third-party pages and a sample PDF with ground truth,
  for local regression tests. See `fixtures/README.md`.

Not built yet: WebAssembly bindings and the terminal UI.

Known limitations of extraction: pages whose text is rendered by JavaScript or
sits behind a paywall do not contain the article in their HTML, so they come
back as a fragment flagged `partial`; PDF text is whatever the file's text
layer holds (math and multi-column layouts can be garbled, and an end-of-line
hyphen is kept as written); legacy `<br>` layouts are split into paragraphs
by the Readability port, which can occasionally leave two paragraphs joined.

The core API, tokenization rules, pacing formula and limitations are
documented in the crates' rustdoc (`cargo doc --workspace --open`).

## Command line

```sh
cargo run -p meatread-cli -- extract https://example.com/post            # block listing
cargo run -p meatread-cli -- extract https://example.com/post --text     # reading text
cargo run -p meatread-cli -- extract https://example.com/post --json     # Document as JSON
cargo run -p meatread-cli -- script  https://example.com/post --wpm 400  # token schedule
cargo run -p meatread-cli -- extract saved-page.html --url https://example.com/post
cat post.md | cargo run -p meatread-cli -- extract - --content-type text/markdown
```

The source is an `http(s)` URL, a file path or `-` for standard input. Fetches
send a generic browser User-Agent, follow redirects and pass the response's
Content-Type to the extractor. A non-2xx answer is reported as an error (403s
are usually bot blocks; meatread does not try to get around them). For files,
the Content-Type is guessed from the extension unless `--content-type` is
given, and `--url` supplies the page's address for resolving links.

## Development

```sh
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# Portability checks
rustup target add thumbv7em-none-eabihf wasm32-unknown-unknown
cargo build -p meatread-core --target thumbv7em-none-eabihf --features serde
cargo build -p meatread-core --target wasm32-unknown-unknown --features serde
cargo build -p meatread-extract --target wasm32-unknown-unknown --release
cargo build -p meatread-extract --target wasm32-unknown-unknown --release --no-default-features
```
