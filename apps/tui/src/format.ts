/** `1815` as `1,815`. */
export function count(n: number): string {
  return Math.round(n).toLocaleString("en-US")
}

/** A duration as `m:ss`, or `h:mm:ss` from an hour up. */
export function clock(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = String(total % 60).padStart(2, "0")
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`
}

/** A reading-time estimate in words: `under a minute`, `about 7 min`, `about 1 h 5 min`. */
export function estimate(ms: number): string {
  const minutes = Math.round(ms / 60_000)
  if (minutes < 1) return "under a minute"
  if (minutes < 60) return `about ${minutes} min`
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  return m === 0 ? `about ${h} h` : `about ${h} h ${m} min`
}
