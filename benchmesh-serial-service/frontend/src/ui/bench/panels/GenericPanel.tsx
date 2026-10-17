import React from 'react'
import { Readout, Annunciators, prefixed, siNumber } from '../../kit'

const isSi = (v: unknown) => !!v && typeof v === 'object' && 'val' in (v as object)

/**
 * Any class without a dedicated panel: SI readings as readouts, booleans as annunciators,
 * other values as labelled text. Read-only; operations remain available through the API.
 */
export function GenericPanel({ status }: { status: Record<string, any> }) {
  const entries = Object.entries(status)
  const readings = entries.filter(([, v]) => isSi(v) || typeof v === 'number')
  const flags = entries.filter(([, v]) => typeof v === 'boolean')
  const texts = entries.filter(([, v]) => typeof v === 'string')
  if (!entries.length) return <span className="bench-hint">No status reported yet.</span>
  return (
    <div className="kit-glass">
      {readings.map(([k, v]) => {
        const p = prefixed(siNumber(v), 5)
        return <Readout key={k} size="small" tag={k} text={p.text} unit={p.prefix} />
      })}
      {texts.length > 0 && (
        <div className="kit-stats">
          {texts.map(([k, v]) => <span key={k}><b>{k}</b>{v}</span>)}
        </div>
      )}
      {flags.length > 0 && <Annunciators items={flags.map(([k, v]) => ({ label: k, on: v === true }))} />}
    </div>
  )
}
