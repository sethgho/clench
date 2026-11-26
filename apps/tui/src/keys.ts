export type Action =
  | { type: "toggle" }
  | { type: "prevSentence" }
  | { type: "nextSentence" }
  | { type: "prevParagraph" }
  | { type: "nextParagraph" }
  | { type: "faster" }
  | { type: "slower" }
  | { type: "stepBack" }
  | { type: "stepForward" }
  | { type: "restart" }
  | { type: "jump"; tenth: number }
  | { type: "help" }
  | { type: "close" }
  | { type: "quit" }

export interface KeyLike {
  name: string
  sequence: string
  ctrl: boolean
  shift: boolean
  meta: boolean
}

/** WPM change per speed key press. */
export const WPM_STEP = 25

/** The action bound to a key press, if any. */
export function actionFor(key: KeyLike): Action | undefined {
  if (key.ctrl) return key.name === "c" ? { type: "quit" } : undefined
  if (key.meta) return undefined
  switch (key.name) {
    case "space":
      return { type: "toggle" }
    case "left":
      return { type: key.shift ? "prevParagraph" : "prevSentence" }
    case "right":
      return { type: key.shift ? "nextParagraph" : "nextSentence" }
    case "up":
      return { type: "faster" }
    case "down":
      return { type: "slower" }
    case "escape":
      return { type: "close" }
  }
  const ch = key.sequence.length === 1 ? key.sequence : key.name
  switch (ch) {
    case " ":
      return { type: "toggle" }
    case "h":
      return { type: "prevSentence" }
    case "l":
      return { type: "nextSentence" }
    case "[":
      return { type: "prevParagraph" }
    case "]":
      return { type: "nextParagraph" }
    case "k":
      return { type: "faster" }
    case "j":
      return { type: "slower" }
    case ",":
      return { type: "stepBack" }
    case ".":
      return { type: "stepForward" }
    case "r":
      return { type: "restart" }
    case "?":
      return { type: "help" }
    case "q":
      return { type: "quit" }
  }
  if (/^[0-9]$/.test(ch)) return { type: "jump", tenth: Number(ch) }
  return undefined
}

/** Key bindings as shown in the help overlay. */
export const KEY_HELP: ReadonlyArray<readonly [string, string]> = [
  ["space", "play / pause"],
  ["← h  → l", "previous / next sentence"],
  ["[  ]  ⇧←  ⇧→", "previous / next paragraph"],
  ["↑ k  ↓ j", `faster / slower (${WPM_STEP} wpm)`],
  [",  .", "back / forward one word"],
  ["0 – 9", "jump to that tenth"],
  ["r", "restart"],
  ["?", "this help"],
  ["q  ctrl-c", "quit"],
]
