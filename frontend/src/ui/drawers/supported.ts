/**
 * Supported instruments (`GET /catalog`): every family the device definitions describe and each
 * model's verification status - the pure logic behind the "Supported" tab.
 */
import type { CommunityIssue, Support } from '../types'

/** What users reported on the website (`community.rs` in bm-server; schema benchmesh.status/v1). */
export interface CommunityEntry { status: 'confirmed' | 'issues'; versions?: string[]; reports?: number; last?: string }
export interface CatalogModel { name: string; support: Support; note?: string | null; community?: CommunityEntry | null; issue?: CommunityIssue | null }
export interface CatalogFamily {
  driver: string
  vendor: string
  family: string
  classes: string[]
  transports: string[]
  models: CatalogModel[]
  manuals?: string[]
}
/** Where the community status came from at app start (all null when it was not fetched yet / disabled). */
export interface CommunitySource {
  source: string | null; fetched_at: string | null; error: string | null; models: number
  /** Models with a reported problem; `generated` of the status this build carries. */
  issues?: number; snapshot?: string | null
}
export interface Catalog { families: CatalogFamily[]; models: Partial<Record<Support, number>>; community?: CommunitySource }

export const SUPPORT_LABEL: Record<Support, string> = {
  tested: 'Bench-tested',
  confirmed: 'Community-confirmed',
  awaiting: 'Awaiting confirmation',
}

export const SUPPORT_HINT: Record<Support, string> = {
  tested: 'Verified on the maintainers’ bench',
  confirmed: 'A user ran the check on the real instrument and it passed',
  awaiting: 'Written from the manufacturer’s manual; not yet confirmed on a real instrument - help verify it',
}

export const TRANSPORT_LABEL: Record<string, string> = { serial: 'Serial', usbtmc: 'USB TMC', tcpip: 'LAN', vxi11: 'LAN (VXI-11)' }

export interface CatalogFilter { text: string; cls: string | null; support: Support | null }

/** Families matching the filter, each with only the models that match; vendor/family order. */
export function filterCatalog(families: CatalogFamily[], f: CatalogFilter): CatalogFamily[] {
  const words = f.text.toLowerCase().split(/\s+/).filter(Boolean)
  return families
    .filter((fam) => !f.cls || fam.classes.includes(f.cls))
    .map((fam) => {
      const head = `${fam.vendor} ${fam.family} ${fam.driver}`.toLowerCase()
      const models = fam.models.filter((m) => {
        if (f.support && m.support !== f.support) return false
        const hay = `${head} ${m.name.toLowerCase()}`
        return words.every((w) => hay.includes(w))
      })
      return { ...fam, models }
    })
    .filter((fam) => fam.models.length > 0)
    .sort((a, b) => `${a.vendor} ${a.family}`.localeCompare(`${b.vendor} ${b.family}`))
}

/** Classes present in the catalog, in a stable order. */
export function catalogClasses(families: CatalogFamily[]): string[] {
  return [...new Set(families.flatMap((f) => f.classes))].sort()
}

/** "Confirmed by 2 users on v0.1.5, v0.1.4" / "Issues reported by 1 user on v0.1.4"; null without a report. */
export function communityText(c: CommunityEntry | null | undefined): string | null {
  if (!c) return null
  const n = c.reports ?? 0
  const who = n > 0 ? ` by ${n} user${n === 1 ? '' : 's'}` : ''
  const on = c.versions?.length ? ` on ${c.versions.map((v) => `v${v.replace(/^v/, '')}`).join(', ')}` : ''
  return `${c.status === 'confirmed' ? 'Confirmed' : 'Issues reported'}${who}${on}`
}

/** One line about the website check: shown under the intro. */
export function communitySourceText(c: CommunitySource | undefined): string {
  if (!c?.source) return 'Community reports are not loaded (offline check disabled or still running); showing those this build carries.'
  if (c.error) return `Community reports could not be loaded (${c.error}); showing those this build carries.`
  const n = c.issues ?? 0
  const issues = n > 0 ? `, ${n} with a reported problem` : ''
  return `Community reports for ${c.models} model${c.models === 1 ? '' : 's'}${issues} loaded from the BenchMesh website at start.`
}
