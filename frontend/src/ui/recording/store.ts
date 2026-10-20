/** Pure state of the recordings client: the recording list and per-recording live sample buffers. */
import type { Recording, RecordingsMessage, Series } from './types'

/** Ticks received live for one recording: `t[i]` with each series' value at `v[seriesId][i]`. */
export interface LiveBuffer { t: number[]; v: Record<string, (number | null)[]> }

export interface RecordingsState {
  recordings: Recording[]
  live: Record<number, LiveBuffer>
  /** Bumped on every snapshot (connect / reconnect): views re-read history they may have missed. */
  epoch: number
}

export const EMPTY_STATE: RecordingsState = { recordings: [], live: {}, epoch: 0 }

/** Live ticks kept per recording (one hour at 1 s); older ones are read from `/data`. */
export const LIVE_CAPACITY = 3600

/** Newest first: by start time, then id. */
export const byNewest = (a: Recording, b: Recording) => b.started_at - a.started_at || b.id - a.id

export function isRecording(x: unknown): x is Recording {
  const r = x as Recording
  return !!r && typeof r === 'object' && typeof r.id === 'number' && typeof r.name === 'string' && Array.isArray(r.series)
}

export function upsert(list: Recording[], r: Recording): Recording[] {
  const i = list.findIndex((x) => x.id === r.id)
  const next = i < 0 ? [...list, r] : list.map((x) => (x.id === r.id ? r : x))
  return next.sort(byNewest)
}

/** Append one tick; series seen for the first time get nulls for earlier ticks. */
export function appendSamples(buf: LiveBuffer | undefined, t: number, values: Record<string, number | null>, capacity = LIVE_CAPACITY): LiveBuffer {
  const b = buf ?? { t: [], v: {} }
  // a repeated or out-of-order tick (reconnect) would break the time order: ignore it
  if (b.t.length && t <= b.t[b.t.length - 1]) return b
  const n = b.t.length
  const v: Record<string, (number | null)[]> = {}
  for (const k of new Set([...Object.keys(b.v), ...Object.keys(values)])) {
    const prev = b.v[k] ?? new Array(n).fill(null)
    const x = values[k]
    v[k] = [...prev, typeof x === 'number' && Number.isFinite(x) ? x : null]
  }
  let ts = [...b.t, t]
  // trim in chunks, not on every tick
  if (ts.length > capacity * 1.1) {
    const drop = ts.length - capacity
    ts = ts.slice(drop)
    for (const k of Object.keys(v)) v[k] = v[k].slice(drop)
  }
  return { t: ts, v }
}

/** Apply one `/ws/recordings` message. Unknown messages leave the state unchanged. */
export function applyMessage(s: RecordingsState, msg: RecordingsMessage | { type?: unknown }): RecordingsState {
  switch (msg?.type) {
    case 'snapshot': {
      const m = msg as Extract<RecordingsMessage, { type: 'snapshot' }>
      const recordings = (Array.isArray(m.recordings) ? m.recordings : []).filter(isRecording).sort(byNewest)
      const ids = new Set(recordings.map((r) => r.id))
      const live = Object.fromEntries(Object.entries(s.live).filter(([id]) => ids.has(Number(id))))
      return { recordings, live, epoch: s.epoch + 1 }
    }
    case 'recording': {
      const m = msg as Extract<RecordingsMessage, { type: 'recording' }>
      return isRecording(m.recording) ? { ...s, recordings: upsert(s.recordings, m.recording) } : s
    }
    case 'deleted': {
      const m = msg as Extract<RecordingsMessage, { type: 'deleted' }>
      const live = { ...s.live }
      delete live[m.recording_id]
      return { ...s, recordings: s.recordings.filter((r) => r.id !== m.recording_id), live }
    }
    case 'samples': {
      const m = msg as Extract<RecordingsMessage, { type: 'samples' }>
      if (typeof m.recording_id !== 'number' || typeof m.t !== 'number' || !m.values || typeof m.values !== 'object') return s
      const recordings = s.recordings.map((r) => (r.id === m.recording_id
        ? { ...r, last_t: m.t, samples: r.samples + Object.values(m.values).filter((x) => x != null).length }
        : r))
      return { ...s, recordings, live: { ...s.live, [m.recording_id]: appendSamples(s.live[m.recording_id], m.t, m.values) } }
    }
    default:
      return s
  }
}

/** A series added through the REST API, before the server's `recording` message arrives. */
export function withSeries(s: RecordingsState, recordingId: number, series: Series): RecordingsState {
  return {
    ...s,
    recordings: s.recordings.map((r) => (r.id === recordingId && !r.series.some((x) => x.id === series.id)
      ? { ...r, series: [...r.series, series] } : r)),
  }
}

/** A series removed through the REST API (its data is kept). */
export function withoutSeries(s: RecordingsState, recordingId: number, seriesId: number, now = Date.now()): RecordingsState {
  return {
    ...s,
    recordings: s.recordings.map((r) => (r.id === recordingId
      ? { ...r, series: r.series.map((x) => (x.id === seriesId && x.removed_at == null ? { ...x, removed_at: now } : x)) } : r)),
  }
}
