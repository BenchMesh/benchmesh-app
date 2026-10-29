/**
 * The top bar's statuses (`StatusPill`): what each one says and its tooltip. **API** - the
 * BenchMesh server answers this window; **Readings** - the instruments' values stream in. Both are
 * about the link from this window to BenchMesh; whether an instrument answers shows on its panel.
 */
import type { StreamState } from '../data'

/** ok: green; warn: amber; alert: red; off: grey. */
export type PillState = 'ok' | 'warn' | 'alert' | 'off'

export interface Status {
  label: string
  title: string
  state: PillState
}

const PANELS = 'Whether an instrument itself answers shows on its panel.'

export function apiStatus(loading: boolean, error: string | null): Status {
  if (error) {
    return { label: 'API unreachable', state: 'alert', title: `The BenchMesh server does not answer this window (${error}). BenchMesh tries again every second; until then nothing can be changed.` }
  }
  if (loading) return { label: 'API connecting…', state: 'warn', title: 'Waiting for the BenchMesh server to answer for the first time.' }
  return { label: 'API', state: 'ok', title: `The BenchMesh server answers this window: the instrument list, settings and every control go through it. ${PANELS}` }
}

export function readingsStatus(state: StreamState): Status {
  switch (state) {
    case 'receiving':
    case 'open':
      return { label: 'Readings', state: 'ok', title: `The instruments' readings arrive from the BenchMesh server about five times a second: the panels are current. ${PANELS}` }
    case 'stale':
      return { label: 'Readings stalled', state: 'warn', title: 'No new readings for more than a second: the values on the panels may be old. Usually a busy computer or network; it recovers by itself.' }
    case 'closed':
      return { label: 'Readings lost', state: 'alert', title: 'The stream of readings from the BenchMesh server is down: the panels show the last values received. Reconnecting every second.' }
    default:
      return { label: 'Readings connecting…', state: 'warn', title: 'Opening the stream of readings from the BenchMesh server.' }
  }
}
