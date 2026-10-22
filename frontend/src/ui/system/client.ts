/**
 * REST client of Instruments - System (contract `ai_context/HEADLESS.md`): the computer's
 * resources, and for a headless BenchMesh restart, shut down and start with the computer.
 * Errors reject with `ApiError` carrying the server's `detail` and status.
 */
import { apiBaseUrl } from '../../api/base'
import { ApiError } from '../automation/client'

/** `GET /system` (sizes in bytes, times in ms since epoch). */
export interface SystemInfo {
  host: string | null
  os: string | null
  arch: string
  cpus: number
  uptime_s: number
  /** 1, 5 and 15 minute load averages; null on Windows */
  load: [number, number, number] | null
  cpu_percent: number
  memory: { total: number; used: number }
  process: { pid: number; memory: number | null; cpu_percent: number | null; started_ms: number }
  /** the disk of the data directory */
  disk: { path: string; total: number; available: number } | null
  temperature_c: number | null
  /** no app window: restart, shut down and start with the computer are available */
  headless: boolean
  version: string
}

/** `GET /system/autostart`. */
export interface Autostart {
  supported: boolean
  enabled: boolean
  method: string | null
  location: string | null
  starts_at: 'boot' | 'login' | null
  /** Linux: run once so it starts at power-on without a login */
  boot_command: string | null
  note: string | null
  /** why it is not available */
  reason: string | null
  command: string | null
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
    } catch { /* not JSON */ }
    throw new ApiError(detail, r.status)
  }
  return r.json() as Promise<T>
}

export const getSystem = () => request<SystemInfo>('GET', '/system')
export const restart = () => request<{ action: string }>('POST', '/system/restart')
export const shutdown = () => request<{ action: string }>('POST', '/system/shutdown')
export const getAutostart = () => request<Autostart>('GET', '/system/autostart')
export const setAutostart = (enabled: boolean) => request<Autostart>('PUT', '/system/autostart', { enabled })
