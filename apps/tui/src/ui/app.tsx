import { useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/solid"
import { createSignal, For, Match, onCleanup, Show, Switch } from "solid-js"
import { Driver, realClock, type Clock } from "../clock"
import type { Context, Document, Frame, Session } from "../engine"
import * as fmt from "../format"
import { actionFor, KEY_HELP, WPM_STEP, type Action } from "../keys"
import type { Style, Theme } from "../theme"
import { highlight, width as textWidth, window, wrap } from "../wrap"
import { Piece } from "./piece"
import { Reticle } from "./reticle"

export interface AppProps {
  doc: Document
  session: Session
  theme: Theme
  clock?: Clock
  /** Called after the renderer has been destroyed on quit. */
  onQuit?: () => void
}

/** Rows used by the parts of the reading view at a given terminal size. */
export function layout(width: number, height: number) {
  const footer = height - 1
  const word = Math.max(1, Math.floor(footer * 0.4))
  const hint = height >= 8 ? height - 2 : -1
  const contextTop = word + 3
  const contextRows = Math.max(0, (hint >= 0 ? hint - 1 : footer) - contextTop)
  const contextWidth = Math.max(10, Math.min(width - 4, 76))
  return { footer, word, hint, contextTop, contextRows, contextWidth }
}

function Line(props: { text: string; style: Style; top: number; left?: number; width: number }) {
  return (
    <text
      position="absolute"
      top={props.top}
      left={props.left ?? 0}
      width={props.width}
      height={1}
      wrapMode="none"
      fg={props.style.fg}
      attributes={props.style.attributes}
    >
      {props.text}
    </text>
  )
}

function truncate(text: string, columns: number): string {
  if (textWidth(text) <= columns) return text
  let out = ""
  for (const ch of text) {
    if (textWidth(out + ch) > columns - 1) break
    out += ch
  }
  return out + "…"
}

/** Text centred in `columns`, by display width. */
function centred(text: string, columns: number): string {
  const t = truncate(text, columns)
  return " ".repeat(Math.max(0, Math.floor((columns - textWidth(t)) / 2))) + t
}

export function siteLabel(doc: Document): string {
  const parts: string[] = []
  if (doc.byline) parts.push(doc.byline)
  let site = doc.site_name ?? ""
  if (!site && doc.url) {
    try {
      site = new URL(doc.url).hostname.replace(/^www\./, "")
    } catch {}
  }
  if (site && site !== doc.byline) parts.push(site)
  return parts.join(" · ")
}

export function warnings(doc: Document): string[] {
  if (doc.partial) {
    return [
      "Only part of this page could be extracted. The rest may be loaded by",
      "JavaScript or sit behind a paywall, so this may not be the whole article.",
    ]
  }
  if (doc.paywall_hint) return ["The page is marked as paywalled; check that the text is complete."]
  return []
}

function Footer(props: { frame: Frame; width: number; top: number; theme: Theme }) {
  const stats = () => {
    const f = props.frame
    return ` ${Math.floor(f.progress * 100)}%  ${f.wpm} wpm  ${fmt.clock(f.remaining_ms)} left`
  }
  const barWidth = () => Math.max(0, props.width - textWidth(stats()) - 1)
  const filled = () => Math.round(barWidth() * Math.min(1, props.frame.progress))
  return (
    <box position="absolute" top={props.top} left={0} width={props.width} height={1} flexDirection="row">
      <Show when={barWidth() >= 8}>
        <Piece text={"━".repeat(filled())} style={props.theme.bar} />
        <Piece text={"─".repeat(barWidth() - filled())} style={props.theme.guide} />
      </Show>
      <Piece text={truncate(stats(), props.width)} style={props.theme.muted} />
    </box>
  )
}

/** The paused view's paragraph, wrapped, with the current word highlighted. */
function ContextView(props: { context: Context; top: number; rows: number; width: number; screen: number; theme: Theme }) {
  const text = () => props.context.before + props.context.token + props.context.after
  const from = () => props.context.before.length
  const to = () => from() + props.context.token.length
  const marker = () => props.context.kind === "code" || props.context.kind === "table"
  const shown = () => window(wrap(text(), props.width), from(), props.rows)
  const left = () => Math.max(0, Math.floor((props.screen - props.width) / 2))
  const base = () => (marker() ? props.theme.muted : props.theme.text)
  return (
    <For each={shown().lines}>
      {(line, i) => {
        const parts = () => highlight(line, from(), to())
        return (
          <box position="absolute" top={props.top + i()} left={left()} width={props.width} height={1} flexDirection="row">
            <Piece text={parts().before} style={base()} />
            <Piece text={parts().mark} style={marker() ? base() : props.theme.mark} />
            <Piece text={parts().after} style={base()} />
          </box>
        )
      }}
    </For>
  )
}

export function App(props: AppProps) {
  const renderer = useRenderer()
  const dims = useTerminalDimensions()
  const session = props.session
  const theme = props.theme
  const [frame, setFrame] = createSignal(session.frame())
  const [context, setContext] = createSignal<Context | undefined>(session.context())
  const [started, setStarted] = createSignal(false)
  const [help, setHelp] = createSignal(false)
  const [played, setPlayed] = createSignal(0)

  const refresh = () => {
    const f = session.frame()
    setFrame(f)
    if (f.state !== "playing") setContext(session.context())
    setPlayed(driver.playedMs)
  }
  const driver = new Driver(session, props.clock ?? realClock, refresh)
  onCleanup(() => driver.dispose())

  const quit = () => {
    driver.dispose()
    renderer.destroy()
    props.onQuit?.()
  }

  const control = (fn: () => void) => driver.control(fn)
  const view = () => {
    if (help()) return "help"
    if (!started()) return "ready"
    return frame().state === "finished" ? "finished" : "reading"
  }

  const act = (action: Action) => {
    if (action.type === "quit") return quit()
    if (action.type === "help") {
      if (help()) return setHelp(false)
      control(() => session.pause())
      return setHelp(true)
    }
    if (action.type === "close") return setHelp(false)
    if (help()) return
    const finished = frame().state === "finished"
    switch (action.type) {
      case "toggle":
        setStarted(true)
        return control(() => session.toggle())
      case "restart":
        setStarted(true)
        return control(() => {
          session.restart()
          if (finished) session.play()
        })
      case "faster":
        return control(() => session.nudgeWpm(WPM_STEP))
      case "slower":
        return control(() => session.nudgeWpm(-WPM_STEP))
    }
    setStarted(true)
    switch (action.type) {
      case "prevSentence":
        return control(() => session.prevSentence())
      case "nextSentence":
        return control(() => session.nextSentence())
      case "prevParagraph":
        return control(() => session.prevParagraph())
      case "nextParagraph":
        return control(() => session.nextParagraph())
      case "stepBack":
        return control(() => session.step(-1))
      case "stepForward":
        return control(() => session.step(1))
      case "jump":
        return control(() => session.seekFraction(action.tenth / 10))
    }
  }

  useKeyboard((key) => {
    if (key.eventType === "release") return
    const action = actionFor(key)
    if (action) act(action)
  })

  const W = () => dims().width
  const H = () => dims().height
  const rows = () => layout(W(), H())
  const paused = () => frame().state !== "playing"

  return (
    <box width={W()} height={H()}>
      <Switch>
        <Match when={view() === "ready"}>
          <ReadyScreen doc={props.doc} session={session} frame={frame()} width={W()} height={H()} theme={theme} />
        </Match>
        <Match when={view() === "finished"}>
          <FinishedScreen words={session.wordCount} playedMs={played()} width={W()} height={H()} theme={theme} />
        </Match>
        <Match when={view() === "help"}>
          <HelpScreen width={W()} height={H()} theme={theme} />
        </Match>
        <Match when={view() === "reading"}>
          <Reticle frame={frame()} width={W()} top={rows().word - 1} theme={theme} />
          <Show when={paused()}>
            <Show when={rows().word >= 3}>
              <Line text={centred(props.doc.title, W())} style={theme.muted} top={0} width={W()} />
            </Show>
            <Show when={context() && rows().contextRows > 0}>
              <ContextView
                context={context()!}
                top={rows().contextTop}
                rows={rows().contextRows}
                width={rows().contextWidth}
                screen={W()}
                theme={theme}
              />
            </Show>
            <Show when={rows().hint >= 0}>
              <Line
                text={centred("paused · space resume · ←→ sentence · [ ] paragraph · ↑↓ speed · ? keys · q quit", W())}
                style={theme.muted}
                top={rows().hint}
                width={W()}
              />
            </Show>
          </Show>
        </Match>
      </Switch>
      <Show when={view() === "reading"}>
        <Footer frame={frame()} width={W()} top={rows().footer} theme={theme} />
      </Show>
    </box>
  )
}

function ReadyScreen(props: { doc: Document; session: Session; frame: Frame; width: number; height: number; theme: Theme }) {
  const inner = () => Math.max(10, Math.min(props.width - 4, 72))
  const title = () => wrap(props.doc.title || "Untitled", inner()).slice(0, 3)
  const lines = () => {
    const out: Array<{ text: string; style: Style }> = []
    for (const l of title()) out.push({ text: l.text, style: props.theme.title })
    const site = siteLabel(props.doc)
    if (site) out.push({ text: site, style: props.theme.muted })
    out.push({ text: "", style: props.theme.text })
    out.push({
      text: `${fmt.count(props.session.wordCount)} words · ${fmt.estimate(props.session.totalMs)} at ${props.frame.wpm} wpm`,
      style: props.theme.text,
    })
    const warn = warnings(props.doc)
    if (warn.length > 0) {
      out.push({ text: "", style: props.theme.text })
      for (const w of warn) for (const l of wrap(w, inner())) out.push({ text: l.text, style: props.theme.warn })
    }
    out.push({ text: "", style: props.theme.text })
    out.push({ text: "space start · ↑↓ speed · ? keys · q quit", style: props.theme.muted })
    return out
  }
  const top = () => Math.max(0, Math.floor((props.height - lines().length) / 2.5))
  return (
    <For each={lines()}>
      {(l, i) => <Line text={centred(l.text, props.width)} style={l.style} top={top() + i()} width={props.width} />}
    </For>
  )
}

function FinishedScreen(props: { words: number; playedMs: number; width: number; height: number; theme: Theme }) {
  const rate = () => (props.playedMs > 0 ? Math.round(props.words / (props.playedMs / 60_000)) : 0)
  const lines = () => [
    { text: "Finished.", style: props.theme.title },
    { text: "", style: props.theme.text },
    {
      text: `${fmt.count(props.words)} words in ${fmt.clock(props.playedMs)}` + (rate() > 0 ? ` · ${fmt.count(rate())} wpm overall` : ""),
      style: props.theme.text,
    },
    { text: "", style: props.theme.text },
    { text: "r read again · q quit", style: props.theme.muted },
  ]
  const top = () => Math.max(0, Math.floor((props.height - lines().length) / 2.5))
  return (
    <For each={lines()}>
      {(l, i) => <Line text={centred(l.text, props.width)} style={l.style} top={top() + i()} width={props.width} />}
    </For>
  )
}

function HelpScreen(props: { width: number; height: number; theme: Theme }) {
  const keyCol = Math.max(...KEY_HELP.map(([k]) => textWidth(k)))
  const rows = () => [
    { text: "Keys", style: props.theme.title },
    { text: "", style: props.theme.text },
    ...KEY_HELP.map(([k, what]) => ({ text: k + " ".repeat(keyCol + 3 - textWidth(k)) + what, style: props.theme.text })),
    { text: "", style: props.theme.text },
    { text: "? or esc to close", style: props.theme.muted },
  ]
  const blockWidth = () => Math.max(...rows().map((r) => textWidth(r.text)))
  const left = () => Math.max(0, Math.floor((props.width - blockWidth()) / 2))
  const top = () => Math.max(0, Math.floor((props.height - rows().length) / 2.5))
  return (
    <For each={rows()}>
      {(r, i) => <Line text={truncate(r.text, props.width - left())} style={r.style} top={top() + i()} left={left()} width={props.width - left()} />}
    </For>
  )
}
