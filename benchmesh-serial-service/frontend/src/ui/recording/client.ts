/** REST client of the recordings API (v2). Errors reject with `ApiError` carrying the server's `detail`. */
import { apiBaseUrl } from '../../api/base'
import type { NewRecording, Recording, RecordingData, Series, Signal } from './types'

export class ApiError extends Error {
  constructor(message: string, readonly status: number) { super(message) }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBaseUrl()}${path}`, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!r.ok) {
    let detail = `HTTP ${r.status}`
    try {
      const j = await r.json()
      if (typeof j?.detail === 'string') detail = j.detail
      // validation errors (422) may carry a list of problems
      else if (Array.isArray(j?.detail)) detail = j.detail.map((d: any) => d?.msg ?? String(d)).join('; ')
    } catch { /* not JSON */ }
    throw new ApiError(detail, r.status)
  }
  if (r.status === 204) return undefined as T
  return r.json() as Promise<T>
}

export interface DataQuery { from?: number; to?: number; max_points?: number }

export interface RecordingsClient {
  list(): Promise<Recording[]>
  create(r: NewRecording): Promise<Recording>
  update(id: number, patch: { name?: string; description?: string }): Promise<Recording>
  addSeries(id: number, s: Signal): Promise<Series>
  removeSeries(id: number, seriesId: number): Promise<void>
  pause(id: number): Promise<Recording>
  resume(id: number): Promise<Recording>
  stop(id: number): Promise<Recording>
  remove(id: number): Promise<void>
  data(id: number, q?: DataQuery): Promise<RecordingData>
}

export const recordingsClient: RecordingsClient = {
  list: () => request('GET', '/recordings'),
  create: (r) => request('POST', '/recordings', r),
  update: (id, patch) => request('PATCH', `/recordings/${id}`, patch),
  addSeries: (id, s) => request('POST', `/recordings/${id}/series`, s),
  removeSeries: (id, sid) => request('DELETE', `/recordings/${id}/series/${sid}`),
  pause: (id) => request('POST', `/recordings/${id}/pause`),
  resume: (id) => request('POST', `/recordings/${id}/resume`),
  stop: (id) => request('POST', `/recordings/${id}/stop`),
  remove: (id) => request('DELETE', `/recordings/${id}`),
  data: (id, q = {}) => {
    const p = new URLSearchParams()
    if (q.from != null) p.set('from', String(Math.floor(q.from)))
    if (q.to != null) p.set('to', String(Math.ceil(q.to)))
    if (q.max_points != null) p.set('max_points', String(q.max_points))
    const qs = p.toString()
    return request('GET', `/recordings/${id}/data${qs ? `?${qs}` : ''}`)
  },
}

/** CSV download of a whole recording. */
export const exportUrl = (id: number) => `${apiBaseUrl()}/recordings/${id}/export`

/** Client-side check of a new recording (the server checks again). Returns field → message. */
export function validateNew(name: string, interval: string): { name?: string; interval?: string } {
  const errors: { name?: string; interval?: string } = {}
  const n = name.trim()
  if (!n) errors.name = 'Name is required'
  else if (n.length > 100) errors.name = 'At most 100 characters'
  const s = interval.trim()
  if (!/^\d+$/.test(s)) errors.interval = 'Whole seconds, 1 or more'
  else if (Number(s) < 1 || Number(s) > 3600) errors.interval = 'Between 1 and 3600 s'
  return errors
}
