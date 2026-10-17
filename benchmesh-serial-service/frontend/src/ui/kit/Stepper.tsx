import React, { useRef, useState } from 'react'
import { useFocusedWheel } from './useFocusedWheel'
import { NOT_CONFIRMED, useConfirmation } from './confirm'

// instruments quantise steps (positions, levels): confirm within 0.2 %
const close = (a: number | null, b: number | null) => a != null && b != null && Math.abs(a - b) <= Math.max(1e-12, Math.abs(b) * 2e-3)

/**
 * Stepped setting (V/div, s/div, position): ◀ value ▶, arrow keys or the wheel while
 * focused. Every step is sent at once, like turning a detented knob on the instrument.
 * `onStep` returns the requested value (or rejects with an Error to show why); with the
 * instrument's reported `value` the stepper waits until the instrument confirms it.
 */
export function Stepper({ label, text, value, format, onStep, disabled, title }: {
  label: string; text: string; value?: number | null
  /** Text for a value: shows the requested value while waiting for confirmation. */
  format?: (v: number) => string
  onStep: (dir: number) => Promise<number | void> | number | void; disabled?: boolean; title?: string
}) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const el = useRef<HTMLDivElement>(null)
  const confirm = useConfirmation<number | null>(value ?? null, close)
  const step = async (dir: number) => {
    if (disabled || busy) return
    setBusy(true)
    setError(null)
    try {
      const target = await onStep(dir)
      if (typeof target === 'number' && value !== undefined) confirm.begin(target)
    } catch (e) { setError(e instanceof Error ? e.message : String(e)) } finally { setBusy(false) }
  }
  useFocusedWheel(el, (dir) => { void step(dir) })
  return (
    <div className="kit-field">
      <span className="kit-label">{label}</span>
      <div
        ref={el}
        className={`kit-stepper${busy ? ' kit-stepper--busy' : ''}${error ? ' kit-stepper--error' : ''}`}
        data-confirm={confirm.state} aria-busy={busy || confirm.state === 'waiting'}
        role="spinbutton"
        tabIndex={disabled ? -1 : 0}
        aria-label={label}
        aria-valuetext={text}
        aria-disabled={disabled || undefined}
        title={error ?? (confirm.state === 'unconfirmed' ? NOT_CONFIRMED : title)}
        onKeyDown={(e) => {
          const d = ['ArrowUp', 'ArrowRight'].includes(e.key) ? 1 : ['ArrowDown', 'ArrowLeft'].includes(e.key) ? -1 : 0
          if (d) { e.preventDefault(); void step(d) }
        }}
      >
        <button type="button" tabIndex={-1} aria-label={`${label} down`} disabled={disabled} onClick={() => step(-1)}>◀</button>
        <span className="kit-stepper__val">{confirm.state === 'waiting' && format && confirm.target != null ? format(confirm.target) : text}</span>
        <button type="button" tabIndex={-1} aria-label={`${label} up`} disabled={disabled} onClick={() => step(1)}>▶</button>
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
