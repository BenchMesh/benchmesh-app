/** Instrument API access for class panels: metadata, channel get/set, live status, history. */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { apiBaseUrl } from '../../api/base'
import { loggedFetch, useRequestLog } from '../RequestLogContext'
import { setterMap, Setters } from './profiles'

/** Features and setter specs of one instrument class (`null` until loaded). */
export function useInstrumentMeta(klass: string, id: string) {
  const [meta, setMeta] = useState<{ features: any; setters: Setters } | null>(null)
  useEffect(() => {
    let cancelled = false
    const base = `${apiBaseUrl()}/instruments/${klass}/${id}`
    Promise.all([fetch(base).then((r) => (r.ok ? r.json() : {})), fetch(`${base}/methods`).then((r): Promise<{ methods?: any[] }> => (r.ok ? r.json() : Promise.resolve({})))])
      .then(([features, methods]) => { if (!cancelled) setMeta({ features, setters: setterMap(methods?.methods ?? []) }) })
      .catch(() => { if (!cancelled) setMeta({ features: {}, setters: {} }) })
    return () => { cancelled = true }
  }, [klass, id])
  return meta
}

async function errorText(r: Response): Promise<string> {
  try {
    const j = await r.json()
    if (typeof j?.detail === 'string') return j.detail
  } catch { /* not JSON */ }
  return `HTTP ${r.status}`
}

export interface ChannelApi {
  /** `GET .../{op}` → the reply's `value`. */
  get: (op: string) => Promise<unknown>
  /** `POST .../{op}/{value}`; rejects with the server's `detail` message. */
  set: (op: string, value: string | number) => Promise<void>
}

export function useChannelApi(klass: string, id: string, ch: string): ChannelApi {
  const { addLog } = useRequestLog()
  const get = useCallback(async (op: string) => {
    const r = await loggedFetch(`${apiBaseUrl()}/instruments/${klass}/${id}/${ch}/${op}`, {
      method: 'GET', instrument: id, channel: ch, action: `Query ${op}`, addLog,
    })
    if (!r.ok) throw new Error(await errorText(r))
    return (await r.json())?.value
  }, [klass, id, ch, addLog])
  const set = useCallback(async (op: string, value: string | number) => {
    const r = await loggedFetch(`${apiBaseUrl()}/instruments/${klass}/${id}/${ch}/${op}/${encodeURIComponent(String(value))}`, {
      method: 'POST', instrument: id, channel: ch, action: `Set ${op}`, parameters: { value }, addLog,
    })
    if (!r.ok) throw new Error(await errorText(r))
  }, [klass, id, ch, addLog])
  return useMemo(() => ({ get, set }), [get, set])
}

/** Registry status block of one channel (`{}` while unknown). */
export function channelStatus(registry: any, id: string, klass: string, ch: string): Record<string, any> {
  return registry?.[id]?.[klass]?.[`status_ch${ch}`] ?? {}
}

export function isOnline(registry: any, instrument: { id: string; health?: { is_alive?: boolean } }): boolean {
  const idn = registry?.[instrument.id]?.IDN
  return Boolean(idn && String(idn).trim()) && instrument.health?.is_alive !== false
}

/**
 * Last `capacity` values of a reading, one sample per status update (`tick` changes on every
 * registry message). Restarts when `resetKey` changes (e.g. the meter function).
 * Derived during render (React's "adjust state when a prop changes" pattern).
 */
export function useHistory(value: number | null, tick: unknown, capacity = 150, resetKey?: unknown): number[] {
  const [h, setH] = useState<{ tick: unknown; key: unknown; data: number[] }>({ tick, key: resetKey, data: value == null ? [] : [value] })
  if (h.tick !== tick || h.key !== resetKey) {
    const base = h.key !== resetKey ? [] : h.data
    const data = value == null ? base : [...base.slice(Math.max(0, base.length - capacity + 1)), value]
    setH({ tick, key: resetKey, data })
    return data
  }
  return h.data
}

/** A setpoint read from the instrument when `op` changes (and set locally after every commit). */
export function useSetpoint(api: ChannelApi, op: string | null, parse: (v: unknown) => number | null) {
  const [entry, setEntry] = useState<{ op: string | null; value: number | null }>({ op: null, value: null })
  const parseRef = useRef(parse)
  useLayoutEffect(() => { parseRef.current = parse })
  useEffect(() => {
    if (!op) return
    let cancelled = false
    api.get(op).then((v) => { if (!cancelled) setEntry({ op, value: parseRef.current(v) }) }).catch(() => {})
    return () => { cancelled = true }
  }, [api, op])
  // after a set, read the setpoint back: the editor waits until the instrument reports it
  const commit = useCallback(async (v: number) => {
    if (!op) return
    await api.set(op, v)
    api.get(op).then((r) => setEntry({ op, value: parseRef.current(r) })).catch(() => {})
  }, [api, op])
  return [entry.op === op ? entry.value : null, commit] as const
}

/**
 * Run instrument requests that have no control of their own to show a failure (dial, chips):
 * the last error message is kept until the next request succeeds.
 */
export function useAction() {
  const [error, setError] = useState<string | null>(null)
  /** Resolves to whether the request succeeded (controls stop waiting for confirmation on false). */
  const run = useCallback((p: Promise<unknown>, onOk?: () => void): Promise<boolean> =>
    p.then(() => { setError(null); onOk?.(); return true })
      .catch((e) => { setError(e instanceof Error ? e.message : String(e)); return false }), [])
  return [error, run] as const
}
