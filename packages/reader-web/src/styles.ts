/**
 * The reader's stylesheet. Every rule is scoped under `.clench-reader`, so it
 * can be injected into any page or shadow root without touching the host.
 * Colours are custom properties; `data-theme` on the root forces light or
 * dark, otherwise the system preference applies.
 */
export const READER_CSS = `
.clench-reader {
  --cr-bg: #FEF6E2;
  --cr-surface: #F6EBD0;
  --cr-text: #1B1411;
  --cr-muted: #6A5B4E;
  --cr-guide: #C9B999;
  --cr-pivot: #D02519;
  --cr-accent: #D02519;
  --cr-on-accent: #FEF6E2;
  --cr-mark-bg: #F7C9C3;
  --cr-marker: #5E5246;
  --cr-marker-bg: #EADFC6;
  --cr-warn: #8A2A00;
  --cr-focus: #1B1411;
  --cr-word-size: clamp(2rem, 9vw, 4.25rem);
  box-sizing: border-box;
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--cr-bg);
  color: var(--cr-text);
  font: 400 1rem/1.5 system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  -webkit-tap-highlight-color: transparent;
  outline: none;
}
.clench-reader[data-theme="dark"] {
  --cr-bg: #0E0B0A;
  --cr-surface: #1C1714;
  --cr-text: #FEF6E2;
  --cr-muted: #B3A58F;
  --cr-guide: #4A4038;
  --cr-pivot: #EF4A3A;
  --cr-accent: #EF4A3A;
  --cr-on-accent: #0E0B0A;
  --cr-mark-bg: #6B1A12;
  --cr-marker: #C8BBA5;
  --cr-marker-bg: #2A231E;
  --cr-warn: #FFB38A;
  --cr-focus: #FEF6E2;
}
@media (prefers-color-scheme: dark) {
  .clench-reader:not([data-theme="light"]) {
    --cr-bg: #0E0B0A;
    --cr-surface: #1C1714;
    --cr-text: #FEF6E2;
    --cr-muted: #B3A58F;
    --cr-guide: #4A4038;
    --cr-pivot: #EF4A3A;
    --cr-accent: #EF4A3A;
    --cr-on-accent: #0E0B0A;
    --cr-mark-bg: #6B1A12;
    --cr-marker: #C8BBA5;
    --cr-marker-bg: #2A231E;
    --cr-warn: #FFB38A;
    --cr-focus: #FEF6E2;
  }
}
.clench-reader *, .clench-reader *::before, .clench-reader *::after { box-sizing: border-box; }
.clench-reader .cr-sr {
  position: absolute; width: 1px; height: 1px; margin: -1px; padding: 0;
  overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0;
}
.clench-reader button {
  font: inherit;
  color: var(--cr-text);
  background: var(--cr-surface);
  border: 1px solid var(--cr-guide);
  border-radius: 999px;
  min-width: 44px;
  min-height: 44px;
  padding: 0 0.9rem;
  cursor: pointer;
  touch-action: manipulation;
}
.clench-reader button:hover { border-color: var(--cr-muted); }
.clench-reader button.cr-primary {
  background: var(--cr-accent);
  color: var(--cr-on-accent);
  border-color: var(--cr-accent);
  font-weight: 650;
}
.clench-reader :focus-visible { outline: 3px solid var(--cr-focus); outline-offset: 2px; }
.clench-reader:focus-visible { outline-offset: -3px; }

.clench-reader .cr-top {
  display: flex; align-items: center; gap: 0.5rem;
  padding: 0.5rem 0.75rem; min-height: 3.25rem;
}
.clench-reader .cr-top-title {
  flex: 1; min-width: 0; color: var(--cr-muted); font-size: 0.9rem;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: center;
}
.clench-reader .cr-main {
  flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden;
}

/* Ready, finished and help screens */
.clench-reader .cr-card {
  flex: 1; min-height: 0; overflow-y: auto;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 0.75rem; padding: 1rem; text-align: center;
}
.clench-reader .cr-card > * { max-width: 40rem; margin: 0; }
.clench-reader .cr-title {
  font-family: ui-serif, Georgia, "Times New Roman", serif;
  font-size: clamp(1.5rem, 5vw, 2.25rem); line-height: 1.2; font-weight: 700;
  overflow-wrap: anywhere;
}
.clench-reader .cr-muted { color: var(--cr-muted); }
.clench-reader .cr-warn {
  color: var(--cr-warn); border-left: 4px solid var(--cr-warn);
  padding: 0.25rem 0.75rem; text-align: left;
}
.clench-reader .cr-row { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 0.5rem; }
.clench-reader .cr-keys { border-collapse: collapse; text-align: left; }
.clench-reader .cr-keys th, .clench-reader .cr-keys td { padding: 0.2rem 0.75rem; vertical-align: top; }
.clench-reader .cr-keys th { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-weight: 600; white-space: nowrap; }
.clench-reader .cr-hint { font-size: 0.85rem; color: var(--cr-muted); }

/* Reading view */
.clench-reader .cr-stage {
  flex: 0 0 auto; padding: clamp(1rem, 8vh, 4rem) 0 1rem;
  touch-action: pan-y; user-select: none; -webkit-user-select: none; cursor: pointer;
}
.clench-reader .cr-guide {
  display: grid; grid-template-columns: minmax(0, 1fr) 2px minmax(0, 1fr);
  height: 0.9rem; align-items: stretch; margin: 0 auto; width: min(100%, 36rem); padding: 0 1rem;
}
.clench-reader .cr-guide > span:nth-child(1), .clench-reader .cr-guide > span:nth-child(3) { align-self: end; height: 1px; background: var(--cr-guide); }
.clench-reader .cr-guide.cr-bottom > span:nth-child(1), .clench-reader .cr-guide.cr-bottom > span:nth-child(3) { align-self: start; }
.clench-reader .cr-guide > span:nth-child(2) { background: var(--cr-guide); }
.clench-reader .cr-guide.cr-block > span { background: var(--cr-muted); }
.clench-reader .cr-guide.cr-block > span:nth-child(1), .clench-reader .cr-guide.cr-block > span:nth-child(3) { height: 3px; }
.clench-reader .cr-word {
  display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: baseline; padding: 0.35em 0.25rem;
  font-size: var(--cr-word-size); line-height: 1.15; font-weight: 500;
  font-variant-ligatures: none; font-kerning: none; white-space: pre;
}
.clench-reader .cr-before { display: flex; justify-content: flex-end; overflow: hidden; }
.clench-reader .cr-after { display: flex; justify-content: flex-start; overflow: hidden; }
.clench-reader .cr-pivot { color: var(--cr-pivot); }
.clench-reader .cr-word.cr-heading { font-weight: 800; }
.clench-reader .cr-word.cr-marker { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-style: italic; font-size: calc(var(--cr-word-size) * 0.7); color: var(--cr-marker); }
.clench-reader .cr-word.cr-marker .cr-pivot { color: var(--cr-marker); }
.clench-reader .cr-word.cr-marker > span { background: var(--cr-marker-bg); }
.clench-reader .cr-context {
  flex: 1; min-height: 0; overflow-y: auto; margin: 0 auto; width: min(100%, 40rem);
  padding: 0.5rem 1rem 1rem; font-family: ui-serif, Georgia, "Times New Roman", serif;
  font-size: 1.125rem; line-height: 1.6; overflow-wrap: anywhere;
}
.clench-reader .cr-context p { margin: 0; white-space: pre-wrap; }
.clench-reader .cr-context mark { background: var(--cr-mark-bg); color: inherit; border-radius: 3px; padding: 0 0.1em; }
.clench-reader .cr-context.cr-code p { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.9rem; color: var(--cr-marker); }
.clench-reader .cr-context.cr-code mark { background: none; }
.clench-reader .cr-spacer { flex: 1; }
.clench-reader .cr-controls {
  display: flex; flex-wrap: wrap; justify-content: center; gap: 0.5rem; padding: 0.5rem 0.75rem;
}
.clench-reader .cr-controls .cr-play { min-width: 5.5rem; }
.clench-reader .cr-wpm { display: inline-flex; align-items: center; gap: 0.25rem; }
.clench-reader .cr-wpm output { min-width: 4.5rem; text-align: center; font-variant-numeric: tabular-nums; }
.clench-reader .cr-footer { padding: 0.25rem 0.75rem 0.75rem; }
.clench-reader .cr-bar { height: 4px; background: var(--cr-guide); border-radius: 2px; overflow: hidden; }
.clench-reader .cr-bar > div { height: 100%; background: var(--cr-accent); }
.clench-reader .cr-stats { display: flex; justify-content: center; gap: 1rem; font-size: 0.85rem; color: var(--cr-muted); font-variant-numeric: tabular-nums; padding-top: 0.25rem; }
@media (max-height: 520px) {
  .clench-reader .cr-stage { padding-top: 0.5rem; }
}
@media (prefers-reduced-motion: no-preference) {
  .clench-reader button { transition: border-color 120ms, background-color 120ms; }
}
`
