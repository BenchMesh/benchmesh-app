/**
 * Recordable readings. A class panel's readings are *signals* `{device_id, class, channel,
 * field, label, unit}`: `SignalScope` (set where a panel knows its instrument and channel)
 * supplies the instrument part, `Recordable` wraps one reading. Right-click, the context-menu
 * key or Shift+F10 opens a menu with the active recordings (checked = recorded here; choosing
 * adds or removes the series) and "New recording…". A recorded reading shows a small red dot.
 * Outside a scope or a `RecordingsProvider` a reading is shown as it is.
 */
import React, { createContext, useContext, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu, MenuItem, useContextMenu } from '../kit'
import { CONFIRM_TIMEOUT_MS } from '../kit/confirm'
import { NewRecordingForm } from './NewRecordingForm'
import './recording.css'
import { RecordingsApi, seriesOf, useRecordings } from './RecordingsProvider'
import { isActive, Recording, Signal } from './types'

export interface SignalScopeValue {
  deviceId: string
  klass: string
  /** Channel of the panel; readings of several channels (oscilloscope) pass their own. */
  channel?: string
  /** Instrument name for labels. */
  name: string
  /** Several channels: labels name the channel. */
  multi: boolean
}

const Scope = createContext<SignalScopeValue | null>(null)

export function SignalScope({ value, children }: { value: SignalScopeValue; children: React.ReactNode }) {
  return <Scope.Provider value={value}>{children}</Scope.Provider>
}

/** The signal of `field` in the current scope (label `"<name> · [CHn · ]<tag>"`). */
export function signalOf(scope: SignalScopeValue, field: string, unit: string, opts: { channel?: string; tag?: string } = {}): Signal {
  const ch = opts.channel ?? scope.channel ?? '1'
  return {
    device_id: scope.deviceId,
    class: scope.klass.toUpperCase(),
    channel: Number(ch) || 1,
    field,
    label: `${scope.name} · ${scope.multi ? `CH${ch} · ` : ''}${opts.tag ?? field}`,
    unit,
  }
}

interface Props {
  field: string
  unit: string
  /** Label suffix (default: the field). */
  tag?: string
  channel?: string
  /** Wrap inline content (`span`) instead of a block. */
  inline?: boolean
  /** The wrapper takes keyboard focus itself (off when the content is a focusable control). */
  focusable?: boolean
  className?: string
  children: React.ReactNode
}

export function Recordable(props: Props) {
  const scope = useContext(Scope)
  const rec = useRecordings()
  if (!scope || !rec) {
    const Tag = props.inline ? 'span' : 'div'
    return <Tag className={props.className}>{props.children}</Tag>
  }
  return <RecordableReading {...props} rec={rec} signal={signalOf(scope, props.field, props.unit, props)} />
}

type Pending = Record<number, { target: boolean; expired: boolean; error?: string }>

