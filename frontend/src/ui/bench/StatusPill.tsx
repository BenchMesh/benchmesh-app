/**
 * One status in the top bar: a dot and a word, the full explanation as its tooltip. With `onOpen`
 * it is a button that opens where the status is dealt with. Distributions use it for their own
 * statuses (`extensions.topBarStatus`), so every status looks the same.
 */
import type { PillState } from './status'

export interface StatusPillProps {
  label: string
  /** tooltip: what the status means */
  title: string
  state: PillState
  /** makes the pill a button */
  onOpen?: () => void
}

export function StatusPill({ label, title, state, onOpen }: StatusPillProps) {
  const body = <><i aria-hidden="true" />{label}</>
  // data-ok: green or not (kept for tests and scripts that read it)
  const attrs = { className: 'bench-pill', 'data-ok': state === 'ok', 'data-state': state, title }
  return onOpen
    ? <button type="button" {...attrs} onClick={onOpen}>{body}</button>
    : <span {...attrs}>{body}</span>
}
