import { describe, it, expect } from 'vitest'
import { issueCard, issueLabel, issueTone } from '../issue'

describe('reported problems', () => {
  it('the mark is short: never longer than "Issue reported"', () => {
    const all = [
      { state: 'investigating', reports: 1 },
      { state: 'fixed', fixed_in: 'v0.1.20', available: true },
      { state: 'fixed', fixed_in: '0.1.20', available: null },
      { state: 'fixed', fixed_in: '0.1.200', available: false },
    ] as const
    expect(all.map(issueLabel)).toEqual(['Issue reported', 'Fix available', 'Fix available', 'Fix coming'])
    for (const i of all) expect(issueLabel(i).length).toBeLessThanOrEqual('Issue reported'.length)
    expect(all.map(issueTone)).toEqual(['problem', 'fix', 'fix', 'coming'])
  })

  it('the card says the state, who and when, and what to do', () => {
    const inv = issueCard({ state: 'investigating', reports: 3, since: '2026-10-12T09:00:00Z' })
    expect(inv.heading).toBe('Issue reported - investigation pending')
    expect(inv.lines.join(' ')).toContain('3 users reported it since 2026-10-12.')
    expect(issueCard({ state: 'investigating', reports: 1, since: '2026-10-12' }).lines[0]).toContain('One user reported it on 2026-10-12.')
    expect(issueCard({ state: 'investigating' }).lines[0]).toContain('Users reported it.')
    const fix = issueCard({ state: 'fixed', fixed_in: 'v0.1.20', available: true, reports: 2 })
    expect(fix.heading).toBe('Fixed in 0.1.20 - update to get it')
    expect(fix.lines.at(-1)).toBe('Update BenchMesh to 0.1.20 or newer to get the fix.')
    const coming = issueCard({ state: 'fixed', fixed_in: '0.1.20', available: false })
    expect(coming.heading).toBe('Fixed in 0.1.20 - not released yet')
    expect(coming.lines.join(' ')).toMatch(/next release/)
  })
})
