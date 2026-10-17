/**
 * Recordings for every window: the list (REST + `/ws/recordings` live updates), the live sample
 * buffers of running recordings, and the actions. Actions apply the server's reply at once; the
 * WebSocket messages that follow (to every window) carry the same state.
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useReducer, useState } from 'react'
import { wsUrl } from '../../api/base'
import { RecordingsClient, recordingsClient } from './client'
import { applyMessage, EMPTY_STATE, isRecording, RecordingsState, upsert, withoutSeries, withSeries } from './store'
import { NewRecording, Recording, RecordingsMessage, sameSignal, Series, Signal } from './types'

type Action =
  | { kind: 'message'; msg: RecordingsMessage }
  | { kind: 'list'; recordings: Recording[] }
  | { kind: 'upsert'; recording: Recording }
  | { kind: 'series'; recordingId: number; series: Series }
  | { kind: 'unseries'; recordingId: number; seriesId: number }
  | { kind: 'deleted'; id: number }

function reducer(s: RecordingsState, a: Action): RecordingsState {
  switch (a.kind) {
    case 'message': return applyMessage(s, a.msg)
    // the REST list only fills in until the WebSocket snapshot arrives
    case 'list': return s.epoch > 0 ? s : { ...s, recordings: a.recordings.filter(isRecording).reduce(upsert, [] as Recording[]) }
    case 'upsert': return { ...s, recordings: upsert(s.recordings, a.recording) }
    case 'series': return withSeries(s, a.recordingId, a.series)
    case 'unseries': return withoutSeries(s, a.recordingId, a.seriesId)
    case 'deleted': return applyMessage(s, { type: 'deleted', recording_id: a.id })
  }
}

export interface RecordingsApi extends RecordingsState {
  connected: boolean
  client: RecordingsClient
  create: (r: NewRecording) => Promise<Recording>
  update: (id: number, patch: { name?: string; description?: string }) => Promise<Recording>
  pause: (id: number) => Promise<Recording>
  resume: (id: number) => Promise<Recording>
  stop: (id: number) => Promise<Recording>
  remove: (id: number) => Promise<void>
  addSeries: (id: number, s: Signal) => Promise<Series>
  removeSeries: (id: number, seriesId: number) => Promise<void>
}

const Ctx = createContext<RecordingsApi | null>(null)

/** The recordings context, or null outside a provider (readings are then not recordable). */
export function useRecordings(): RecordingsApi | null {
  return useContext(Ctx)
}

/** Active series of `signal` per recording id. */
export function seriesOf(recordings: Recording[], signal: Pick<Signal, 'device_id' | 'class' | 'channel' | 'field'>): Map<number, Series> {
  const out = new Map<number, Series>()
  for (const r of recordings) {
    const s = r.series.find((x) => x.removed_at == null && sameSignal(x, signal))
    if (s) out.set(r.id, s)
  }
  return out
}

export function RecordingsProvider({ children, client = recordingsClient }: { children: React.ReactNode; client?: RecordingsClient }) {
  const [state, dispatch] = useReducer(reducer, EMPTY_STATE)
  const [connected, setConnected] = useState(false)

  useEffect(() => {
    let cancelled = false
    client.list().then((l) => { if (!cancelled && Array.isArray(l)) dispatch({ kind: 'list', recordings: l }) }).catch(() => {})
    return () => { cancelled = true }
  }, [client])

  useEffect(() => {
    let ws: WebSocket | null = null
    let retry: ReturnType<typeof setTimeout> | undefined
    let closed = false
    const connect = () => {
      try {
        ws = new WebSocket(wsUrl('/ws/recordings'))
      } catch {
        retry = setTimeout(connect, 2000)
        return
      }
      ws.onopen = () => setConnected(true)
      ws.onclose = () => {
        setConnected(false)
        if (!closed) retry = setTimeout(connect, 2000)
      }
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(typeof ev.data === 'string' ? ev.data : '')
          if (msg && typeof msg === 'object' && typeof msg.type === 'string') dispatch({ kind: 'message', msg })
        } catch { /* not a recordings message */ }
      }
    }
    connect()
    return () => {
      closed = true
      if (retry) clearTimeout(retry)
      if (ws) { ws.onclose = null; ws.close() }
    }
  }, [])

  const withRecording = useCallback((p: Promise<Recording>) => p.then((r) => { dispatch({ kind: 'upsert', recording: r }); return r }), [])
  const create = useCallback((r: NewRecording) => withRecording(client.create(r)), [client, withRecording])
  const update = useCallback((id: number, patch: { name?: string; description?: string }) => withRecording(client.update(id, patch)), [client, withRecording])
  const pause = useCallback((id: number) => withRecording(client.pause(id)), [client, withRecording])
  const resume = useCallback((id: number) => withRecording(client.resume(id)), [client, withRecording])
  const stop = useCallback((id: number) => withRecording(client.stop(id)), [client, withRecording])
  const remove = useCallback((id: number) => client.remove(id).then(() => dispatch({ kind: 'deleted', id })), [client])
  const addSeries = useCallback((id: number, s: Signal) => client.addSeries(id, s).then((series) => {
    dispatch({ kind: 'series', recordingId: id, series })
    return series
  }), [client])
  const removeSeries = useCallback((id: number, seriesId: number) => client.removeSeries(id, seriesId)
    .then(() => dispatch({ kind: 'unseries', recordingId: id, seriesId })), [client])

  const value = useMemo<RecordingsApi>(() => ({
    ...state, connected, client, create, update, pause, resume, stop, remove, addSeries, removeSeries,
  }), [state, connected, client, create, update, pause, resume, stop, remove, addSeries, removeSeries])
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}
