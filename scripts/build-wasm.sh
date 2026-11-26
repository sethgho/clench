#!/bin/sh
# Builds the meatread WebAssembly package into crates/meatread-wasm/pkg:
# cargo (release, wasm32) -> wasm-bindgen (ES module, `web` target) -> wasm-opt.
#
# Extra arguments go to cargo, e.g. to leave PDF support out:
#   scripts/build-wasm.sh --no-default-features --features markdown
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/crates/meatread-wasm/pkg"
wasm="$root/target/wasm32-unknown-unknown/release/meatread_wasm.wasm"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "build-wasm: '$1' not found. $2" >&2
        exit 1
    }
}
need cargo "Install Rust from https://rustup.rs."
need wasm-bindgen "Install it with: cargo install wasm-bindgen-cli --version 0.2.130"
need wasm-opt "Install binaryen (e.g. brew install binaryen)."

pinned=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' "$root/crates/meatread-wasm/Cargo.toml")
installed=$(wasm-bindgen --version | awk '{print $2}')
if [ "$pinned" != "$installed" ]; then
    echo "build-wasm: wasm-bindgen CLI is $installed but the crate pins $pinned." >&2
    echo "Install the matching CLI: cargo install wasm-bindgen-cli --version $pinned" >&2
    exit 1
fi

cd "$root"
cargo build -p meatread-wasm --target wasm32-unknown-unknown --release "$@"
rm -rf "$out"
wasm-bindgen --target web --out-dir "$out" --out-name meatread "$wasm"
wasm-opt -Oz "$out/meatread_bg.wasm" -o "$out/meatread_bg.wasm"
echo "build-wasm: $(wc -c <"$out/meatread_bg.wasm" | tr -d ' ') bytes -> $out"
