import React, { forwardRef, useImperativeHandle, useMemo, useRef, useState } from 'react'
import { digitModel, DigitSpec } from './digitModel'
import { useFocusedWheel } from './useFocusedWheel'
import { NOT_CONFIRMED, nearly, useConfirmation } from './confirm'

export interface DigitEditorHandle {
  /** Propose a value (e.g. from a knob): shown as pending until applied. */
  setDraft: (v: number) => void
}

interface Props {
  label: string
  /** The instrument's current value (null while unknown). */
  value: number | null
  spec: DigitSpec
  unit: string
  /** Digit grouping of the integer part (3 → `70 000 000`). */
  group?: number
  /** Send the value to the instrument; reject with an Error to keep the draft and show why. */
  onCommit: (v: number) => Promise<void> | void
  disabled?: boolean
}

/**
 * Setpoint editor: pick a digit (click or ←/→), change it with the wheel, ↑/↓ or by typing,
 * Enter applies, Esc reverts. Nothing is sent until Enter; an unsent edit is outlined.
 */
export const DigitEditor = forwardRef<DigitEditorHandle, Props>(function DigitEditor(
  { label, value, spec, unit, group = 0, onCommit, disabled }, ref,
) {
  const { min, max, intDigits, decimals } = spec
  const m = useMemo(() => digitModel({ min, max, intDigits, decimals }), [min, max, intDigits, decimals])
  const [draft, setDraft] = useState<number | null>(null)
  const [sel, setSel] = useState(spec.intDigits - 1)
  const [focused, setFocused] = useState(false)
  const [status, setStatus] = useState<'idle' | 'busy' | 'error' | 'applied'>('idle')
  const [error, setError] = useState<string | null>(null)
  const el = useRef<HTMLDivElement>(null)

  const confirm = useConfirmation<number | null>(value, nearly)
  // after Enter, show the sent value until the instrument reports it
  const base = (confirm.state === 'waiting' ? confirm.target : null) ?? value ?? Math.max(spec.min, 0)
  const shown = draft ?? base
  const pending = draft != null && draft !== value

  useImperativeHandle(ref, () => ({ setDraft: (v: number) => { setDraft(m.clamp(v)); setStatus('idle') } }), [m])

  const edit = (v: number) => {
    if (disabled || status === 'busy') return
    setDraft(v)
    if (status !== 'idle') setStatus('idle')
    setError(null)
  }

  const commit = async () => {
    if (draft == null || disabled) return
    if (draft === value) { setDraft(null); return }
    setStatus('busy')
    try {
      await onCommit(draft)
      confirm.begin(draft)
      setDraft(null)
      setStatus('applied')
    } catch (e) {
      setStatus('error')
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return
    const k = e.key
    if (k === 'ArrowUp') edit(m.bump(shown, sel, 1))
    else if (k === 'ArrowDown') edit(m.bump(shown, sel, -1))
    else if (k === 'ArrowLeft') setSel((s) => Math.max(0, s - 1))
    else if (k === 'ArrowRight') setSel((s) => Math.min(m.total - 1, s + 1))
    else if (k === 'Enter') void commit()
    else if (k === 'Escape') { setDraft(null); setStatus('idle'); setError(null) }
    else if (k === '-' && m.signed) edit(m.negate(shown))
    else if (/^[0-9]$/.test(k)) { edit(m.type(shown, sel, k)); setSel((s) => Math.min(m.total - 1, s + 1)) }
    else return
    e.preventDefault()
  }

  useFocusedWheel(el, (dir, e) => {
    const d = (e.target as HTMLElement).closest('[data-i]') as HTMLElement | null
    const i = d ? Number(d.dataset.i) : sel
    setSel(i)
    edit(m.bump(shown, i, dir))
  })

  const digits = m.digits(shown)
  const cells: React.ReactNode[] = []
  digits.forEach((ch, i) => {
    if (i === spec.intDigits && spec.decimals) cells.push(<span key="dp" className="kit-digits__sep">.</span>)
    else if (group && i > 0 && i < spec.intDigits && (spec.intDigits - i) % group === 0) {
      cells.push(<span key={`g${i}`} className="kit-digits__sep kit-digits__sep--grp" />)
    }
    cells.push(
      <span
        key={i}
        data-i={i}
        data-testid="digit"
        className={`kit-digits__d${focused && i === sel && !disabled ? ' kit-digits__d--sel' : ''}`}
        onClick={() => { setSel(i); el.current?.focus() }}
      >{ch}</span>,
    )
  })

  const cls = ['kit-digits']
  if (pending) cls.push('kit-digits--pending')
  if (status === 'busy') cls.push('kit-digits--busy')
  if (status === 'error') cls.push('kit-digits--error')
  if (status === 'applied') cls.push('kit-digits--applied')

  return (
    <div className="kit-field">
      <span className="kit-label">{label}</span>
      <div
        ref={el}
        className={cls.join(' ')}
        data-confirm={draft == null ? confirm.state : 'idle'}
        aria-busy={confirm.state === 'waiting' || status === 'busy'}
        title={confirm.state === 'unconfirmed' && draft == null ? `${NOT_CONFIRMED} (sent ${confirm.target}, reads ${value ?? '—'})` : undefined}
        tabIndex={disabled ? -1 : 0}
        role="spinbutton"
        aria-label={label}
        aria-valuenow={shown}
        aria-valuemin={spec.min}
        aria-valuemax={spec.max}
        aria-valuetext={`${shown} ${unit}`}
        aria-disabled={disabled || undefined}
        onKeyDown={onKeyDown}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onAnimationEnd={() => status === 'applied' && setStatus('idle')}
      >
        {m.signed && <span className="kit-digits__sign">{shown < 0 ? '-' : '+'}</span>}
        {cells}
        <span className="kit-digits__unit">{unit}</span>
        <span className="kit-digits__hint">{status === 'busy' ? 'SENDING' : 'ENTER ↵'}</span>
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
})
