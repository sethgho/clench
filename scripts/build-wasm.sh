#!/bin/sh
# Builds the clench WebAssembly package into crates/clench-wasm/pkg:
# cargo (release, wasm32) -> wasm-bindgen (ES module, `web` target) -> wasm-opt.
#
# Extra arguments go to cargo, e.g. to leave PDF support out:
#   scripts/build-wasm.sh --no-default-features --features markdown
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/crates/clench-wasm/pkg"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "build-wasm: '$1' not found. $2" >&2
        exit 1
    }
}
need cargo "Install Rust from https://rustup.rs."
need wasm-bindgen "Install it with: cargo install wasm-bindgen-cli --version 0.2.130"
need wasm-opt "Install binaryen (e.g. brew install binaryen)."

pinned=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' "$root/crates/clench-wasm/Cargo.toml")
installed=$(wasm-bindgen --version | awk '{print $2}')
if [ "$pinned" != "$installed" ]; then
    echo "build-wasm: wasm-bindgen CLI is $installed but the crate pins $pinned." >&2
    echo "Install the matching CLI: cargo install wasm-bindgen-cli --version $pinned" >&2
    exit 1
fi

cd "$root"
# Cargo's own answer, so CARGO_TARGET_DIR, build.target-dir and the default
# agree with where the build below writes.
target=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
if [ -z "$target" ]; then
    echo "build-wasm: could not determine cargo's target directory." >&2
    exit 1
fi
wasm="$target/wasm32-unknown-unknown/release/clench_wasm.wasm"

cargo build -p clench-wasm --target wasm32-unknown-unknown --release "$@"
if [ ! -f "$wasm" ]; then
    echo "build-wasm: expected $wasm after the build, but it does not exist." >&2
    exit 1
fi
rm -rf "$out"
wasm-bindgen --target web --out-dir "$out" --out-name clench "$wasm"
wasm-opt -Oz "$out/clench_bg.wasm" -o "$out/clench_bg.wasm"
echo "build-wasm: $(wc -c <"$out/clench_bg.wasm" | tr -d ' ') bytes -> $out"
