# clench for iOS and macOS

A SwiftUI app for iOS 18+ and macOS 15+ with a share extension and a
Continue widget: share a page, link or text to clench and choose **Read
now** or **Save for later**. All reading logic (extraction, tokens,
pivots, timing, sections, the document map, time estimates, sentence
steps) comes from the Rust engine through native bindings; the Swift code
moves bytes, owns the clock and draws frames.

## Layout

| Path | What it is |
|---|---|
| `project.yml` | XcodeGen spec. `Clench.xcodeproj` is generated from it and gitignored |
| `Config/Base.xcconfig` | Shared settings: bundle IDs, App Group, signing switch |
| `Config/Local.xcconfig.example` | Template for your own signing settings |
| `Clench/` | The app (one multiplatform target): library, reader, settings, Mac windows |
| `ClenchShare/` | Share extension (iOS and macOS), with `Preprocess.js` for Safari |
| `ClenchWidget/` | The Continue widget (WidgetKit, iOS and macOS) |
| `ClenchKit/` | Swift package: the engine bindings, the queue store and library models, and the shared SwiftUI theme |
| `../../crates/clench-ffi` | The UniFFI bindings the package wraps |

`ClenchKit` has four targets:

- `ClenchCore`: the binary target, `ClenchCore.xcframework` (static
  libraries for iOS, the iOS Simulator and macOS, all arm64).
- `ClenchFFI`: the Swift bindings UniFFI generates (`extract`, `Session`,
  `version`, and the `Document`, `Frame`, `Context` records).
- `ClenchKit`: what the app and the extensions share. `QueueStore` keeps
  `queue.json` and stored page content in the App Group container;
  `ReaderDriver` drives a `Session` from the host clock the same way the
  JavaScript hosts do (`packages/host/src/clock.ts`). `ItemSummary` stores
  the engine's outline, document map and position with each item, keeping
  times speed-independent so `readingMs` can give minutes at any speed;
  `LibraryShelves` filters the queue by time budget and search;
  `ReadingGesture` is the reader's touch model as a pure state machine;
  `Preferences` and the theme tokens live in the App Group's defaults.
- `ClenchUI`: SwiftUI pieces the app, the share extension and the widget
  share: theme colours, the bundled IBM Plex fonts (see
  `Sources/ClenchUI/Resources/Fonts/README.md` for versions and the OFL
  licence), the barcode and edge-map views of the engine's terrain, and
  the design's controls.

The xcframework and `ClenchFFI.swift` are build outputs and gitignored.

## Build

Requirements: Xcode 27, [XcodeGen](https://github.com/yonaskolb/XcodeGen)
(`brew install xcodegen`), Rust with the Apple targets:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin
```

From the repository root:

```sh
bun run apple:xcframework                     # engine -> ClenchKit/Artifacts, bindings -> ClenchKit/Sources/ClenchFFI
xcodegen generate --spec apps/apple/project.yml
open apps/apple/Clench.xcodeproj              # scheme "Clench", pick an iPhone simulator or My Mac
```

Rerun `bun run apple:xcframework` after changing Rust code. Extra
arguments go to cargo, for example
`sh scripts/build-apple.sh --no-default-features --features markdown`
to leave PDF support out. `--macos-only` builds just the macOS slice.

From the command line, unsigned:

```sh
cd apps/apple
xcodebuild build -project Clench.xcodeproj -scheme Clench -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO
xcodebuild build -project Clench.xcodeproj -scheme Clench -destination 'generic/platform=macOS' CODE_SIGNING_ALLOWED=NO
```

## Tests

```sh
bun run apple:test           # macOS slice + clench-cli + swift test in ClenchKit
# or, through the app's scheme (needs `cargo build -p clench-cli` for the macOS-only CLI comparison):
xcodebuild test -project apps/apple/Clench.xcodeproj -scheme Clench -destination 'platform=macOS' CODE_SIGNING_ALLOWED=NO
```

The Swift tests read `fixtures/` from the repository, check that the
engine's word count for a fixture equals what `clench-cli extract --json`
reports (macOS only), round-trip the queue store and exercise the reader
clock. The Rust side is tested by `cargo test -p clench-ffi`.

## Signing with your own team

Nothing personal is committed: the project's bundle IDs use the
placeholder prefix `com.example`, and without a team it builds ad hoc
("Sign to Run Locally") or unsigned. To run on a device, or to use the App
Group between the app and the extension on macOS:

```sh
cp apps/apple/Config/Local.xcconfig.example apps/apple/Config/Local.xcconfig
```

and set, in `Local.xcconfig` (gitignored):

```
DEVELOPMENT_TEAM = <your team ID>
BUNDLE_ID_PREFIX = <a reverse-DNS prefix you own, e.g. org.yourname>
```

With a team set, `Base.xcconfig` switches to automatic signing. Without
one, the macOS targets are ad-hoc signed with `*-macOS-AdHoc.entitlements`,
which leave the App Group out (on macOS a `group.` App Group needs a
provisioning profile): the app works on its own, but the share extension
cannot reach its queue. Everything else derives from the prefix:

| | Value |
|---|---|
| App | `$(BUNDLE_ID_PREFIX).clench` |
| Share extension | `$(BUNDLE_ID_PREFIX).clench.share` |
| Widget | `$(BUNDLE_ID_PREFIX).clench.widget` |
| App Group | `group.$(BUNDLE_ID_PREFIX).clench` |

Regenerate the project after creating `Local.xcconfig` only if Xcode does
not pick it up (it is included with `#include?`, so no regeneration is
normally needed).

