import React, { useMemo, useRef } from 'react'
import { CloseIcon, DetachIcon, FoldIcon, PinIcon, UnfoldIcon } from './icons'
import { getClassDescription } from '../instrumentClasses'
import { Selector } from '../kit'
import { Instrument } from '../types'
import { channelStatus, isOnline, useChannelApi, useInstrumentMeta } from './api'
import { PanelSize, SIZES } from './layout'
import { psuProfile, dmmProfile, ellProfile, awgProfile, oscProfile, Setters } from './profiles'
import { PsuPanel } from './panels/PsuPanel'
import { DmmPanel } from './panels/DmmPanel'
import { EllPanel } from './panels/EllPanel'
import { AwgChannel } from './panels/AwgPanel'
import { GenericPanel } from './panels/GenericPanel'
import { OscPanel } from './panels/OscPanel'

/** Model name for the header: the IDN's model field (`OWON,SPM3103,...` → SPM3103). */
export function modelOf(idn: string | undefined): string {
  if (!idn) return ''
  const parts = idn.split(',').map((s) => s.trim())
  return parts.length > 1 ? `${parts[0]} ${parts[1]}` : idn.trim()
}

function ChannelBody({ instrument, klass, ch, registry, features, setters, online }: {
  instrument: Instrument; klass: string; ch: string; registry: any; features: any; setters: Setters; online: boolean
}) {
  const api = useChannelApi(klass, instrument.id, ch)
  const status = channelStatus(registry, instrument.id, klass, ch)
  const k = klass.toUpperCase()
  const profile = useMemo(() => {
    if (k === 'PSU') return psuProfile(features, setters)
    if (k === 'DMM') return dmmProfile(features, setters)
    if (k === 'ELL') return ellProfile(features, setters)
    if (k === 'AWG') return awgProfile(features, setters)
    return null
  }, [k, features, setters])
  if (k === 'PSU') return <PsuPanel status={status} profile={profile as ReturnType<typeof psuProfile>} api={api} online={online} />
  if (k === 'DMM') return <DmmPanel status={status} profile={profile as ReturnType<typeof dmmProfile>} api={api} online={online} />
  if (k === 'ELL') return <EllPanel status={status} profile={profile as ReturnType<typeof ellProfile>} api={api} online={online} />
  if (k === 'AWG') return <AwgChannel ch={ch} status={status} profile={profile as ReturnType<typeof awgProfile>} api={api} online={online} />
  return <GenericPanel status={status} />
}

export interface ArrangeProps {
  size: PanelSize
  onSize: (s: PanelSize) => void
  onHide: () => void
  onMove: (delta: number) => void
}

/** Window-like controls in the panel header (top right). */
export interface WindowControls {
  folded: boolean
  onFold: () => void
  /** On the bench: open the panel in its own window, sized like the panel is now. */
  onDetach?: (size: { width: number; height: number }) => void
  /** In its own window: keep that window above other programs. */
  onTop?: { on: boolean; toggle: () => void }
  /** In its own window: close it (the panel returns to the bench). */
  onClose?: () => void
  /** In its own window: the header moves the (undecorated) window. */
  onDragStart?: () => void
}

/**
 * One instrument class on the bench: header (class, name, model, link, window controls) and
 * its channels; folded, only the header. In arrange mode the header carries the drag handle,
 * size and hide controls.
 */
