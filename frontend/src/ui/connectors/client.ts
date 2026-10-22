/**
 * Connectors: the virtual SCPI instruments (contract `ai_context/SCPI.md`, `GET|PUT /scpi`, this
 * computer only) and the code snippets that use them or the Python package. Pure model + REST.
 */
import { apiBaseUrl } from '../../api/base'
import { ApiError } from '../automation/client'

export type ScpiAccess = 'off' | 'read' | 'control'

export interface ScpiInstrument {
  id: string
  name: string
  port: number | null
  /** VISA resource name, e.g. `TCPIP0::127.0.0.1::5025::SOCKET` */
  address: string | null
  listening: boolean
  error: string | null
}

export interface ScpiStatus {
  access: ScpiAccess
  base_port: number
  host: string
  instruments: ScpiInstrument[]
}

const ACCESSES: ScpiAccess[] = ['off', 'read', 'control']

/** A `/scpi` reply with every field present, or null when it is not one (an older server). */
export function asScpiStatus(x: any): ScpiStatus | null {
  if (!x || typeof x !== 'object' || !ACCESSES.includes(x.access)) return null
  const str = (v: unknown) => (typeof v === 'string' && v ? v : null)
  return {
    access: x.access,
    base_port: typeof x.base_port === 'number' ? x.base_port : 5025,
    host: str(x.host) ?? '127.0.0.1',
    instruments: (Array.isArray(x.instruments) ? x.instruments : [])
      .filter((i: any) => i && typeof i.id === 'string')
      .map((i: any) => ({
        id: i.id,
        name: str(i.name) ?? i.id,
        port: typeof i.port === 'number' ? i.port : null,
        address: str(i.address),
        listening: i.listening === true,
        error: str(i.error),
      })),
  }
}

async function request(method: string, body?: unknown): Promise<ScpiStatus | null> {
  const r = await fetch(`${apiBaseUrl()}/scpi`, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  const j: any = await r.json().catch(() => null)
  if (!r.ok) throw new ApiError(typeof j?.detail === 'string' ? j.detail : `HTTP ${r.status}`, r.status, j)
  return asScpiStatus(j)
}

export const scpiClient = {
  status: () => request('GET'),
  update: (u: { access?: ScpiAccess; base_port?: number }) => request('PUT', u),
}

export type ScpiClient = typeof scpiClient

/** Why a base port cannot be saved, or null. */
export function basePortProblem(text: string): string | null {
  const n = Number(text)
  if (!/^\d+$/.test(text.trim()) || n < 1024 || n > 65000) return 'A port from 1024 to 65000.'
  return null
}

/** One line of state for an instrument's port. */
export function portState(s: ScpiStatus, i: ScpiInstrument): string {
  if (s.access === 'off') return 'Off'
  if (i.error) return i.error
  return i.listening ? 'Listening' : 'Starting...'
}

/** Ready-to-paste code for one instrument's virtual SCPI port. */
export function scpiSnippets(i: Pick<ScpiInstrument, 'address' | 'port'>, host = '127.0.0.1'): { title: string; text: string }[] {
  if (!i.address || i.port == null) return []
  return [
    {
      title: 'MATLAB (Instrument Control Toolbox)',
      text: `v = visadev("${i.address}");\nwriteline(v, "*IDN?"); disp(readline(v))\nwriteline(v, "FETCH:VOUT?"); vout = str2double(readline(v))`,
    },
    {
      title: 'MATLAB (no toolbox)',
      text: `t = tcpclient("${host}", ${i.port}); configureTerminator(t, "LF");\nwriteline(t, "*IDN?"); disp(readline(t))`,
    },
    {
      title: 'Python (PyVISA)',
      text: `import pyvisa\ninst = pyvisa.ResourceManager().open_resource("${i.address}", read_termination="\\n", write_termination="\\n")\nprint(inst.query("*IDN?"))`,
    },
    { title: 'LabVIEW, TestStand, other VISA programs', text: i.address },
  ]
}

export const PYTHON_SNIPPET = `pip install benchmesh

import benchmesh
bench = benchmesh.connect()
dmm = bench.instrument("my-dmm")
print(dmm.read("MEAS"))`
