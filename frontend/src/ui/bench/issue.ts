/**
 * A model's reported problem from the BenchMesh website (`issue` of `/catalog` and `/instruments`,
 * `community.rs` in bm-server): under investigation, or fixed in a version newer than this app.
 * Shown as a small mark on the instrument's panel and in Supported instruments.
 */
import type { CommunityIssue } from '../types'

/** "Issue reported - investigation pending" / "Fixed in 0.1.20 - update" / "Fixed in 0.1.20 - next release". */
export function issueLabel(issue: CommunityIssue): string {
  if (issue.state === 'investigating') return 'Issue reported - investigation pending'
  const v = issue.fixed_in.replace(/^v/, '')
  return issue.available === false ? `Fixed in ${v} - next release` : `Fixed in ${v} - update`
}

/** The longer explanation (tooltip). */
export function issueHint(issue: CommunityIssue): string {
  const n = issue.reports ?? 0
  const who = n > 0 ? `${n} user report${n === 1 ? '' : 's'}` : 'User reports'
  if (issue.state === 'investigating') {
    return `${who} of a problem with this model; the maintainers are looking into it. The instrument may still work for what you need.`
  }
  const v = issue.fixed_in.replace(/^v/, '')
  if (issue.available === false) return `A reported problem with this model is fixed in v${v}, which is not released yet.`
  return `A reported problem with this model is fixed in v${v}: update BenchMesh to get the fix.`
}