export function InstrumentPanel({ instrument, klass, registry, arrange, win }: {
  instrument: Instrument; klass: string; registry: any; arrange?: ArrangeProps; win?: WindowControls
}) {
  const el = useRef<HTMLElement>(null)
  const folded = win?.folded ?? false
  const meta = useInstrumentMeta(klass, instrument.id)
  const online = isOnline(registry, instrument)
  const cls = instrument.classes.find((c) => c.class === klass)
  const channels = (cls?.channels ?? []).map((p) => p.split('/').pop() || '1')
  const idn = registry?.[instrument.id]?.IDN ?? instrument.IDN
  const title = getClassDescription(klass)

  return (
    <article ref={el} className={`bench-panel bench-panel--${klass.toLowerCase()}${online ? '' : ' bench-panel--offline'}${folded ? ' bench-panel--folded' : ''}`}
      aria-label={`${title}: ${instrument.name ?? instrument.id}`}>
      <header className={`bench-panel__head${win?.onDragStart ? ' bench-panel__head--drag' : ''}`}
        onPointerDown={(e) => {
          if (win?.onDragStart && e.button === 0 && !(e.target as HTMLElement).closest('button, [role="group"]')) win.onDragStart()
        }}
        onDoubleClick={(e) => { if (win && !(e.target as HTMLElement).closest('button, [role="group"]')) win.onFold() }}>
        {arrange && <span className="bench-panel__grip" aria-hidden="true" title="Drag to move">⠿</span>}
        <h2 className="bench-panel__title">{title}</h2>
        <span className="bench-panel__name">{instrument.name ?? instrument.id}</span>
        <span className="bench-panel__model">{modelOf(idn)}</span>
        <span className="bench-panel__link" data-online={online}><i />{online ? 'connected' : 'offline'}</span>
        {win && (
          <span className="bench-panel__wctl">
            {win.onTop && (
              <button type="button" className="bench-wbtn" aria-pressed={win.onTop.on} onClick={win.onTop.toggle}
                aria-label="Keep on top" title={win.onTop.on ? 'Kept on top of other windows' : 'Keep on top of other windows'}><PinIcon /></button>
            )}
            <button type="button" className="bench-wbtn" onClick={win.onFold} aria-expanded={!folded}
              aria-label={folded ? 'Unfold' : 'Fold'} title={folded ? 'Unfold' : 'Fold to the header'}>{folded ? <UnfoldIcon /> : <FoldIcon />}</button>
            {win.onDetach && (
              <button type="button" className="bench-wbtn" aria-label="Open in own window" title="Open in its own window"
                onClick={() => { const r = el.current?.getBoundingClientRect(); win.onDetach!({ width: r?.width ?? 600, height: r?.height ?? 400 }) }}><DetachIcon /></button>
            )}
            {win.onClose && (
              <button type="button" className="bench-wbtn bench-wbtn--close" onClick={win.onClose}
                aria-label="Close window" title="Close: back to the bench"><CloseIcon /></button>
            )}
          </span>
        )}
        {arrange && (
          <span className="bench-panel__arrange">
            <button type="button" className="bench-icon" onClick={() => arrange.onMove(-1)} aria-label="Move earlier" title="Move earlier">◀</button>
            <button type="button" className="bench-icon" onClick={() => arrange.onMove(1)} aria-label="Move later" title="Move later">▶</button>
            <Selector label="Panel width" value={arrange.size} onChange={(v) => arrange.onSize(v as PanelSize)}
              options={SIZES.map((s) => ({ value: s.value, label: s.label, title: s.title }))} />
            <button type="button" className="bench-icon" onClick={arrange.onHide} aria-label="Hide panel" title="Hide panel">✕</button>
          </span>
        )}
      </header>
      {folded ? null : <div className="bench-panel__body">{!meta ? (
        <span className="bench-hint">Loading…</span>
      ) : klass.toUpperCase() === 'OSC' ? (
        // one screen for all channels: the scope panel is per instrument, not per channel
        <OscPanel id={instrument.id} channels={channels} registry={registry} online={online}
          profile={oscProfile(meta.features, meta.setters, channels.length)} />
      ) : (
        <div className={`bench-panel__channels${channels.length > 1 ? ' bench-panel__channels--multi' : ''}`}>
          {channels.map((ch) => (
            <section key={ch} className="bench-panel__channel" aria-label={channels.length > 1 ? `Channel ${ch}` : undefined}>
              {channels.length > 1 && klass.toUpperCase() !== 'AWG' && <span className="kit-label">Channel {ch}</span>}
              <ChannelBody instrument={instrument} klass={klass} ch={ch} registry={registry} features={meta.features} setters={meta.setters} online={online} />
            </section>
          ))}
        </div>
      )}</div>}
      {!online && !folded && <div className="bench-panel__offline">Offline — reconnecting</div>}
    </article>
  )
}
