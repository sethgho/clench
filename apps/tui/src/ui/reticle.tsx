import type { Frame } from "../engine"
import type { Theme } from "../theme"
import { clipStart } from "../wrap"
import { Piece } from "./piece"

/** The column the pivot letter is drawn at: the middle of the screen. */
export function pivotColumn(width: number): number {
  return Math.max(0, Math.floor((width - 1) / 2))
}

/** Half-width of the guide lines either side of the pivot column. */
const GUIDE_REACH = 12

function guide(width: number, tick: string, line: string): string {
  const pc = pivotColumn(width)
  const reach = Math.max(0, Math.min(GUIDE_REACH, pc, width - pc - 1))
  return " ".repeat(pc - reach) + line.repeat(reach) + tick + line.repeat(reach)
}

/**
 * The fixation reticle: guide marks above and below the pivot column, and
 * the current word with its pivot grapheme on that column. The text before
 * the pivot is right-aligned against it and the text after starts right
 * after it, so alignment follows the renderer's own display-width
 * measurement (wide and combining characters included). Overflow on either
 * side is clipped rather than shifting the pivot. Heading words are bold,
 * and the guide lines thicken on the first word of a new block.
 */
export function Reticle(props: { frame: Frame; width: number; top: number; theme: Theme; blockStart?: boolean }) {
  const heading = () => props.frame.block_kind === "heading" && !props.frame.is_marker
  const word = () => (props.frame.is_marker ? props.theme.marker : heading() ? props.theme.heading : props.theme.text)
  const line = () => (props.blockStart ? "━" : "─")
  const pivot = () => (props.frame.is_marker ? props.theme.marker : props.theme.pivot)
  return (
    <box position="absolute" top={props.top} left={0} width={props.width} height={3} flexDirection="column">
      <text fg={props.theme.guide.fg} attributes={props.theme.guide.attributes} wrapMode="none" height={1}>
        {guide(props.width, "┬", line())}
      </text>
      <box flexDirection="row" width={props.width} height={1}>
        <box
          width={pivotColumn(props.width)}
          height={1}
          flexDirection="row"
          justifyContent="flex-end"
          overflow="hidden"
          flexShrink={0}
        >
          <Piece text={clipStart(props.frame.before, pivotColumn(props.width))} style={word()} />
        </box>
        <Piece text={props.frame.pivot} style={pivot()} />
        <box flexGrow={1} flexShrink={1} height={1} flexDirection="row" overflow="hidden">
          <Piece text={props.frame.after} style={word()} />
        </box>
      </box>
      <text fg={props.theme.guide.fg} attributes={props.theme.guide.attributes} wrapMode="none" height={1}>
        {guide(props.width, "┴", line())}
      </text>
    </box>
  )
}
