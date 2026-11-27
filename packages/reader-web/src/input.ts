import { actionFor, type Action } from "@clench/host"

/** The fields of a browser `KeyboardEvent` the reader looks at. */
export interface BrowserKey {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
  shiftKey: boolean
}

const NAMES: Record<string, string> = {
  " ": "space",
  ArrowLeft: "left",
  ArrowRight: "right",
  ArrowUp: "up",
  ArrowDown: "down",
  Escape: "escape",
}

/**
 * The reader action for a browser key press, using the terminal reader's key
 * map. Presses with ctrl, meta or alt are left to the browser (copy, reload,
 * tab switching), so there is no ctrl-c quit or ctrl-z suspend here.
 */
export function actionForKey(event: BrowserKey): Action | undefined {
  if (event.ctrlKey || event.metaKey || event.altKey) return undefined
  const name = NAMES[event.key] ?? event.key
  return actionFor({ name, sequence: event.key.length === 1 ? event.key : "", ctrl: false, meta: false, shift: event.shiftKey })
}

/** Movement under this many CSS pixels, both ways, is a tap. */
export const TAP_SLOP = 12
/** A swipe must travel this far horizontally… */
export const SWIPE_MIN = 48
/** …in at most this long. */
export const SWIPE_MAX_MS = 800

export type Gesture = "tap" | "swipeLeft" | "swipeRight"

/**
 * Classifies a pointer gesture from its displacement and duration. A swipe
 * is mostly horizontal; anything else (a scroll, a long drag) is ignored.
 */
export function classifyGesture(dx: number, dy: number, ms: number): Gesture | undefined {
  if (Math.abs(dx) < TAP_SLOP && Math.abs(dy) < TAP_SLOP) return ms < SWIPE_MAX_MS ? "tap" : undefined
  if (ms > SWIPE_MAX_MS || Math.abs(dx) < SWIPE_MIN || Math.abs(dx) < Math.abs(dy) * 1.5) return undefined
  return dx < 0 ? "swipeLeft" : "swipeRight"
}

/** Swiping left moves forward, as when turning a page. */
export function actionForGesture(gesture: Gesture): Action {
  switch (gesture) {
    case "tap":
      return { type: "toggle" }
    case "swipeLeft":
      return { type: "nextSentence" }
    case "swipeRight":
      return { type: "prevSentence" }
  }
}
