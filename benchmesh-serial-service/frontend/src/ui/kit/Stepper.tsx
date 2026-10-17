import React, { useRef, useState } from 'react'
import { useFocusedWheel } from './useFocusedWheel'

/**
 * Stepped setting (V/div, s/div, position): ◀ value ▶, arrow keys or the wheel while
 * focused. Every step is sent at once, like turning a detented knob on the instrument.
 * `onStep` may reject with an Error to show why.
 */
export function Stepper({ label, text, onStep, disabled, title }: {
  label: string; text: string; onStep: (dir: number) => Promise<void> | void; disabled?: boolean; title?: string
}) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const el = useRef<HTMLDivElement>(null)
  const step = async (dir: number) => {
    if (disabled || busy) return
    setBusy(true)
    setError(null)
    try { await onStep(dir) } catch (e) { setError(e instanceof Error ? e.message : String(e)) } finally { setBusy(false) }
  }
  useFocusedWheel(el, (dir) => { void step(dir) })
  return (
    <div className="kit-field">
      <span className="kit-label">{label}</span>
      <div
        ref={el}
        className={`kit-stepper${busy ? ' kit-stepper--busy' : ''}${error ? ' kit-stepper--error' : ''}`}
        role="spinbutton"
        tabIndex={disabled ? -1 : 0}
        aria-label={label}
        aria-valuetext={text}
        aria-disabled={disabled || undefined}
        title={error ?? title}
        onKeyDown={(e) => {
          const d = ['ArrowUp', 'ArrowRight'].includes(e.key) ? 1 : ['ArrowDown', 'ArrowLeft'].includes(e.key) ? -1 : 0
          if (d) { e.preventDefault(); void step(d) }
        }}
      >
        <button type="button" tabIndex={-1} aria-label={`${label} down`} disabled={disabled} onClick={() => step(-1)}>◀</button>
        <span className="kit-stepper__val">{text}</span>
        <button type="button" tabIndex={-1} aria-label={`${label} up`} disabled={disabled} onClick={() => step(1)}>▶</button>
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
