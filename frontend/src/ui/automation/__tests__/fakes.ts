/** Test doubles for the automation API: a vocabulary like the simulated bench, and a fetch router. */
import { vi } from 'vitest'
import type { FlowDoc, Vocab } from '../types'

export const vocab: Vocab = {
  devices: [
    {
      id: 'psu', name: 'Tenma', vendor: 'TENMA', model: '72-2540', online: true,
      classes: {
        PSU: {
          channels: 1,
          fields: { VOUT: { type: 'si', description: 'Measured output voltage (V)' }, IOUT: { type: 'si' }, OUTPUT: { type: 'enum', values: ['ON', 'OFF'] } },
          operations: {
            voltage: { get: true, set: true, value: { type: 'number', min: 0, max: 60, format: '%.2f', unit: 'V' }, description: 'Voltage setpoint (V)' },
            current: { get: true, set: true, value: { type: 'number', min: 0, max: 15, format: '%.3f', unit: 'A' } },
            output: { get: false, set: true, energizes: true, value: { type: 'enum', values: ['OFF', 'ON'] } },
            output_voltage: { get: true, set: false },
          },
        },
      },
    },
    {
      id: 'awg', name: 'DGE', vendor: 'OWON', online: false,
      classes: {
        AWG: {
          channels: 2,
          fields: { FREQ: { type: 'si' } },
          operations: { frequency: { get: true, set: true, value: { type: 'number', min: 0, max: 7e7, unit: 'Hz' } } },
        },
      },
    },
  ],
}

export const flowDoc = (over: Partial<FlowDoc> = {}): FlowDoc => ({
  schema: 'benchmesh.flow/v1', name: 'Ramp', description: '',
  params: [{ name: 'target', value: 5, unit: 'V', min: 0, max: 30 }],
  setup: [],
  steps: [
    { do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'voltage', expr: 'target' },
    { do: 'read', device: 'psu', class: 'PSU', channel: 1, field: 'VOUT', into: 'v' },
  ],
  cleanup: [{ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'output', value: 'OFF' }],
  ...over,
})

export interface Call { method: string; path: string; body: any }
type Handler = (body: any, path: string) => { status?: number; body?: unknown } | undefined

export const json = (body: unknown, status = 200) =>
  Promise.resolve(status === 204 ? new Response(null, { status }) : new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

/**
 * Routes `fetch` to handlers keyed `"METHOD /path"` (query string kept in `path`, matched without it).
 * Unknown routes answer like an idle server. Returns the recorded calls.
 */
export function routeFetch(routes: Record<string, Handler>) {
  const calls: Call[] = []
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url, init) => {
    const u = new URL(String(url), 'http://localhost')
    const method = (init?.method ?? 'GET').toUpperCase()
    const body = init?.body ? JSON.parse(String(init.body)) : undefined
    const path = u.pathname + u.search
    calls.push({ method, path, body })
    const h = routes[`${method} ${u.pathname}`]
    const r = h?.(body, path)
    if (r) return json(r.body ?? null, r.status ?? 200)
    if (u.pathname === '/automation/run' && method === 'GET') return json({ state: 'idle', events: [] })
    if (u.pathname === '/automation/flows' && method === 'GET') return json([])
    if (u.pathname === '/automation/vocabulary') return json(vocab)
    if (u.pathname === '/automation/validate') return json({ ok: true, issues: [], energizes: [] })
    if (u.pathname === '/automation/assistant') return json({ provider: 'anthropic', model: 'claude-x', base_url: '', keys: { anthropic: null, openai: null, openrouter: null, custom: null } })
    return json({ detail: `no route ${method} ${u.pathname}` }, 404)
  })
  return calls
}
