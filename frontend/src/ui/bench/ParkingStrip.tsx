import React, { useEffect, useState } from 'react'
import { Instrument } from '../types'
import { getClassDescription } from '../instrumentClasses'
import { ParkIcon } from './icons'
import { Led, type LedState } from './Led'
import { parkedState, ParkedState, presenceKind, useParking } from './parking'

/** One line about a parked instrument's presence. */
export function presenceText(s: ParkedState): string {
  const at = s.detail ? ` at ${s.detail}` : ''
  switch (presenceKind(s)) {
    case 'checking': return 'Checking whether it is attached…'
    case 'absent': return `Not found${s.detail ? `: ${s.detail}` : ''}`
    case 'port': return `Port attached${at}. The instrument may be switched off: its USB interface can be powered by the computer. Unpark to find out.`
    case 'instrument': return `Attached${at}`
  }
}

/** Short state for the tab's accessible name. */
const PRESENCE_WORD = { checking: 'checking', instrument: 'attached', port: 'port attached, instrument unknown', absent: 'not found' } as const

/** The presence LED: the instrument seen (green), only its port (amber: may be switched off), not found (red), checking (dark). */
const PRESENCE_LED: Record<keyof typeof PRESENCE_WORD, LedState> = { checking: 'off', instrument: 'ok', port: 'warn', absent: 'alert' }

function PresenceLed({ s }: { s: ParkedState }) {
  const kind = presenceKind(s)
  return <Led state={PRESENCE_LED[kind]} label={PRESENCE_WORD[kind]} title={`Parked. ${presenceText(s)}`} />
}

/**
 * Parked instruments, collected in a slim strip on the left of the bench (shown only while one
 * is parked): each with its presence LED before its name. Opening the strip shows the details and
 * Unpark; the panels then return to their place on the bench.
 */
export function ParkingStrip({ parked, registry }: { parked: Instrument[]; registry: any }) {
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState<string | null>(null)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const setParked = useParking()

  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false)
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open])
  useEffect(() => { if (!parked.length) setOpen(false) }, [parked.length])
  if (!parked.length) return null

  const unpark = async (id: string) => {
    setBusy(id)
    setErrors(({ [id]: _, ...rest }) => rest)
    try {
      await setParked(id, false)
    } catch (e) {
      setErrors((x) => ({ ...x, [id]: e instanceof Error ? e.message : String(e) }))
    } finally {
      setBusy(null)
    }
  }
  const name = (i: Instrument) => i.name ?? i.id

  return (
    <aside className="bench-parking" aria-label="Parked instruments" data-open={open}>
      <button type="button" className="bench-parking__toggle" aria-expanded={open} onClick={() => setOpen((o) => !o)}
        title={open ? 'Close the parking' : 'Parked instruments: details and unpark'}>
        <ParkIcon /><span>{parked.length}</span>
      </button>
      <ul className="bench-parking__list">
        {parked.map((i) => {
          const s = parkedState(i, registry)
          return (
            <li key={i.id}>
              <button type="button" className="bench-parking__tab" onClick={() => setOpen(true)} title={`${name(i)} (parked): ${presenceText(s)}`}
                aria-label={`${name(i)}, parked, ${PRESENCE_WORD[presenceKind(s)]}`}>
                <span className="bench-parking__name"><PresenceLed s={s} />{name(i)}</span>
              </button>
            </li>
          )
        })}
      </ul>
      {open && (
        <div className="bench-parking__sheet" role="dialog" aria-label="Parking">
          <div className="bench-parking__head">
            <h2>Parking</h2>
            <button type="button" className="bench-icon" onClick={() => setOpen(false)} aria-label="Close">✕</button>
          </div>
          <div className="bench-page">
            <p className="bench-hint">
              Parked instruments are disconnected: nothing is sent to them, BenchMesh only checks every few seconds whether
              they are attached. For a serial instrument that shows only its port (amber): whether the instrument is
              switched on is known once it answers. Unpark one to connect it again; its panels return to their place.
            </p>
            {parked.map((i) => {
              const s = parkedState(i, registry)
              return (
                <section key={i.id} className="bench-group bench-parking__item" aria-label={name(i)}>
                  <div className="bench-group__row">
                    <strong><PresenceLed s={s} />{name(i)}</strong>
                    <span className="bench-hint">{[i.model, i.classes.map((c) => getClassDescription(c.class)).join(', ')].filter(Boolean).join(' · ')}</span>
                  </div>
                  <span className="bench-hint" data-testid="presence">{presenceText(s)}</span>
                  <div className="bench-group__row">
                    <button type="button" className="bench-btn bench-btn--primary" disabled={busy === i.id} onClick={() => void unpark(i.id)}>
                      {busy === i.id ? 'Unparking…' : 'Unpark'}
                    </button>
                    {errors[i.id] && <span className="kit-error" role="alert">{errors[i.id]}</span>}
                  </div>
                </section>
              )
            })}
          </div>
        </div>
      )}
    </aside>
  )
}
