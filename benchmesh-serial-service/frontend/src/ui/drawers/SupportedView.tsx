/**
 * "Supported" tab of the Instruments drawer: which instruments BenchMesh has definitions for and
 * how far each model is known to work (bench-tested, community-confirmed, awaiting confirmation).
 */
import { useEffect, useMemo, useState } from 'react'
import { apiError } from './detect'
import {
  catalogClasses, filterCatalog, SUPPORT_HINT, SUPPORT_LABEL, TRANSPORT_LABEL,
  type Catalog, type CatalogFilter,
} from './supported'
import { getClassDescription } from '../instrumentClasses'
import type { Support } from '../types'

const STATUSES: Support[] = ['tested', 'confirmed', 'awaiting']

export function SupportedView({ apiBase }: { apiBase: string }) {
  const [catalog, setCatalog] = useState<Catalog | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [filter, setFilter] = useState<CatalogFilter>({ text: '', cls: null, support: null })

  useEffect(() => {
    let live = true
    fetch(`${apiBase}/catalog`)
      .then(async (r) => { if (!r.ok) throw new Error(await apiError(r)); return r.json() })
      .then((c: Catalog) => live && setCatalog(c))
      .catch((e) => live && setError(e?.message || String(e)))
    return () => { live = false }
  }, [apiBase])

  const families = useMemo(() => (catalog ? filterCatalog(catalog.families, filter) : []), [catalog, filter])
  const classes = useMemo(() => (catalog ? catalogClasses(catalog.families) : []), [catalog])
  if (error) return <div className="instrument-config__inline-error">{error}</div>
  if (!catalog) return <span className="detect__hint">Loading…</span>

  return (
    <section className="supported" aria-label="Supported instruments" data-testid="supported-view">
      <p className="supported__intro">
        Instruments BenchMesh has definitions for. Models <b>awaiting confirmation</b> were written from the
        manufacturer’s manual; if you own one, connecting it and reporting whether it works helps everyone.
      </p>
      <div className="supported__counts">
        {STATUSES.map((s) => (
          <button key={s} type="button" className={`supported__count supported__count--${s}`} title={SUPPORT_HINT[s]}
            aria-pressed={filter.support === s} onClick={() => setFilter((f) => ({ ...f, support: f.support === s ? null : s }))}>
            <b>{catalog.models[s] ?? 0}</b> {SUPPORT_LABEL[s]}
          </button>
        ))}
      </div>
      <div className="supported__bar">
        <input type="search" placeholder="Search vendor, family or model" value={filter.text} aria-label="Search"
          onChange={(e) => setFilter((f) => ({ ...f, text: e.target.value }))} />
        <select aria-label="Class" value={filter.cls ?? ''} onChange={(e) => setFilter((f) => ({ ...f, cls: e.target.value || null }))}>
          <option value="">All classes</option>
          {classes.map((c) => <option key={c} value={c}>{getClassDescription(c)}</option>)}
        </select>
      </div>
      {families.length === 0 && <p className="detect__empty">No instrument matches.</p>}
      <ul className="supported__list">
        {families.map((fam) => (
          <li key={fam.driver} className="supported__family" data-testid="supported-family">
            <div className="supported__head">
              <span className="supported__name">{fam.vendor} {fam.family}</span>
              <span className="supported__meta">
                {fam.classes.map((c) => getClassDescription(c)).join(' · ')} — {fam.transports.map((t) => TRANSPORT_LABEL[t] ?? t).join(', ')}
              </span>
            </div>
            <ul className="supported__models">
              {fam.models.map((m) => (
                <li key={m.name} className={`supported__model supported__model--${m.support}`} title={m.note ? `${SUPPORT_HINT[m.support]} — ${m.note}` : SUPPORT_HINT[m.support]}>
                  {m.name}<i>{SUPPORT_LABEL[m.support]}</i>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </section>
  )
}
