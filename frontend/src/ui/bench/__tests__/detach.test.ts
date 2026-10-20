import { describe, it, expect, beforeEach } from 'vitest'
import { markDetached, clearDetached, detachedKeys, DETACHED_KEY, STALE_MS } from '../detach'

beforeEach(() => localStorage.clear())

describe('detached panels', () => {
  it('a detached window keeps its panel off the bench while its heartbeat is fresh', () => {
    markDetached('tenma:PSU', 1000)
    expect(detachedKeys(1000 + STALE_MS - 1)).toEqual(['tenma:PSU'])
    expect(detachedKeys(1000 + STALE_MS + 1)).toEqual([])
  })
  it('closing the window returns the panel', () => {
    markDetached('tenma:PSU', 1000)
    markDetached('dge:AWG', 1000)
    clearDetached('tenma:PSU')
    expect(detachedKeys(1500)).toEqual(['dge:AWG'])
  })
  it('ignores corrupt storage', () => {
    localStorage.setItem(DETACHED_KEY, '{nope')
    expect(detachedKeys(0)).toEqual([])
    markDetached('x:OSC', 5)
    expect(detachedKeys(5)).toEqual(['x:OSC'])
  })
})
