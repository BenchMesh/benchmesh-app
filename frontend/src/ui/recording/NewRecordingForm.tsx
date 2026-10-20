/** Form for a new recording: name, description, interval (whole seconds). Shows field and server errors. */
import React, { useEffect, useRef, useState } from 'react'
import { validateNew } from './client'
import type { NewRecording, Recording } from './types'

let seq = 0

export function NewRecordingForm({ onCreate, onDone, onCancel, defaultName = '' }: {
  onCreate: (r: NewRecording) => Promise<Recording>
  onDone: (r: Recording) => void
  onCancel: () => void
  defaultName?: string
}) {
  const [id] = useState(() => `rec-new-${++seq}`)
  const [name, setName] = useState(defaultName)
  const [description, setDescription] = useState('')
  const [interval, setIntervalText] = useState('1')
  const [touched, setTouched] = useState(false)
  const [busy, setBusy] = useState(false)
  const [serverError, setServerError] = useState<string | null>(null)
  const nameRef = useRef<HTMLInputElement>(null)
  useEffect(() => { nameRef.current?.focus() }, [])
  const errors = validateNew(name, interval)
  const invalid = Object.keys(errors).length > 0

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setTouched(true)
    if (invalid || busy) return
    setBusy(true)
    setServerError(null)
    try {
      const r = await onCreate({ name: name.trim(), description: description.trim() || undefined, interval_s: Number(interval) })
      onDone(r)
    } catch (err) {
      setServerError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="rec-form" onSubmit={submit} aria-label="New recording" noValidate
      onKeyDown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); onCancel() } }}>
      <div className="kit-field rec-form__name">
        <label className="kit-label" htmlFor={`${id}-name`}>Name</label>
        <input id={`${id}-name`} ref={nameRef} className="kit-input" type="text" value={name} maxLength={100} autoComplete="off"
          aria-invalid={touched && !!errors.name} aria-describedby={touched && errors.name ? `${id}-name-err` : undefined}
          onChange={(e) => setName(e.target.value)} />
        {touched && errors.name && <span id={`${id}-name-err`} className="kit-error">{errors.name}</span>}
      </div>
      <div className="kit-field rec-form__interval">
        <label className="kit-label" htmlFor={`${id}-int`}>Interval (s)</label>
        <input id={`${id}-int`} className="kit-input" type="text" inputMode="numeric" value={interval} autoComplete="off"
          aria-invalid={touched && !!errors.interval} aria-describedby={touched && errors.interval ? `${id}-int-err` : undefined}
          onChange={(e) => setIntervalText(e.target.value)} />
        {touched && errors.interval && <span id={`${id}-int-err`} className="kit-error">{errors.interval}</span>}
      </div>
      <div className="kit-field rec-form__desc">
        <label className="kit-label" htmlFor={`${id}-desc`}>Description</label>
        <input id={`${id}-desc`} className="kit-input" type="text" value={description} autoComplete="off"
          onChange={(e) => setDescription(e.target.value)} />
      </div>
      <div className="rec-form__actions">
        <button type="submit" className="bench-btn bench-btn--primary" disabled={busy} aria-busy={busy}>{busy ? 'Starting…' : 'Start recording'}</button>
        <button type="button" className="bench-btn" onClick={onCancel}>Cancel</button>
        {serverError && <span className="kit-error" role="alert">{serverError}</span>}
      </div>
    </form>
  )
}
