import React, { useEffect, useRef, useState } from 'react'

const ARM_MS = 3000

/**
 * Switch for anything that energises an output or input: press twice within 3 s to switch
 * on, once to switch off. `on` is the instrument's reported state; `onChange` sends the
 * request (reject with an Error to show why it failed).
 */
export function ArmSwitch({ label, on, onChange, disabled }: {
  label: string
  on: boolean
  onChange: (next: boolean) => Promise<void> | void
  disabled?: boolean
}) {
  const [armed, setArmed] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const timer = useRef<ReturnType<typeof setTimeout>>()

  useEffect(() => () => clearTimeout(timer.current), [])

  const send = async (next: boolean) => {
    setBusy(true)
    setError(null)
    try {
      await onChange(next)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const click = () => {
    if (busy) return
    clearTimeout(timer.current)
    if (on) { void send(false); return }
    if (!armed) {
      setArmed(true)
      timer.current = setTimeout(() => setArmed(false), ARM_MS)
      return
    }
    setArmed(false)
    void send(true)
  }

  const state = busy ? 'busy' : on ? 'on' : armed ? 'armed' : 'off' // armed only matters while off
  const sub = error ?? {
    busy: 'sending…', on: 'press once to switch off', armed: 'press again to confirm', off: 'press twice to switch on',
  }[state]

  return (
    <button type="button" className="kit-arm" data-state={state} aria-pressed={on} disabled={disabled} onClick={click}>
      <span className="kit-arm__led" />
      <span>
        {label}
        <span className="kit-arm__sub" style={error ? { color: 'var(--kit-alert)' } : undefined} role={error ? 'alert' : undefined}>{sub}</span>
      </span>
    </button>
  )
}
