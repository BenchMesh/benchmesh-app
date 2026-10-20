/** A remote access server in memory, with the contract's rules (409 without a password, min 10 characters). */
import { vi } from 'vitest'
import { ApiError } from '../../automation/client'
import type { RemoteClient, RemoteStatus, SessionInfo } from '../client'

export const SESSIONS = [
  { id: 's1', address: '192.168.1.30', agent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0 Safari/537.36', since: Date.now() - 60000, last_seen: Date.now() },
  { id: 's2', address: '192.168.1.31', agent: 'Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0', since: Date.now() - 30000, last_seen: Date.now() },
]

export function fakeRemote(over: Partial<RemoteStatus> = {}, session: SessionInfo = { principal: 'local' }) {
  const state: RemoteStatus = {
    enabled: false, port: 57667, password_set: false, listening: false, error: null, urls: [], fingerprint: null, sessions: [], ...over,
  }
  const listen = () => {
    state.listening = state.enabled
    state.urls = state.enabled ? ['https://192.168.1.20:57667/'] : []
    state.fingerprint = state.enabled ? 'AB:CD:EF:01' : null
  }
  if (state.enabled) listen()
  const copy = () => structuredClone(state)
  const client: RemoteClient = {
    session: vi.fn(async () => session),
    status: vi.fn(async () => copy()),
    update: vi.fn(async ({ enabled }: { enabled: boolean }) => {
      if (enabled && !state.password_set) throw new ApiError('a password is needed', 409)
      state.enabled = enabled
      if (!enabled) state.sessions = []
      listen()
      return copy()
    }),
    setPassword: vi.fn(async (p: string) => {
      if (p.length < 10) throw new ApiError('too short', 400)
      state.password_set = true
      state.sessions = []
    }),
    disconnect: vi.fn(async (id) => { state.sessions = state.sessions.filter((s) => s.id !== id) }),
    disconnectAll: vi.fn(async () => { state.sessions = [] }),
  }
  return { state, client }
}
