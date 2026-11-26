import { RGBA, TextAttributes } from "@opentui/core"

/** A foreground colour and text attributes. */
export interface Style {
  fg: RGBA
  attributes: number
}

/** Every style the interface uses. */
export interface Theme {
  text: Style
  title: Style
  heading: Style
  muted: Style
  pivot: Style
  marker: Style
  guide: Style
  warn: Style
  mark: Style
  bar: Style
}

const { NONE, BOLD, DIM, ITALIC, UNDERLINE, INVERSE } = TextAttributes

/**
 * The palette. Colours are the terminal's own indexed colours and default
 * foreground, so the reader's theme (light or dark) decides how they look;
 * the background is never painted. With `color` off (`NO_COLOR`) only text
 * attributes distinguish the roles.
 */
export function makeTheme(color: boolean): Theme {
  const text = RGBA.defaultForeground()
  const indexed = (n: number) => (color ? RGBA.fromIndex(n) : text)
  const s = (fg: RGBA, attributes: number): Style => ({ fg, attributes })
  return {
    text: s(text, NONE),
    title: s(text, BOLD),
    heading: s(text, BOLD),
    muted: s(indexed(8), color ? NONE : DIM),
    pivot: s(indexed(1), color ? BOLD : BOLD | UNDERLINE),
    marker: s(indexed(6), color ? ITALIC : ITALIC | DIM),
    guide: s(indexed(8), color ? NONE : DIM),
    warn: s(indexed(3), BOLD),
    mark: s(indexed(1), color ? BOLD | UNDERLINE : INVERSE),
    bar: s(indexed(1), NONE),
  }
}

/** Whether colour output is wanted, following https://no-color.org. */
export function colorWanted(env: Record<string, string | undefined> = process.env): boolean {
  return !env.NO_COLOR
}
