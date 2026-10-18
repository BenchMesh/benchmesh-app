/**
 * Supported instruments (`GET /catalog`): every family the device definitions describe and each
 * model's verification status — the pure logic behind the "Supported" tab.
 */
import type { Support } from '../types'

export interface CatalogModel { name: string; support: Support; note?: string | null }
export interface CatalogFamily {
  driver: string
  vendor: string
  family: string
  classes: string[]
  transports: string[]
  models: CatalogModel[]
  manuals?: string[]
}
export interface Catalog { families: CatalogFamily[]; models: Partial<Record<Support, number>> }

export const SUPPORT_LABEL: Record<Support, string> = {
  tested: 'Bench-tested',
  confirmed: 'Community-confirmed',
  awaiting: 'Awaiting confirmation',
}

export const SUPPORT_HINT: Record<Support, string> = {
  tested: 'Verified on the maintainers’ bench',
  confirmed: 'A user ran the check on the real instrument and it passed',
  awaiting: 'Written from the manufacturer’s manual; not yet confirmed on a real instrument — help verify it',
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
