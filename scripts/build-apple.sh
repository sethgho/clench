#!/bin/sh
# Builds the clench engine for Apple platforms and wires it into ClenchKit:
#   cargo (profile "apple", static libraries for iOS, iOS Simulator, macOS)
#   -> uniffi-bindgen (Swift bindings from the macOS library)
#   -> xcodebuild -create-xcframework
#
# Outputs (gitignored):
#   apps/apple/ClenchKit/Artifacts/ClenchCore.xcframework
#   apps/apple/ClenchKit/Sources/ClenchFFI/ClenchFFI.swift
#
# Options:
#   --macos-only   build only the macOS slice (faster; enough for `swift test`)
# Other arguments go to cargo, e.g. to leave PDF support out:
#   scripts/build-apple.sh --no-default-features --features markdown
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
kit="$root/apps/apple/ClenchKit"
xcframework="$kit/Artifacts/ClenchCore.xcframework"
swift_out="$kit/Sources/ClenchFFI"

targets="aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin"
if [ "${1:-}" = "--macos-only" ]; then
    targets="aarch64-apple-darwin"
    shift
fi

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "build-apple: '$1' not found. $2" >&2
        exit 1
    }
}
need cargo "Install Rust from https://rustup.rs."
need rustup "Install Rust from https://rustup.rs."
need xcodebuild "Install Xcode and run xcode-select."

installed=$(rustup target list --installed)
for t in $targets; do
    if ! printf '%s\n' "$installed" | grep -qx "$t"; then
        echo "build-apple: Rust target $t is missing. Install it with: rustup target add $t" >&2
        exit 1
    fi
done

cd "$root"
target_dir=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
if [ -z "$target_dir" ]; then
    echo "build-apple: could not determine cargo's target directory." >&2
    exit 1
fi

# Keep the libraries' minimum OS in line with the app's deployment targets
# (apps/apple/project.yml), so the linker does not warn about newer objects.
export IPHONEOS_DEPLOYMENT_TARGET=18.0
export MACOSX_DEPLOYMENT_TARGET=15.0

for t in $targets; do
    echo "build-apple: building $t"
    cargo build -p clench-ffi --lib --profile apple --target "$t" "$@"
done

echo "build-apple: generating Swift bindings"
cargo build -q -p clench-ffi --features bindgen --bin uniffi-bindgen
gen=$(mktemp -d)
trap 'rm -rf "$gen"' EXIT
"$target_dir/debug/uniffi-bindgen" generate \
    --library "$target_dir/aarch64-apple-darwin/apple/libclench_ffi.a" \
    --language swift \
    --out-dir "$gen"

# Headers sit in a subdirectory named after the module so that several
# xcframeworks' module maps never collide in Xcode's shared include folder.
headers="$gen/headers/ClenchCoreFFI"
mkdir -p "$headers"
cp "$gen/ClenchCoreFFI.h" "$headers/"
cp "$gen/ClenchCoreFFI.modulemap" "$headers/module.modulemap"

set --
for t in $targets; do
    set -- "$@" -library "$target_dir/$t/apple/libclench_ffi.a" -headers "$gen/headers"
done
rm -rf "$xcframework"
mkdir -p "$(dirname "$xcframework")" "$swift_out"
xcodebuild -create-xcframework "$@" -output "$xcframework" >/dev/null
cp "$gen/ClenchFFI.swift" "$swift_out/ClenchFFI.swift"

echo "build-apple: $xcframework"
echo "build-apple: $swift_out/ClenchFFI.swift"