function RecordableReading({ signal, rec, inline, focusable = true, className, children }: Props & { rec: RecordingsApi; signal: Signal }) {
  const menu = useContextMenu()
  const [creating, setCreating] = useState<{ x: number; y: number } | null>(null)
  const [pending, setPending] = useState<Pending>({})
  const recorded = seriesOf(rec.recordings, signal)
  const active = rec.recordings.filter(isActive)
  const recordedIn = rec.recordings.filter((r) => recorded.has(r.id) && isActive(r)).map((r) => r.name)

  // a toggle is confirmed when the recording reports the requested state
  const settled = Object.entries(pending).filter(([id, p]) => !p.error && recorded.has(Number(id)) === p.target)
  if (settled.length) {
    const next = { ...pending }
    for (const [id] of settled) delete next[Number(id)]
    setPending(next)
  }
  // not reported within the timeout: unconfirmed (the kit's red dot)
  const waiting = Object.values(pending).some((p) => !p.error && !p.expired)
  useEffect(() => {
    if (!waiting) return
    const t = setTimeout(() => setPending((ps) => Object.fromEntries(Object.entries(ps).map(([k, v]) => [k, v.error ? v : { ...v, expired: true }]))), CONFIRM_TIMEOUT_MS)
    return () => clearTimeout(t)
  }, [waiting])

  const toggle = (r: Recording) => {
    const series = recorded.get(r.id)
    const target = !series
    setPending((p) => ({ ...p, [r.id]: { target, expired: false } }))
    const req = series ? rec.removeSeries(r.id, series.id) : rec.addSeries(r.id, signal)
    req.catch((e) => setPending((p) => ({ ...p, [r.id]: { target, expired: false, error: e instanceof Error ? e.message : String(e) } })))
  }

  const items: MenuItem[] = active.map((r) => {
    const p = pending[r.id]
    return {
      key: `r${r.id}`,
      label: r.name,
      name: r.name,
      checked: recorded.has(r.id),
      hint: r.state === 'paused' ? 'paused' : undefined,
      keepOpen: true,
      confirm: p && !p.error ? (p.expired ? 'unconfirmed' : 'waiting') : 'idle',
      error: p?.error ?? null,
      onSelect: () => { if (!p || p.error) toggle(r) },
    }
  })
  items.push({
    key: 'new',
    label: 'New recording…',
    separator: active.length > 0,
    onSelect: () => setCreating(menu.at ?? { x: 0, y: 0 }),
  })

  const Tag = inline ? 'span' : 'div'
  return (
    <Tag className={`rec-signal${inline ? ' rec-signal--inline' : ''}${className ? ` ${className}` : ''}`}
      data-recorded={recordedIn.length > 0}
      tabIndex={focusable ? 0 : undefined} role="group" aria-label={signal.label} aria-haspopup="menu"
      title={recordedIn.length ? `Recorded in: ${recordedIn.join(', ')}` : 'Right-click to record this reading'}
      {...menu.props}>
      {children}
      {recordedIn.length > 0 && <span className="rec-dot" role="img" aria-label={`Recorded in ${recordedIn.join(', ')}`} />}
      {menu.at && (
        <ContextMenu x={menu.at.x} y={menu.at.y} label={`Record ${signal.label}`} items={items}
          onClose={() => { menu.close(); setPending((p) => Object.fromEntries(Object.entries(p).filter(([, v]) => !v.error))) }} />
      )}
      {creating && (
        <NewRecordingDialog at={creating} signal={signal} rec={rec} onClose={() => setCreating(null)} />
      )}
    </Tag>
  )
}

/** "New recording…" from a reading: create the recording, then add the reading to it. */
function NewRecordingDialog({ at, signal, rec, onClose }: { at: { x: number; y: number }; signal: Signal; rec: RecordingsApi; onClose: () => void }) {
  const [error, setError] = useState<string | null>(null)
  const failed = useRef(false)
  const stop = (e: React.SyntheticEvent) => e.stopPropagation()
  // portalled: still inside the reading in React's tree, so keep its events from reaching it
  return createPortal(
    <div className="rec-popover-layer" onClick={stop} onContextMenu={stop} onKeyDown={stop}>
      <div className="rec-popover__scrim" onClick={onClose} />
      <div className="rec-popover" role="dialog" aria-label={`New recording with ${signal.label}`}
        style={{ left: Math.max(8, Math.min(at.x, window.innerWidth - 348)), top: Math.max(8, Math.min(at.y, window.innerHeight - 300)) }}>
        <div className="rec-popover__title">New recording with <b>{signal.label}</b></div>
        <NewRecordingForm onCancel={onClose}
          onCreate={async (r) => {
            const created = await rec.create(r)
            try {
              await rec.addSeries(created.id, signal)
            } catch (e) {
              failed.current = true
              setError(`Recording "${created.name}" started, but the reading was not added: ${e instanceof Error ? e.message : e}`)
            }
            return created
          }}
          onDone={() => { if (!failed.current) onClose() }} />
        {error && <span className="kit-error" role="alert">{error}</span>}
      </div>
    </div>,
    document.body,
  )
}
