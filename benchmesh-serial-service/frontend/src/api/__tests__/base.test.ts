import { afterEach, describe, expect, it } from 'vitest'
import { apiBaseUrl, wsUrl } from '../base'

describe('api base', () => {
  afterEach(() => {
    delete (window as { __BENCHMESH_API_BASE__?: string }).__BENCHMESH_API_BASE__
  })

  it('uses the page origin (UI served by the BenchMesh server)', () => {
    expect(apiBaseUrl()).toBe(window.location.origin)
    expect(apiBaseUrl()).not.toContain('57666')
  })

  it('derives WebSocket URLs from the same origin', () => {
    const ws = wsUrl('/ws/registry')
    expect(ws.startsWith('ws')).toBe(true)
    expect(ws).toBe(window.location.origin.replace(/^http/, 'ws') + '/ws/registry')
  })

  it('honours an explicit override without trailing slash', () => {
    ;(window as { __BENCHMESH_API_BASE__?: string }).__BENCHMESH_API_BASE__ = 'https://lab.example:8443/'
    expect(apiBaseUrl()).toBe('https://lab.example:8443')
    expect(wsUrl('/ws/registry')).toBe('wss://lab.example:8443/ws/registry')
  })
})
