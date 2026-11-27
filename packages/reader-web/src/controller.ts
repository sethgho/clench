import { Driver, realClock, WPM_STEP, type Action, type Clock } from "@clench/host"
import type { Context, Frame, Omitted } from "@clench/host/engine-types"

/** The part of the engine's `Session` the reader drives. */
export interface ReaderSession {
  readonly state: "paused" | "playing" | "finished"
  readonly wordCount: number
  readonly totalMs: number
  frame(): Frame
  context(): Context | undefined
  omitted(): Omitted[]
  advance(ms: number): boolean
  nextChangeIn(): number | undefined
  play(): void
  pause(): void
  toggle(): void
  restart(): void
  step(delta: number): void
  nextSentence(): void
  prevSentence(): void
  nextParagraph(): void
  prevParagraph(): void
  seekFraction(fraction: number): void
  nudgeWpm(delta: number): void
  setWpm(wpm: number): void
}

export type View = "ready" | "reading" | "finished" | "help"

/** Everything the view draws, as one immutable snapshot. */
export interface ReaderState {
  view: View
  frame: Frame
  /** The current word's block; refreshed whenever playback is not running. */
  context: Context | undefined
  /** Wall-clock time spent playing in this run, ms. */
  playedMs: number
  /** No jump was made in this run, so an overall rate is meaningful. */
  straight: boolean
  /** Estimated time for the whole script at the current WPM, ms. */
  totalMs: number
}

export interface ControllerOptions {
  clock?: Clock
  /** Called for `q`; without it the key does nothing. */
  onClose?: () => void
  /** Called after any change of words per minute. */
  onWpmChange?: (wpm: number) => void
}

/**
 * The reader's state machine. It owns the clock driver, applies actions the
 * way the terminal reader does (ready screen, help overlay, finished screen,
 * straight-read tracking) and publishes snapshots; it never touches the DOM.
 */
export class ReaderController {
  private readonly driver: Driver
  private readonly listeners = new Set<(state: ReaderState) => void>()
  private started = false
  private help = false
  private straight = true
  private context: Context | undefined
  private current: ReaderState

  constructor(
    readonly session: ReaderSession,
    private readonly options: ControllerOptions = {},
  ) {
    this.context = session.context()
    this.driver = new Driver(session, options.clock ?? realClock, () => this.refresh())
    this.current = this.snapshot()
  }

  get state(): ReaderState {
    return this.current
  }

  subscribe(listener: (state: ReaderState) => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  dispose(): void {
    this.driver.dispose()
    this.listeners.clear()
  }

  /** Pauses without leaving the current screen, e.g. when the page is hidden. */
  pause(): void {
    if (this.session.state === "playing") this.control(() => this.session.pause())
  }

  setWpm(wpm: number): void {
    this.control(() => this.session.setWpm(wpm))
    this.options.onWpmChange?.(this.session.frame().wpm)
  }

  act(action: Action): void {
    const session = this.session
    switch (action.type) {
      case "quit":
        this.pause()
        this.options.onClose?.()
        return
      case "suspend":
        return
      case "help":
        if (this.help) return this.setHelp(false)
        this.control(() => session.pause())
        return this.setHelp(true)
      case "close":
        return this.setHelp(false)
    }
    if (this.help) return
    const finished = session.state === "finished"
    switch (action.type) {
      case "toggle":
        this.started = true
        if (finished) this.newRun()
        return this.control(() => session.toggle())
      case "restart":
        this.started = true
        this.newRun()
        return this.control(() => {
          session.restart()
          if (finished) session.play()
        })
      case "faster":
      case "slower":
        this.control(() => session.nudgeWpm(action.type === "faster" ? WPM_STEP : -WPM_STEP))
        this.options.onWpmChange?.(session.frame().wpm)
        return
    }
    this.started = true
    this.straight = false
    switch (action.type) {
      case "prevSentence":
        return this.control(() => session.prevSentence())
      case "nextSentence":
        return this.control(() => session.nextSentence())
      case "prevParagraph":
        return this.control(() => session.prevParagraph())
      case "nextParagraph":
        return this.control(() => session.nextParagraph())
      case "stepBack":
        return this.control(() => session.step(-1))
      case "stepForward":
        return this.control(() => session.step(1))
      case "jump":
        return this.control(() => session.seekFraction(action.tenth / 10))
    }
  }

  private control(fn: () => void): void {
    this.driver.control(fn)
  }

  private setHelp(open: boolean): void {
    this.help = open
    this.publish()
  }

  /** Starts a fresh reading run: time and "straight read" tracking begin again. */
  private newRun(): void {
    this.driver.resetPlayed()
    this.straight = true
  }

  private refresh(): void {
    if (this.session.state !== "playing") this.context = this.session.context()
    this.publish()
  }

  private publish(): void {
    this.current = this.snapshot()
    for (const listener of this.listeners) listener(this.current)
  }

  private snapshot(): ReaderState {
    const frame = this.session.frame()
    const view: View = this.help ? "help" : !this.started ? "ready" : frame.state === "finished" ? "finished" : "reading"
    return { view, frame, context: this.context, playedMs: this.driver.playedMs, straight: this.straight, totalMs: this.session.totalMs }
  }
}
