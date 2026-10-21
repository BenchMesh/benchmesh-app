/**
 * Parking (`ai_context/PARKING.md`): a parked instrument has no link - nothing is sent to it, the
 * server only checks whether it is attached / reachable. Its panels leave the rack for the
 * parking strip and return to their place when it is unparked.
 */
import { useCallback, useState } from 'react'
import { apiBaseUrl } from '../../api/base'
import { loggedFetch, useRequestLog } from '../RequestLogContext'
import { Instrument } from '../types'
import { isActiveState, RunStatus } from '../automation/types'

/** Presence of a parked instrument: `present` null until first checked; `detail` where, or why not. */
export interface ParkedState { present: boolean | null; detail: string }

/**
 * Whether the instrument is parked: the live registry decides once it lists the instrument
 * (pushed every second), the instrument list (polled) until then.
 */
export function isParked(instrument: Instrument, registry: any): boolean {
  const live = registry?.[instrument.id]
  return live && typeof live === 'object' ? Boolean(live.parked) : Boolean(instrument.parked)
}

/** Presence of a parked instrument (live registry first). */
export function parkedState(instrument: Instrument, registry: any): ParkedState {
  const live = registry?.[instrument.id]?.parked
  const src = live ?? { present: instrument.health?.present, detail: instrument.health?.presence }
  return { present: typeof src.present === 'boolean' ? src.present : null, detail: src.detail ?? '' }
}

/** Status fields that say an output (or a load input, an LCR bias) is switched on. */
const ENERGIZED_FIELDS = ['OUTPUT', 'INPUT', 'BIAS_ON']
const isOn = (v: unknown) => v === true || v === 1 || (typeof v === 'string' && ['ON', '1'].includes(v.trim().toUpperCase()))

/** What is switched on right now, e.g. `PSU ch1 OUTPUT` (it stays on, unwatched, when parked). */
export function energizedOutputs(instrument: Instrument, registry: any): string[] {
  const live = registry?.[instrument.id] ?? {}
  const out: string[] = []
  for (const { class: klass } of instrument.classes) {
    for (const [key, status] of Object.entries(live[klass] ?? {})) {
      const ch = key.match(/^status_ch(\d+)$/)?.[1]
      if (!ch || !status || typeof status !== 'object') continue
      for (const f of ENERGIZED_FIELDS) if (isOn((status as Record<string, unknown>)[f])) out.push(`${klass} ch${ch} ${f}`)
    }
  }
  return out
}

/** The running flow's name when it uses the instrument (it cannot be parked then). */
export function runUsing(run: RunStatus | undefined, id: string): string | null {
  return run && isActiveState(run.state) && run.devices?.includes(id) ? run.flow?.name ?? 'a flow' : null
}

/** `POST` (park) / `DELETE` (unpark) `/devices/{id}/park`; rejects with the server's `detail`. */
export function useParking() {
  const { addLog } = useRequestLog()
  return useCallback(async (id: string, parked: boolean) => {
    const r = await loggedFetch(`${apiBaseUrl()}/devices/${encodeURIComponent(id)}/park`, {
      method: parked ? 'POST' : 'DELETE', instrument: id, action: parked ? 'Park' : 'Unpark', addLog,
    })
    if (!r.ok) {
      let detail = `HTTP ${r.status}`
      try { const j = await r.json(); if (typeof j?.detail === 'string') detail = j.detail } catch { /* not JSON */ }
      throw new Error(detail)
    }
  }, [addLog])
}

/**
 * The park control of a panel header: refused while the running flow uses the instrument; with
 * an output on, the first press asks (`asking`) and a second press within 5 s parks.
 */
export function useParkControl(instrument: Instrument, registry: any, run: RunStatus | undefined, onParked?: () => void) {
  const setParked = useParking()
  const [asking, setAsking] = useState<string[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const blockedBy = runUsing(run, instrument.id)
  const press = useCallback(async () => {
    if (blockedBy || busy) return
    const on = energizedOutputs(instrument, registry)
    if (on.length && !asking) {
      setAsking(on)
      setTimeout(() => setAsking(null), 5000)
      return
    }
    setAsking(null)
    setBusy(true)
    setError(null)
    try {
      await setParked(instrument.id, true)
      onParked?.()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }, [blockedBy, busy, instrument, registry, asking, setParked, onParked])
  return { press, asking, busy, error, blockedBy, dismiss: () => { setAsking(null); setError(null) } }
}