## How it fits together

- **Share extension.** It reads Safari's preprocessing result (the page as
  rendered, or the selected text), else a URL, else text, and stores it in
  the queue without extracting, so large pages do not count against the
  extension's memory. **Add to queue** finishes there. **Read now** queues
  the item and opens `clench://read?id=<uuid>`.
- **The app** extracts stored content with the engine, or fetches the URL
  itself (the macOS app has the outgoing-network entitlement), and saves
  the reading position (token index, WPM, last read) back to the queue.
- **URL scheme.** `clench://read?id=<uuid>` opens an item and
  `clench://queue` the queue.

## Two-screen phones

The iOS 27.1 SDK has a public hinge API: SwiftUI's
`onHingeChange(isEnabled:_:)` with `DeviceHingeContext` and
`DeviceHinge.Status` (`closed`, `partiallyOpen`, `fullyOpen`), and UIKit's
`UIHingeInteraction` (`UIHinge.h`, `UIHingeInteraction.h`, both
`API_AVAILABLE(ios(27.1))`). The reader uses it behind
`if #available(iOS 27.1, *)` and otherwise lays out by window size only:

- closed (outer display): the phone reader, with a hint to open the phone;
- opened flat, an iPad, or any window at least 700 points wide and wider
  than tall: the page reader (the block beside the word) and the
  two-column library;
- propped half open in portrait: hands-free reading with a large pause
  button and a speed slider.

## Debug launch arguments

Debug builds accept `-clench-demo` (fill an empty queue with sample
items), `-clench-screen reader|settings|peek|finished|search|end`,
`-clench-reader dial|scrub|strip|playing` and `-clench-layout page|stand`,
for screenshots and manual checks (`Clench/Demo.swift`).

## Known limitations

- **Read now on iOS.** Apple documents `NSExtensionContext.open(_:)` for
  the Today and iMessage extension points only ("In iOS, the Today and
  iMessage app extension points support this method",
  [NSExtensionContext.open](https://developer.apple.com/documentation/foundation/nsextensioncontext/open(_:completionhandler:))).
  The share extension still tries; when it cannot open the app it says the
  item was queued. On macOS it opens the app through `NSWorkspace`.
- **Safari page content on macOS.** Apple's archived guide says the
  preprocessing file works "In Share extensions (on both platforms)", but
  it predates current macOS Safari and has not been checked on a Mac
  ([App Extension Programming Guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/ExtensionScenarios.html)).
  Without it the extension receives the URL and the app fetches it.
- **The queue format** is the app's own JSON for now; the shared format
  for every host is planned in
  [#6](https://github.com/sethgho/clench/issues/6).
- **arm64 only.** There is no x86_64 simulator slice, so Intel Macs cannot
  run the iOS Simulator build (`EXCLUDED_ARCHS` leaves x86_64 out), and the
  macOS build is Apple silicon only.
- The share extension's memory ceiling has not been measured on a device.
  It reads pages up to 3 MB in for its preview and leaves bigger ones to
  the app.
- **Clipboard suggestion.** The library checks only whether the clipboard
  holds a link (no permission needed) and offers the system Paste button,
  so it cannot show the link's title before it is pasted.
- **Mac global shortcut** ("Shortcut to read or save" in the Mac settings
  design) is not implemented.
