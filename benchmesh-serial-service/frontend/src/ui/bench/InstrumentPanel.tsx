import React, { useMemo } from 'react'
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

/**
 * One instrument class on the bench: header (class, name, model, link) and its channels.
 * In arrange mode the header carries the drag handle, size and hide controls.
 */
export function InstrumentPanel({ instrument, klass, registry, arrange }: {
  instrument: Instrument; klass: string; registry: any; arrange?: ArrangeProps
}) {
  const meta = useInstrumentMeta(klass, instrument.id)
  const online = isOnline(registry, instrument)
  const cls = instrument.classes.find((c) => c.class === klass)
  const channels = (cls?.channels ?? []).map((p) => p.split('/').pop() || '1')
  const idn = registry?.[instrument.id]?.IDN ?? instrument.IDN
  const title = getClassDescription(klass)

  return (
    <article className={`bench-panel bench-panel--${klass.toLowerCase()}${online ? '' : ' bench-panel--offline'}`} aria-label={`${title}: ${instrument.name ?? instrument.id}`}>
      <header className="bench-panel__head">
        {arrange && <span className="bench-panel__grip" aria-hidden="true" title="Drag to move">⠿</span>}
        <h2 className="bench-panel__title">{title}</h2>
        <span className="bench-panel__name">{instrument.name ?? instrument.id}</span>
        <span className="bench-panel__model">{modelOf(idn)}</span>
        <span className="bench-panel__link" data-online={online}><i />{online ? 'connected' : 'offline'}</span>
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
      {!meta ? (
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
      )}
      {!online && <div className="bench-panel__offline">Offline — reconnecting</div>}
    </article>
  )
}
