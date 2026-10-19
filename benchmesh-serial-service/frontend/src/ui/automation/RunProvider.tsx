/**
 * The automation run as the whole UI sees it: one poller of `GET /automation/run` shared by
 * the top-bar indicator and the run panel. Polls every 500 ms while the run panel is open
 * and a run is active, 1.5 s while a run is active elsewhere, 5 s when idle. The log is
 * accumulated from `?after=<last_seq>`; a new run id starts a new log.
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import { automationClient, AutomationClient } from './client'
import { isActiveState, RunEvent, RunRequest, RunStatus } from './types'

export const POLL_FAST_MS = 500
export const POLL_ACTIVE_MS = 1500
export const POLL_IDLE_MS = 5000

export interface RunApi {
  status: RunStatus
  events: RunEvent[]
  active: boolean
  client: AutomationClient
  /** Start a run (rejects with the server's error: 409 busy / energizes, 422). */
  start(req: RunRequest): Promise<RunStatus>
  abort(): Promise<void>
  resume(): Promise<void>
  refresh(): Promise<void>
  /** The run panel is visible: poll fast while active. Returns the release function. */
  watch(): () => void
}

const Ctx = createContext<RunApi | null>(null)

export const useRun = () => useContext(Ctx)

const IDLE: RunStatus = { state: 'idle', events: [] }
const MAX_EVENTS = 5000

export function RunProvider({ children, client = automationClient }: { children: React.ReactNode; client?: AutomationClient }) {
  const [status, setStatus] = useState<RunStatus>(IDLE)
  const [events, setEvents] = useState<RunEvent[]>([])
  const [watchers, setWatchers] = useState(0)
  const known = useRef<{ id?: number; seq: number }>({ seq: 0 })
  const busy = useRef(false)
  /** Bumped by `start`: a poll begun before it must not overwrite the new run. */
  const gen = useRef(0)
  const mounted = useRef(true)
  useEffect(() => () => { mounted.current = false }, [])

  const apply = useCallback((s: RunStatus, fresh: boolean) => {
    const evs = s.events ?? []
    setEvents((cur) => {
      const base = fresh ? [] : cur
      const last = base.length ? base[base.length - 1].seq : 0
      const add = evs.filter((e) => e.seq > last)
      const all = add.length ? [...base, ...add] : base
      return all.length > MAX_EVENTS ? all.slice(all.length - MAX_EVENTS) : all
    })
    known.current = { id: s.id, seq: Math.max(s.last_seq ?? 0, evs.length ? evs[evs.length - 1].seq : 0) }
    setStatus(s)
  }, [])

  const refresh = useCallback(async () => {
    if (busy.current) return
    busy.current = true
    const g = gen.current
    try {
      const k = known.current
      let s = await client.runStatus(k.id != null ? k.seq : 0)
      let fresh = s.id !== k.id
      // another run since the last poll: its events from the start
      if (fresh && k.id != null && s.id != null && k.seq > 0) s = await client.runStatus(0)
      if (s.id == null) fresh = k.id != null
      if (mounted.current && g === gen.current) apply(s, fresh)
    } catch { /* keep the last known state; the next poll retries */ } finally { busy.current = false }
  }, [client, apply])

  const active = isActiveState(status.state)
  const interval = active ? (watchers > 0 ? POLL_FAST_MS : POLL_ACTIVE_MS) : POLL_IDLE_MS
  useEffect(() => {
    void refresh()
    const t = setInterval(() => void refresh(), interval)
    return () => clearInterval(t)
  }, [refresh, interval])

  const start = useCallback(async (req: RunRequest) => {
    const s = await client.run(req)
    gen.current++
    if (mounted.current) apply(s, true)
    return s
  }, [client, apply])

  const abort = useCallback(async () => { await client.abort(); await refresh() }, [client, refresh])
  const resume = useCallback(async () => { await client.resume(); await refresh() }, [client, refresh])
  const watch = useCallback(() => {
    setWatchers((n) => n + 1)
    return () => setWatchers((n) => n - 1)
  }, [])

  const api = useMemo<RunApi>(() => ({ status, events, active, client, start, abort, resume, refresh, watch }),
    [status, events, active, client, start, abort, resume, refresh, watch])
  return <Ctx.Provider value={api}>{children}</Ctx.Provider>
}

export const STATE_TEXT: Record<string, string> = {
  idle: 'Idle', running: 'Running', waiting: 'Waiting for you', cleanup: 'Cleanup', finished: 'Finished', failed: 'Failed', aborted: 'Aborted',
}

/** Label of the abort button: a press during cleanup also skips the rest of the cleanup. */
export const abortLabel = (state: string, short = false) => (state === 'cleanup' ? 'Stop cleanup too' : short ? 'Stop' : 'Abort')

/** Top-bar indicator: `▶ <flow> — <state>` and a red Stop, visible from anywhere while a run is active. */
export function RunIndicator({ onOpen }: { onOpen: () => void }) {
  const run = useRun()
  const [error, setError] = useState<string | null>(null)
  if (!run || !run.active) return null
  const { status } = run
  return (
    <div className="auto-indicator" role="status" aria-label="Automation run">
      <button type="button" className="auto-indicator__open" onClick={onOpen} title="Open the run in Automation">
        ▶ {status.flow?.name ?? 'Flow'} — {STATE_TEXT[status.state] ?? status.state}
      </button>
      <button type="button" className="bench-btn auto-btn--stop" onClick={() => { setError(null); run.abort().catch((e) => setError(e instanceof Error ? e.message : String(e))) }}
        aria-label={status.state === 'cleanup' ? 'Stop the cleanup too' : 'Stop the automation run'}>
        ■ {abortLabel(status.state, true)}
      </button>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
