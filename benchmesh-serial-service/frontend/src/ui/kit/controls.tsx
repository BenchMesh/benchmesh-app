/** Small presentational controls of the kit: readout, annunciators, switches, selectors. */
import React, { useState } from 'react'
import { SegmentDisplay } from './SegmentDisplay'
import { Sparkline } from './Sparkline'
import type { Tone } from './useCanvas'

/**
 * Measured value: tag, seven-segment number, unit, optional trend underneath. `tone` names
 * the quantity (volt, amp, watt, meas) so skins that colour-code readouts can do so.
 */
export function Readout({ tag, text, unit, size = 'big', history, label, tone }: {
  tag: string; text: string; unit: string; size?: 'big' | 'mid' | 'small'; history?: number[]; label?: string; tone?: Tone
}) {
  return (
    <div className={`kit-readout kit-readout--${size}`}>
      <span className="kit-readout__tag">{tag}</span>
      <div className="kit-readout__num"><SegmentDisplay text={text} label={label ?? tag} tone={tone} /></div>
      <span className="kit-readout__unit">{unit}</span>
      {history && <Sparkline data={history} tone={tone} />}
    </div>
  )
}

export interface Annunciator { label: string; on: boolean; alert?: boolean; title?: string }

/** State flags, always present and lit when active, so the layout never shifts. */
export function Annunciators({ items }: { items: Annunciator[] }) {
  return (
    <div className="kit-anns" role="list" aria-label="Status">
      {items.map((a) => (
        <span key={a.label} role="listitem" className={`kit-ann${a.alert ? ' kit-ann--alert' : ''}`} data-on={a.on}
          title={a.title} aria-label={`${a.label} ${a.on ? 'on' : 'off'}`}>{a.label}</span>
      ))}
    </div>
  )
}

/** Discreet switch for secondary, non-energising settings. */
export function MicroSwitch({ label, checked, onChange, disabled, title }: {
  label: string; checked: boolean; onChange: (next: boolean) => Promise<void> | void; disabled?: boolean; title?: string
}) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  return (
    <button
      type="button" role="switch" aria-checked={checked} className="kit-micro" disabled={disabled || busy}
      title={error ?? title}
      onClick={async () => {
        setBusy(true)
        setError(null)
        try { await onChange(!checked) } catch (e) { setError(e instanceof Error ? e.message : String(e)) } finally { setBusy(false) }
      }}
    >
      <span className="kit-micro__trk" />
      {label}
      {error && <span className="kit-error" role="alert">!</span>}
    </button>
  )
}

export interface Choice { value: string; label: React.ReactNode; title?: string; disabled?: boolean }

/** Mutually exclusive choice: `seg` for modes, `chips` for ranges and options. */
export function Selector({ options, value, onChange, label, variant = 'seg', disabled }: {
  options: Choice[]; value: string | null; onChange: (v: string) => void; label: string; variant?: 'seg' | 'chips'; disabled?: boolean
}) {
  return (
    <div className={variant === 'seg' ? 'kit-seg' : 'kit-chips'} role="group" aria-label={label}>
      {options.map((o) => (
        <button key={o.value} type="button" aria-pressed={o.value === value} title={o.title}
          disabled={disabled || o.disabled} onClick={() => o.value !== value && onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  )
}
