/** Test doubles for the recordings client and the `/ws/recordings` socket. */
import { vi } from 'vitest'
import { ApiError, RecordingsClient } from '../client'
import type { Recording, Series, Signal } from '../types'

export const T0 = 1792234800000

export function recording(over: Partial<Recording> = {}): Recording {
  return {
    id: 1, name: 'PSU ramp', description: '', interval_s: 1, state: 'recording',
    started_at: T0, stopped_at: null, paused_at: null, samples: 0, last_t: null, series: [], ...over,
  }
}

export function series(over: Partial<Series> = {}): Series {
  return {
    id: 11, device_id: 'tenma-72-2540', class: 'PSU', channel: 1, field: 'VOUT', label: 'Tenma · VOUT', unit: 'V',
    added_at: T0 + 1000, removed_at: null, ...over,
  }
}

/** In-memory server: keeps recordings, answers like the v2 API. */
export function fakeClient(initial: Recording[] = []) {
  const db = initial.map((r) => structuredClone(r))
  let nextId = 100
  let nextSeries = 1000
  const find = (id: number) => {
    const r = db.find((x) => x.id === id)
    if (!r) throw new ApiError('Recording not found', 404)
    return r
  }
  const setState = (id: number, state: Recording['state']) => {
    const r = find(id)
    if (r.state !== 'stopped') r.state = state
    return Promise.resolve(structuredClone(r))
  }
  const client = {
    list: vi.fn(async () => structuredClone(db)),
    create: vi.fn(async (n: { name: string; description?: string; interval_s: number }) => {
      if (db.some((r) => r.name === n.name)) throw new ApiError(`Recording name '${n.name}' is taken`, 409)
      const r = recording({ id: nextId++, name: n.name, description: n.description ?? '', interval_s: n.interval_s, started_at: Date.now() })
      db.unshift(r)
      return structuredClone(r)
    }),
    update: vi.fn(async (id: number, p: { name?: string; description?: string }) => {
      const r = find(id)
      Object.assign(r, p)
      return structuredClone(r)
    }),
    addSeries: vi.fn(async (id: number, s: Signal) => {
      const r = find(id)
      if (r.state === 'stopped') throw new ApiError('Recording is stopped', 409)
      const x = series({ ...s, id: nextSeries++, added_at: Date.now() })
      r.series.push(x)
      return structuredClone(x)
    }),
    removeSeries: vi.fn(async (id: number, sid: number) => {
      const s = find(id).series.find((x) => x.id === sid)
      if (!s) throw new ApiError('Series not found', 404)
      s.removed_at = Date.now()
    }),
    pause: vi.fn((id: number) => setState(id, 'paused')),
    resume: vi.fn((id: number) => setState(id, 'recording')),
    stop: vi.fn((id: number) => setState(id, 'stopped')),
    remove: vi.fn(async (id: number) => { db.splice(db.indexOf(find(id)), 1) }),
    data: vi.fn(async (id: number) => ({ recording_id: id, from: null, to: null, series: [] })),
  }
  return client satisfies RecordingsClient
}

/** `window.WebSocket` replacement; `sockets` holds the open ones, `emit` sends a message. */
export class MockSocket {
  static sockets: MockSocket[] = []
  onopen: ((e: Event) => void) | null = null
  onclose: ((e: CloseEvent) => void) | null = null
  onmessage: ((e: MessageEvent) => void) | null = null
  onerror: ((e: Event) => void) | null = null
  constructor(readonly url: string) {
    MockSocket.sockets.push(this)
    setTimeout(() => this.onopen?.(new Event('open')), 0)
  }
  emit(msg: unknown) { this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(msg) })) }
  close() { MockSocket.sockets = MockSocket.sockets.filter((s) => s !== this) }
  send() {}
  static last(path = '/ws/recordings') { return [...MockSocket.sockets].reverse().find((s) => s.url.endsWith(path))! }
}
