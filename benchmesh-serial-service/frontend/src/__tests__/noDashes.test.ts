import { execFileSync } from 'node:child_process'
import { describe, expect, it } from 'vitest'

// Plain text uses hyphens only, never em or en dashes (the UI, the definitions, docs and messages).
describe('no em or en dashes', () => {
  it('appears in any tracked file of the repository', () => {
    const args = ['grep', '--untracked', '-nE', '\u2013|\u2014', '--', ':/', ':/!**/package-lock.json', ':/!captures']
    let hits = ''
    try {
      hits = execFileSync('git', args, { encoding: 'utf8' })
    } catch {
      // git grep exits 1 when nothing matches
    }
    expect(hits).toBe('')
  })
})
