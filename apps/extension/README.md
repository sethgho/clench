# clench for Chrome

Reads the page you are looking at, one word at a time. It works on the page as
your browser has already rendered it, so JavaScript-rendered pages, pages you
are logged in to and sites that refuse non-browser requests work the same as
any other. Nothing is fetched and nothing leaves your machine: there is no
server, no remote code, no analytics and no AI. The text is the author's,
never rewritten.

## Install (unpacked)

Build the extension (see the repository README for the toolchain: Rust with the
wasm32 target, `wasm-bindgen-cli`, binaryen):

```sh
bun install
bun run ext:build        # writes apps/extension/dist
```

Then in Chrome 114 or newer:

1. Open `chrome://extensions` and turn on **Developer mode**.
2. Choose **Load unpacked** and select `apps/extension/dist`.
3. Pin the clench button if you like.

## Use

Click the toolbar button, or press **Alt+Shift+R**, on any web page. The
reader opens over the page. Space starts and pauses, `?` lists the keys, and
**Esc** (or `q`, or the close button) removes the reader and puts the page back
as it was, with your scroll position and focus restored. Pressing the button
again also closes it. Your reading speed is remembered between pages. Change
the shortcut at `chrome://extensions/shortcuts`.

When the page could only be partly extracted or looks paywalled, the ready
screen says so, as the terminal and web readers do.

## Permissions

| Permission | Why |
|---|---|
| `activeTab` | Lets the button or shortcut read the current tab, only at the moment you use it. There is no all-sites host access. |
| `scripting` | Injects the small function that captures the page and shows the reader. |
| `storage` | Remembers your words per minute (`chrome.storage.local`, on this device). |

Chrome does not allow extensions on `chrome://` pages, the Chrome Web Store or
other extensions' pages; on those the button shows a `!` badge.

## How it works

- `src/background.ts` handles the button and shortcut, and runs `toggleOverlay`
  (`src/inject.ts`) in the tab with `chrome.scripting.executeScript`.
- That function copies `document.documentElement.outerHTML`, then adds a
  full-viewport iframe inside a closed shadow root, so page styles cannot
  reach the reader. The iframe is the extension page `reader.html`.
- `reader.html` receives the HTML by `postMessage`, calls the engine's
  `extract(bytes, "text/html; charset=utf-8", pageUrl)` and mounts
  `@clench/reader-web`. All extraction logic is in Rust; the TypeScript never
  touches the text.
- The WebAssembly engine ships in the package and is loaded with
  `loadEngine(chrome.runtime.getURL(...))`. The extension page's content
  security policy is `script-src 'self' 'wasm-unsafe-eval'; object-src 'self'; base-uri 'none'`:
  `'wasm-unsafe-eval'` is the one relaxation Chrome needs to compile WebAssembly.

## Development

```sh
bun run ext:build        # production build into apps/extension/dist
bun run ext:e2e          # builds apps/extension/dist-e2e and drives it in Chromium
```

The browser test uses Playwright's Chromium (`./node_modules/.bin/playwright-core
install chromium`) and Node 22.18 or newer. A test cannot click the toolbar
button, which is what grants `activeTab`, so the `dist-e2e` build adds a host
permission for `http://127.0.0.1/*` and a hook that does what the button does.
That build is never shipped; a unit test checks the real manifest.

## Known limits

- A page whose Content-Security-Policy forbids framing other origins
  (`frame-src` or `default-src`) can block the reader frame.
- Only the whole page is read; selection-only reading and a context-menu item
  are not built yet.
- The page is read once, when you open the reader; later changes are not
  followed.
