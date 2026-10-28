import { describe, it, expect } from 'vitest'
import { issueHint, issueLabel } from '../issue'

describe('reported problems', () => {
  it('say whether a problem is investigated, fixed in a release to update to, or fixed in the next one', () => {
    expect(issueLabel({ state: 'investigating', reports: 1 })).toBe('Issue reported - investigation pending')
    expect(issueLabel({ state: 'fixed', fixed_in: 'v0.1.20', available: true })).toBe('Fixed in 0.1.20 - update')
    expect(issueLabel({ state: 'fixed', fixed_in: '0.1.20', available: null })).toBe('Fixed in 0.1.20 - update')
    expect(issueLabel({ state: 'fixed', fixed_in: '0.1.20', available: false })).toBe('Fixed in 0.1.20 - next release')
  })
  it('explain it in the tooltip', () => {
    expect(issueHint({ state: 'investigating', reports: 1 })).toMatch(/^1 user report of a problem/)
    expect(issueHint({ state: 'investigating' })).toMatch(/^User reports/)
    expect(issueHint({ state: 'fixed', fixed_in: '0.1.20', available: true })).toMatch(/fixed in v0.1.20: update BenchMesh/)
    expect(issueHint({ state: 'fixed', fixed_in: '0.1.20', available: false })).toMatch(/not released yet/)
  })
})
