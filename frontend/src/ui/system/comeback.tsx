/**
 * BenchMesh stops and (usually) starts again: Restart / Shut down on Instruments - System, or a
 * distribution's group that hands the process over (an update install, `GroupProps.onStopping`).
 * The page then says what is happening and reloads once a new BenchMesh answers (a remote browser
 * signs in again).
 */
import { useCallback, useEffect, useRef, useState } from 'react'
import { ApiError } from '../automation/client'
import * as api from './client'
import type { Stopping } from '../../extensions/contract'

const COMEBACK_MS = 90_000

export type Phase = 'running' | 'restarting' | 'handed-over' | 'late' | 'stopped'

export interface Comeback {
  phase: Phase
  stopping: Stopping | null
  /** this process started at (ms): a different one answering means BenchMesh came back */
  setStarted: (ms: number | null) => void
  /** a restart was asked for */
  restarting: () => void
  /** shut down: nothing comes back */
  stopped: () => void
  /** a group stopped BenchMesh (`GroupProps.onStopping`) */
  onStopping: (what: Stopping) => void
}

export function useComeback(): Comeback {
  const [phase, setPhase] = useState<Phase>('running')
  const [stopping, setStopping] = useState<Stopping | null>(null)
  const started = useRef<number | null>(null)

  // which BenchMesh answers now (a page that does not poll /system itself)
  useEffect(() => {
    api.getSystem().then((s) => { started.current ??= s.process.started_ms }, () => { /* asked again while waiting */ })
  }, [])

  useEffect(() => {
    if (phase !== 'restarting' && phase !== 'handed-over') return
    const since = Date.now()
    const t = setInterval(async () => {
      try {
        const s = await api.getSystem()
        if (s.process.started_ms !== started.current) window.location.reload()
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) window.location.reload()
        else if (Date.now() - since > COMEBACK_MS) setPhase('late')
      }
    }, 1000)
    return () => clearInterval(t)
  }, [phase])

  const setStarted = useCallback((ms: number | null) => { started.current = ms }, [])
  const restarting = useCallback(() => setPhase('restarting'), [])
  const stopped = useCallback(() => setPhase('stopped'), [])
  const onStopping = useCallback((what: Stopping) => { setStopping(what); setPhase('handed-over') }, [])
  return { phase, stopping, setStarted, restarting, stopped, onStopping }
}

/** What the page shows instead of its content while BenchMesh is away. */
export function ComebackNotice({ phase, stopping }: Pick<Comeback, 'phase' | 'stopping'>) {
  return (
    <div className="bench-group" role="status">
      {phase === 'restarting' && <><span className="kit-label">Restarting…</span><p className="bench-hint">Waiting for BenchMesh to answer again; this page reloads by itself.</p></>}
      {phase === 'handed-over' && stopping && <><span className="kit-label">{stopping.title}</span><p className="bench-hint">{stopping.hint} This page reloads by itself.</p></>}
      {phase === 'late' && <><span className="kit-label">BenchMesh did not come back</span><p className="bench-hint">It has not answered for 90 s: check the computer it runs on.</p></>}
      {phase === 'stopped' && <><span className="kit-label">BenchMesh is stopped</span><p className="bench-hint">Start it again on the computer it runs on.</p></>}
    </div>
  )
}
