/** @jsxImportSource solid-js */
import { finishedSummary, fmt, KEY_HELP, notRead, siteLabel, warnings, WPM_STEP, type Action } from "@clench/host"
import type { Document } from "@clench/host/engine-types"
import { createSignal, For, Match, onCleanup, Show, Switch } from "solid-js"
import type { ReaderController, ReaderState } from "./controller"
import { actionForGesture, classifyGesture } from "./input"

export interface ViewProps {
  doc: Document
  controller: ReaderController
  /** Whether a close control is shown (the host handles closing). */
  closable: boolean
}

/** Key help for a browser: no suspend, and `q` closes rather than quits. */
function keyHelp(closable: boolean): ReadonlyArray<readonly [string, string]> {
  const rows: Array<readonly [string, string]> = KEY_HELP.filter(([k]) => !k.startsWith("ctrl-z") && !k.startsWith("q"))
  if (closable) rows.push(["q", "close the reader"])
  rows.push(["esc", "close this help"])
  return rows
}

export function ReaderView(props: ViewProps) {
  const controller = props.controller
  const [state, setState] = createSignal<ReaderState>(controller.state)
  onCleanup(controller.subscribe(setState))
  const act = (action: Action) => controller.act(action)
  const frame = () => state().frame
  const paused = () => frame().state !== "playing"

  return (
    <>
      <div class="cr-top">
        <Show when={state().view === "reading" || state().view === "help"}>
          <button type="button" aria-label="Keyboard shortcuts" aria-expanded={state().view === "help"} onClick={() => act({ type: "help" })}>
            ?
          </button>
        </Show>
        <div class="cr-top-title" aria-hidden={state().view === "ready" ? "true" : undefined}>
          <Show when={state().view !== "ready"}>{props.doc.title}</Show>
        </div>
        <Show when={props.closable}>
          <button type="button" aria-label="Close the reader" onClick={() => act({ type: "quit" })}>
            ✕
          </button>
        </Show>
      </div>
      <div class="cr-main">
        <Switch>
          <Match when={state().view === "ready"}>
            <ReadyScreen doc={props.doc} controller={controller} state={state()} />
          </Match>
          <Match when={state().view === "finished"}>
            <FinishedScreen controller={controller} state={state()} closable={props.closable} />
          </Match>
          <Match when={state().view === "help"}>
            <div class="cr-card" role="dialog" aria-label="Keyboard shortcuts">
              <h2 class="cr-title">Keys</h2>
              <table class="cr-keys">
                <tbody>
                  <For each={keyHelp(props.closable)}>
                    {([k, what]) => (
                      <tr>
                        <th scope="row">{k}</th>
                        <td>{what}</td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
              <p class="cr-hint">Touch: tap the word to play or pause, swipe left or right for the next or previous sentence.</p>
              <button type="button" class="cr-primary" onClick={() => act({ type: "close" })}>
                Back to reading
              </button>
            </div>
          </Match>
          <Match when={state().view === "reading"}>
            <Stage state={state()} onAction={act} />
            <Show when={paused() && state().context} fallback={<div class="cr-spacer" />}>
              <ContextView state={state()} />
            </Show>
            <Controls state={state()} onAction={act} controller={controller} />
            <Footer state={state()} />
          </Match>
        </Switch>
      </div>
      <p class="cr-sr" role="status" aria-live="polite">
        {state().view === "reading" ? (paused() ? "Paused. The current paragraph is shown as text." : "Playing.") : ""}
      </p>
    </>
  )
}

/** The fixation reticle and the current word, aligned on its pivot letter. */
function Stage(props: { state: ReaderState; onAction: (a: Action) => void }) {
  const f = () => props.state.frame
  const kind = () => (f().is_marker ? " cr-marker" : f().block_kind === "heading" ? " cr-heading" : "")
  const block = () => (f().block_start && f().index > 0 ? " cr-block" : "")
  let start: { x: number; y: number; t: number } | undefined
  const down = (e: PointerEvent) => {
    if (e.button !== 0) return
    start = { x: e.clientX, y: e.clientY, t: e.timeStamp }
  }
  const up = (e: PointerEvent) => {
    if (!start) return
    const gesture = classifyGesture(e.clientX - start.x, e.clientY - start.y, e.timeStamp - start.t)
    start = undefined
    if (gesture) props.onAction(actionForGesture(gesture))
  }
  return (
    <div class="cr-stage" onPointerDown={down} onPointerUp={up} onPointerCancel={() => (start = undefined)}>
      <div class={`cr-guide${block()}`} aria-hidden="true">
        <span />
        <span />
        <span />
      </div>
      <div class={`cr-word${kind()}`} aria-hidden="true" data-index={f().index}>
        <span class="cr-before">
          <span>{f().before}</span>
        </span>
        <span class="cr-pivot">{f().pivot}</span>
        <span class="cr-after">
          <span>{f().after}</span>
        </span>
      </div>
      <div class={`cr-guide cr-bottom${block()}`} aria-hidden="true">
        <span />
        <span />
        <span />
      </div>
    </div>
  )
}

/** The paused view's paragraph as real text, with the current word marked. */
function ContextView(props: { state: ReaderState }) {
  const ctx = () => props.state.context!
  const code = () => ctx().kind === "code" || ctx().kind === "table"
  let mark: HTMLElement | undefined
  const scrollIntoView = (el: HTMLElement) => {
    mark = el
    queueMicrotask(() => mark?.scrollIntoView?.({ block: "nearest" }))
  }
  return (
    <section class={`cr-context${code() ? " cr-code" : ""}`} aria-label="Current paragraph">
      <p>
        {ctx().before}
        <mark ref={scrollIntoView}>{ctx().token}</mark>
        {ctx().after}
      </p>
    </section>
  )
}

function WpmControl(props: { wpm: number; onAction: (a: Action) => void }) {
  return (
    <span class="cr-wpm" role="group" aria-label="Reading speed">
      <button type="button" aria-label={`Slower (${WPM_STEP} words per minute less)`} onClick={() => props.onAction({ type: "slower" })}>
        −
      </button>
      <output aria-live="off">{props.wpm} wpm</output>
      <button type="button" aria-label={`Faster (${WPM_STEP} words per minute more)`} onClick={() => props.onAction({ type: "faster" })}>
        +
      </button>
    </span>
  )
}

function Controls(props: { state: ReaderState; onAction: (a: Action) => void; controller: ReaderController }) {
  const playing = () => props.state.frame.state === "playing"
  return (
    <div class="cr-controls">
      <button type="button" aria-label="Previous paragraph" onClick={() => props.onAction({ type: "prevParagraph" })}>
        ⏮
      </button>
      <button type="button" aria-label="Previous sentence" onClick={() => props.onAction({ type: "prevSentence" })}>
        ◀
      </button>
      <button type="button" class="cr-primary cr-play" aria-label={playing() ? "Pause" : "Play"} onClick={() => props.onAction({ type: "toggle" })}>
        {playing() ? "Pause" : "Play"}
      </button>
      <button type="button" aria-label="Next sentence" onClick={() => props.onAction({ type: "nextSentence" })}>
        ▶
      </button>
      <button type="button" aria-label="Next paragraph" onClick={() => props.onAction({ type: "nextParagraph" })}>
        ⏭
      </button>
      <WpmControl wpm={props.state.frame.wpm} onAction={props.onAction} />
    </div>
  )
}

function Footer(props: { state: ReaderState }) {
  const f = () => props.state.frame
  const percent = () => Math.floor(Math.min(1, f().progress) * 100)
  return (
    <div class="cr-footer">
      <div class="cr-bar" role="progressbar" aria-label="Progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={percent()}>
        <div style={{ width: `${percent()}%` }} />
      </div>
      <div class="cr-stats">
        <span>{percent()}%</span>
        <span>{f().wpm} wpm</span>
        <span>{fmt.clock(f().remaining_ms)} left</span>
      </div>
    </div>
  )
}

function ReadyScreen(props: { doc: Document; controller: ReaderController; state: ReaderState }) {
  const session = props.controller.session
  const act = (a: Action) => props.controller.act(a)
  const omitted = notRead(session.omitted())
  const warn = warnings(props.doc).join(" ")
  const site = siteLabel(props.doc)
  return (
    <div class="cr-card">
      <h1 class="cr-title">{props.doc.title || "Untitled"}</h1>
      <Show when={site}>
        <p class="cr-muted">{site}</p>
      </Show>
      <p class="cr-stats-line">
        {fmt.count(session.wordCount)} words · {fmt.estimate(props.state.totalMs)} at {props.state.frame.wpm} wpm
      </p>
      <Show when={omitted}>
        <p class="cr-muted cr-not-read">{omitted}</p>
      </Show>
      <Show when={warn}>
        <p class="cr-warn" role="note">
          {warn}
        </p>
      </Show>
      <div class="cr-row">
        <WpmControl wpm={props.state.frame.wpm} onAction={act} />
      </div>
      <button type="button" class="cr-primary" style={{ "min-width": "10rem", "min-height": "3.25rem", "font-size": "1.125rem" }} onClick={() => act({ type: "toggle" })}>
        Start reading
      </button>
      <p class="cr-hint">space start · ↑↓ speed · ? keys</p>
    </div>
  )
}

function FinishedScreen(props: { controller: ReaderController; state: ReaderState; closable: boolean }) {
  const act = (a: Action) => props.controller.act(a)
  return (
    <div class="cr-card">
      <h2 class="cr-title">Finished.</h2>
      <p>{finishedSummary(props.controller.session.wordCount, props.state.playedMs, props.state.straight)}</p>
      <div class="cr-row">
        <button type="button" class="cr-primary" onClick={() => act({ type: "restart" })}>
          Read again
        </button>
        <Show when={props.closable}>
          <button type="button" onClick={() => act({ type: "quit" })}>
            Close
          </button>
        </Show>
      </div>
      <p class="cr-hint">space or r read again{props.closable ? " · q close" : ""}</p>
    </div>
  )
}
