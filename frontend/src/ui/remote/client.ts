/**
 * REST client of remote access (contract `ai_context/REMOTE_ACCESS.md`). `GET /remote/session`
 * answers on both listeners; the other calls are `admin` (local principal only, 403 remotely).
 * Errors reject with `ApiError` carrying the server's `detail` and status.
 */
import { apiBaseUrl } from '../../api/base'
import { ApiError } from '../automation/client'

export type Principal = 'local' | 'remote'
/** What a remote session's password allows: `view` sessions see the bench and change nothing. */
export type Role = 'control' | 'view'

/** `GET /remote/session`: who this UI is. */
export interface SessionInfo {
  principal: Principal
  /** remote only: which password signed it in */
  role?: Role
  /** remote only: the address this browser connects from */
  address?: string
  /** remote only: sign-in time, ms since epoch */
  since?: number
}

/** One signed-in remote browser (`id` is not the token). Times in ms since epoch. */
export interface RemoteSession {
  id: string | number
  role: Role
  address: string
  agent: string
  since: number
  last_seen: number
}

/** `GET /remote`: settings and state of the network listener. */
export interface RemoteStatus {
  enabled: boolean
  port: number
  password_set: boolean
  view_password_set: boolean
  listening: boolean
  error: string | null
  urls: string[]
  fingerprint: string | null
  sessions: RemoteSession[]
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBaseUrl()}${path}`, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!r.ok) {
    let detail = `HTTP ${r.status}`
    let j: any = null
    try {
      j = await r.json()
      if (typeof j?.detail === 'string') detail = j.detail
    } catch { /* not JSON */ }
    throw new ApiError(detail, r.status, j)
  }
  if (r.status === 204) return undefined as T
  return r.json() as Promise<T>
}

/** A `/remote/session` reply as the UI uses it; anything else (e.g. an older server) is the local principal. */
export function asSession(x: any): SessionInfo {
  if (x?.principal === 'remote') {
    return { principal: 'remote', role: x.role === 'view' ? 'view' : 'control', address: typeof x.address === 'string' ? x.address : undefined, since: typeof x.since === 'number' ? x.since : undefined }
  }
  return { principal: 'local' }
}

/** A `/remote` reply with every field present, or null when it is not one (feature absent). */
export function asStatus(x: any): RemoteStatus | null {
  if (!x || typeof x !== 'object' || Array.isArray(x) || typeof x.enabled !== 'boolean') return null
  return {
    enabled: x.enabled,
    port: typeof x.port === 'number' ? x.port : 57667,
    password_set: x.password_set === true,
    view_password_set: x.view_password_set === true,
    listening: x.listening === true,
    error: typeof x.error === 'string' && x.error ? x.error : null,
    urls: Array.isArray(x.urls) ? x.urls.filter((u: unknown) => typeof u === 'string') : [],
    fingerprint: typeof x.fingerprint === 'string' && x.fingerprint ? x.fingerprint : null,
    sessions: Array.isArray(x.sessions)
      ? x.sessions.filter((s: any) => s && s.id != null).map((s: any) => ({ ...s, role: s.role === 'view' ? 'view' : 'control' }))
      : [],
  }
}

export const remoteClient = {
  session: () => request<unknown>('GET', '/remote/session').then(asSession),
  status: () => request<unknown>('GET', '/remote').then(asStatus),
  update: (u: { enabled: boolean; port?: number }) => request<unknown>('PUT', '/remote', u).then(asStatus),
  setPassword: (password: string) => request<void>('PUT', '/remote/password', { password }),
  setViewPassword: (password: string) => request<void>('PUT', '/remote/view-password', { password }),
  removeViewPassword: () => request<void>('DELETE', '/remote/view-password'),
  disconnect: (id: RemoteSession['id']) => request<void>('DELETE', `/remote/sessions/${encodeURIComponent(String(id))}`),
  disconnectAll: () => request<void>('DELETE', '/remote/sessions'),
}

export type RemoteClient = typeof remoteClient

/** Where a remote browser signs in (the server's own page, not part of this UI). */
export const loginUrl = () => `${apiBaseUrl()}/remote/login`

/** Where the log-out form posts: the server ends the session and answers 303 to the sign-in page. */
export const logoutUrl = () => `${apiBaseUrl()}/remote/logout`

export const MIN_PASSWORD = 10
