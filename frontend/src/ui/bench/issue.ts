/**
 * A model's reported problem from the BenchMesh website (`issue` of `/catalog` and `/instruments`,
 * `community.rs` in bm-server): under investigation, or fixed in a version newer than this app.
 * Shown as a short mark (never longer than "Issue reported") on the instrument's panel and in
 * Supported instruments; hovering, focusing or pressing it shows the card (`IssueMark.tsx`).
 */
import type { CommunityIssue } from '../types'

/** The tone of the mark: a problem (red), a fix to install (accent), a fix not released yet (muted). */
export type IssueTone = 'problem' | 'fix' | 'coming'

const version = (v: string) => v.replace(/^v/, '')

/** The mark: "Issue reported" / "Fix available" / "Fix coming". */
export function issueLabel(issue: CommunityIssue): string {
  if (issue.state === 'investigating') return 'Issue reported'
  return issue.available === false ? 'Fix coming' : 'Fix available'
}

export function issueTone(issue: CommunityIssue): IssueTone {
  if (issue.state === 'investigating') return 'problem'
  return issue.available === false ? 'coming' : 'fix'
}

/** "3 users reported it since 2026-10-12." */
function reported(issue: CommunityIssue): string {
  const n = issue.reports ?? 0
  const day = issue.since?.slice(0, 10)
  if (n === 1) return `One user reported it${day ? ` on ${day}` : ''}.`
  return `${n > 1 ? `${n} users` : 'Users'} reported it${day ? ` since ${day}` : ''}.`
}

/** What the card says: its heading (the state) and a few lines (who, when, what to do). */
export function issueCard(issue: CommunityIssue): { heading: string; lines: string[] } {
  if (issue.state === 'investigating') {
    return {
      heading: 'Issue reported - investigation pending',
      lines: [`A problem with this model. ${reported(issue)}`, 'The maintainers are looking into it. The instrument may still work for what you need.'],
    }
  }
  const v = version(issue.fixed_in)
  if (issue.available === false) {
    return {
      heading: `Fixed in ${v} - not released yet`,
      lines: [`A reported problem with this model is fixed in BenchMesh ${v}. ${reported(issue)}`, 'The fix comes with the next release; until then the instrument may still work for what you need.'],
    }
  }
  return {
    heading: `Fixed in ${v} - update to get it`,
    lines: [`A reported problem with this model is fixed in BenchMesh ${v}. ${reported(issue)}`, `Update BenchMesh to ${v} or newer to get the fix.`],
  }
}
