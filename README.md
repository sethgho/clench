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

Not built yet: HTML extraction, WebAssembly bindings, and the terminal UI.

The API, tokenization rules, pacing formula and known limitations are
documented in the crate's rustdoc (`cargo doc -p meatread-core --open`).

## Development

```sh
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# Portability checks
rustup target add thumbv7em-none-eabihf wasm32-unknown-unknown
cargo build -p meatread-core --target thumbv7em-none-eabihf --features serde
cargo build -p meatread-core --target wasm32-unknown-unknown --features serde
```
