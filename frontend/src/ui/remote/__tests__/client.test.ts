import { afterEach, describe, expect, it, vi } from 'vitest'
import { routeFetch } from '../../automation/__tests__/fakes'
import { asSession, asStatus, remoteClient } from '../client'
import { shortAgent, sinceText } from '../format'

afterEach(() => vi.restoreAllMocks())

const STATUS = {
  enabled: true, port: 57667, password_set: true, view_password_set: false, listening: true, error: null,
  urls: ['https://192.168.1.20:57667/'], fingerprint: 'AB:CD', sessions: [{ id: 's1', role: 'control', address: '192.168.1.30', agent: 'x', since: 1, last_seen: 2 }],
}

describe('remote client', () => {
  it('calls the contract routes', async () => {
    const calls = routeFetch({
      'GET /remote/session': () => ({ body: { principal: 'remote', role: 'view', address: '192.168.1.30', since: 5 } }),
      'GET /remote': () => ({ body: STATUS }),
      'PUT /remote': (b) => ({ body: { ...STATUS, enabled: b.enabled, sessions: [] } }),
      'PUT /remote/password': () => ({ status: 204 }),
      'PUT /remote/view-password': () => ({ status: 204 }),
      'DELETE /remote/view-password': () => ({ status: 204 }),
      'DELETE /remote/sessions/a%20b': () => ({ status: 204 }),
      'DELETE /remote/sessions': () => ({ status: 204 }),
    })
    expect(await remoteClient.session()).toEqual({ principal: 'remote', role: 'view', address: '192.168.1.30', since: 5 })
    expect(await remoteClient.status()).toEqual(STATUS)
    expect((await remoteClient.update({ enabled: false }))?.enabled).toBe(false)
    await remoteClient.setPassword('0123456789')
    await remoteClient.setViewPassword('9876543210')
    await remoteClient.removeViewPassword()
    await remoteClient.disconnect('a b')
    await remoteClient.disconnectAll()
    expect(calls.map((c) => `${c.method} ${c.path}`)).toEqual([
      'GET /remote/session', 'GET /remote', 'PUT /remote', 'PUT /remote/password', 'PUT /remote/view-password',
      'DELETE /remote/view-password', 'DELETE /remote/sessions/a%20b', 'DELETE /remote/sessions',
    ])
    expect(calls[2].body).toEqual({ enabled: false })
    expect(calls[3].body).toEqual({ password: '0123456789' })
    expect(calls[4].body).toEqual({ password: '9876543210' })
  })

  it('rejects with the server detail and status', async () => {
    routeFetch({ 'PUT /remote': () => ({ status: 409, body: { detail: 'set a password first' } }) })
    await expect(remoteClient.update({ enabled: true })).rejects.toMatchObject({ message: 'set a password first', status: 409 })
  })

  it('reads anything but a remote session as the local principal', () => {
    expect(asSession({ principal: 'local' })).toEqual({ principal: 'local' })
    expect(asSession([])).toEqual({ principal: 'local' })
    expect(asSession(null)).toEqual({ principal: 'local' })
    expect(asSession({ principal: 'remote' })).toEqual({ principal: 'remote', role: 'control', address: undefined, since: undefined })
    expect(asSession({ principal: 'remote', role: 'view' })?.role).toBe('view')
  })

  it('accepts only a status object and fills missing fields', () => {
    expect(asStatus([])).toBeNull()
    expect(asStatus({ detail: 'x' })).toBeNull()
    expect(asStatus({ enabled: false })).toEqual({
      enabled: false, port: 57667, password_set: false, view_password_set: false, listening: false, error: null, urls: [], fingerprint: null, sessions: [],
    })
  })
})

describe('session texts', () => {
  it('shortens user agents', () => {
    expect(shortAgent('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36')).toBe('Chrome on Windows')
    expect(shortAgent('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36 Edg/120.0')).toBe('Edge on Windows')
    expect(shortAgent('Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0')).toBe('Firefox on Linux')
    expect(shortAgent('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Version/17.0 Mobile/15E148 Safari/604.1')).toBe('Safari on iOS')
    expect(shortAgent('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 Version/17.0 Safari/605.1.15')).toBe('Safari on macOS')
    expect(shortAgent('curl/8.0')).toBe('curl/8.0')
    expect(shortAgent('x'.repeat(60))).toHaveLength(40)
    expect(shortAgent('')).toBe('Unknown browser')
  })

  it('shows the time for today and the date otherwise', () => {
    const now = new Date(2026, 9, 20, 15, 0).getTime()
    const today = new Date(2026, 9, 20, 9, 5).getTime()
    expect(sinceText(today, now)).not.toContain(new Date(today).toLocaleDateString())
    expect(sinceText(today - 86400000, now)).toContain(new Date(today - 86400000).toLocaleDateString())
  })
})
