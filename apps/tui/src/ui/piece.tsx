import { Show } from "solid-js"
import type { Style } from "../theme"

/** A run of styled text inside a row. Empty runs are left out: an empty text element still takes a cell. */
export function Piece(props: { text: string; style: Style }) {
  return (
    <Show when={props.text.length > 0}>
      <text fg={props.style.fg} attributes={props.style.attributes} wrapMode="none" flexShrink={0}>
        {props.text}
      </text>
    </Show>
  )
}
