/** Building blocks of Instruments - System: meter, two-press button, command with Copy. */
import { useEffect, useRef, useState } from 'react'
import { levelOf, type Level } from './format'

export function Meter({ label, percent, text, level }: { label: string; percent: number; text: string; level?: Level }) {
  const l = level ?? levelOf(percent)
  return (
    <div className="sys-meter" role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(percent)} data-level={l}>
      <span className="kit-label">{label}</span>
      <span className="sys-meter__bar"><i style={{ width: `${percent}%` }} /></span>
      <span className="sys-meter__text">{text}</span>
    </div>
  )
}

/** Anything that stops the instruments takes a second press within 4 s. */
export function TwoPress({ label, confirmLabel, onConfirm, disabled, danger }: {
  label: string; confirmLabel: string; onConfirm: () => Promise<void>; disabled?: boolean; danger?: boolean
}) {
  const [armed, setArmed] = useState(false)
  const [busy, setBusy] = useState(false)
  const timer = useRef<ReturnType<typeof setTimeout>>()
  useEffect(() => () => clearTimeout(timer.current), [])
  const press = async () => {
    clearTimeout(timer.current)
    if (!armed) {
      setArmed(true)
      timer.current = setTimeout(() => setArmed(false), 4000)
      return
    }
    setArmed(false)
    setBusy(true)
    try { await onConfirm() } finally { setBusy(false) }
  }
  return (
    <button type="button" className={`bench-btn${danger ? ' bench-btn--danger' : ''}${armed ? ' sys-armed' : ''}`} disabled={disabled || busy} aria-busy={busy} onClick={() => void press()}>
      {armed ? confirmLabel : label}
    </button>
  )
}

export function CopyCommand({ command }: { command: string }) {
  const [copied, setCopied] = useState<boolean | null>(null)
  const copy = async () => {
    try { await navigator.clipboard.writeText(command); setCopied(true) } catch { setCopied(false) }
    setTimeout(() => setCopied(null), 2000)
  }
  return (
    <div className="sys-command">
      <code>{command}</code>
      <button type="button" className="bench-btn" onClick={() => void copy()}>
        {copied ? 'Copied' : copied === false ? 'Copy failed - select the text' : 'Copy command'}
      </button>
    </div>
  )
}
