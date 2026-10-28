import React, { useMemo, useRef, useState } from 'react'
import { CloseIcon, ConsoleIcon, DetachIcon, FoldIcon, ParkIcon, PinIcon, UnfoldIcon } from './icons'
import { WireConsole } from './WireConsole'
import { getClassDescription } from '../instrumentClasses'
import { Selector } from '../kit'
import { Instrument } from '../types'
import { useChannelStatus, isOnline, useChannelApi, useInstrumentMeta } from './api'
import { PanelSize, SIZES } from './layout'
import { psuProfile, dmmProfile, ellProfile, awgProfile, oscProfile, lcrProfile, salProfile, Setters } from './profiles'
import { PsuPanel } from './panels/PsuPanel'
import { DmmPanel } from './panels/DmmPanel'
import { EllPanel } from './panels/EllPanel'
import { AwgChannel } from './panels/AwgPanel'
import { GenericPanel } from './panels/GenericPanel'
import { OscPanel } from './panels/OscPanel'
import { LcrPanel } from './panels/LcrPanel'
import { SalPanel } from './panels/SalPanel'
import { SignalScope } from '../recording/signal'
import { useParkControl } from './parking'
import { offlineReason } from './offline'
import { issueHint, issueLabel } from './issue'
import { ProblemText } from './AccessFix'
import { useRun } from '../automation/RunProvider'
import { useViewOnly } from '../remote/RemoteProvider'
import { extensions } from '../../extensions'

const CONSOLES_KEY = 'benchmesh:consoles'

/** Panels (`id:CLASS`) whose console is open, remembered across restarts. */
function consoleOpen(key: string): boolean {
  try { return (JSON.parse(localStorage.getItem(CONSOLES_KEY) ?? '[]') as string[]).includes(key) } catch { return false }
}

function setConsoleOpen(key: string, open: boolean) {
  try {
    const list = (JSON.parse(localStorage.getItem(CONSOLES_KEY) ?? '[]') as string[]).filter((k) => k !== key)
    localStorage.setItem(CONSOLES_KEY, JSON.stringify(open ? [...list, key] : list))
  } catch { /* storage unavailable: the console just is not remembered */ }
}

/** Model name for the header: the IDN's model field (`OWON,SPM3103,...` → SPM3103). */
export function modelOf(idn: string | undefined): string {
  if (!idn) return ''
  const parts = idn.split(',').map((s) => s.trim())
  return parts.length > 1 ? `${parts[0]} ${parts[1]}` : idn.trim()
}

function ChannelBody({ instrument, klass, ch, multi, registry, features, setters, online }: {
  instrument: Instrument; klass: string; ch: string; multi: boolean; registry: any; features: any; setters: Setters; online: boolean
}) {
  const scope = useMemo(() => ({ deviceId: instrument.id, klass, channel: ch, name: instrument.name ?? instrument.id, multi }), [instrument.id, instrument.name, klass, ch, multi])
  return <SignalScope value={scope}><ChannelPanel instrument={instrument} klass={klass} ch={ch} registry={registry} features={features} setters={setters} online={online} /></SignalScope>
}

function ChannelPanel({ instrument, klass, ch, registry, features, setters, online }: {
  instrument: Instrument; klass: string; ch: string; registry: any; features: any; setters: Setters; online: boolean
}) {
  const api = useChannelApi(klass, instrument.id, ch)
  const status = useChannelStatus(registry, instrument.id, klass, ch)
  const k = klass.toUpperCase()
  const profile = useMemo(() => {
    if (k === 'PSU') return psuProfile(features, setters)
    if (k === 'DMM') return dmmProfile(features, setters)
    if (k === 'ELL') return ellProfile(features, setters)
    if (k === 'AWG') return awgProfile(features, setters)
    if (k === 'LCR') return lcrProfile(features, setters)
    if (k === 'SAL') return salProfile(features, setters)
    return null
  }, [k, features, setters])
  if (k === 'PSU') return <PsuPanel status={status} profile={profile as ReturnType<typeof psuProfile>} api={api} online={online} />
  if (k === 'DMM') return <DmmPanel status={status} profile={profile as ReturnType<typeof dmmProfile>} api={api} online={online} />
  if (k === 'ELL') return <EllPanel status={status} profile={profile as ReturnType<typeof ellProfile>} api={api} online={online} />
  if (k === 'AWG') return <AwgChannel ch={ch} status={status} profile={profile as ReturnType<typeof awgProfile>} api={api} online={online} />
  if (k === 'LCR') return <LcrPanel status={status} profile={profile as ReturnType<typeof lcrProfile>} api={api} online={online} />
  if (k === 'SAL') return <SalPanel status={status} profile={profile as ReturnType<typeof salProfile>} api={api} online={online} />
  return <GenericPanel status={status} />
}

export interface ArrangeProps {
  size: PanelSize
  onSize: (s: PanelSize) => void
  onHide: () => void
  onMove: (delta: number) => void
  /** Channels side by side or stacked (offered when the panel has more than one channel). */
  onStack: () => void
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
  /** After the instrument was parked (its own window closes). */
  onParked?: () => void
}

/**
 * Park the instrument (header, before Fold): refused while the running flow uses it; with an
 * output on, the first press asks and a second one parks. Messages take their own header line.
 */
function ParkButton({ instrument, registry, onParked }: { instrument: Instrument; registry: any; onParked?: () => void }) {
  const run = useRun()
  const park = useParkControl(instrument, registry, run?.status, onParked)
  const title = park.blockedBy
    ? `Used by the running flow "${park.blockedBy}": it cannot be parked until the flow ends`
    : park.asking ? 'Press again to park' : 'Park: disconnect and stop all traffic; it waits in the parking strip on the left'
  return (
    <>
      <button type="button" className={`bench-wbtn bench-wbtn--park${park.asking ? ' bench-wbtn--asking' : ''}`} onClick={() => void park.press()}
        disabled={Boolean(park.blockedBy) || park.busy} aria-label={park.asking ? 'Park anyway' : 'Park instrument'} title={title}><ParkIcon /></button>
      {(park.asking || park.error) && (
        <span className={`bench-panel__notice${park.error ? ' kit-error' : ''}`} role="alert">
          {park.error ?? `${park.asking!.join(', ')} ${park.asking!.length > 1 ? 'are' : 'is'} on and stays on, unwatched, while parked. Press park again to park anyway.`}
          <button type="button" className="bench-icon" onClick={park.dismiss} aria-label="Dismiss">✕</button>
        </span>
      )}
    </>
  )
}

/**
 * One instrument class on the bench: header (class, name, model, link, window controls) and
 * its channels; folded, only the header. In arrange mode the header carries the drag handle,
 * size and hide controls.
 */
export function InstrumentPanel({ instrument, klass, registry, arrange, win, stacked = false }: {
  instrument: Instrument; klass: string; registry: any; arrange?: ArrangeProps; win?: WindowControls
  /** Multi-channel panels: channels stacked vertically instead of side by side. */
  stacked?: boolean
}) {
  const el = useRef<HTMLElement>(null)
  const folded = win?.folded ?? false
  const meta = useInstrumentMeta(klass, instrument.id)
  const consoleKey = `${instrument.id}:${klass}`
  const [showConsole, setShowConsole] = useState(() => consoleOpen(consoleKey))
  const toggleConsole = () => { setConsoleOpen(consoleKey, !showConsole); setShowConsole(!showConsole) }
  const online = isOnline(registry, instrument)
  // a view-only remote session sees the panel and cannot use it (`inert`: no pointer, no focus)
  const viewOnly = useViewOnly()
  const cls = instrument.classes.find((c) => c.class === klass)
  const channels = (cls?.channels ?? []).map((p) => p.split('/').pop() || '1')
  const idn = registry?.[instrument.id]?.IDN ?? instrument.IDN
  const title = getClassDescription(klass)
  // the oscilloscope draws all its channels on one screen
  const perChannel = klass.toUpperCase() !== 'OSC'
  const multi = channels.length > 1

  return (
    <article ref={el} className={`bench-panel bench-panel--${klass.toLowerCase()}${online ? '' : ' bench-panel--offline'}${folded ? ' bench-panel--folded' : ''}${viewOnly ? ' bench-panel--view' : ''}`}
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
        {instrument.support === 'awaiting' && (
          <span className="bench-panel__support" data-testid="support-awaiting"
            title="Written from the manufacturer's manual and not yet confirmed on a real instrument. If something does not work, please tell us.">
            awaiting confirmation
          </span>
        )}
        {instrument.issue && (
          <span className={`bench-panel__issue bench-panel__issue--${instrument.issue.state}`} data-testid="community-issue" title={issueHint(instrument.issue)}>
            {issueLabel(instrument.issue)}
          </span>
        )}
        <span className="bench-panel__link" data-online={online}><i />{online ? 'connected' : 'offline'}</span>
        {!arrange && (
          <button type="button" className={`bench-wbtn bench-panel__console-btn${showConsole ? ' bench-wbtn--on' : ''}`} onClick={toggleConsole}
            aria-pressed={showConsole} aria-label="Instrument console" title={showConsole ? 'Hide the console' : 'Show what is sent to and received from the instrument'}>
            <ConsoleIcon />
          </button>
        )}
        {win && (
          <span className="bench-panel__wctl">
            {win.onTop && (
              <button type="button" className="bench-wbtn" aria-pressed={win.onTop.on} onClick={win.onTop.toggle}
                aria-label="Keep on top" title={win.onTop.on ? 'Kept on top of other windows' : 'Keep on top of other windows'}><PinIcon /></button>
            )}
            {!arrange && !viewOnly && extensions.panelActions.map((Action, i) => (
              <Action key={i} instrumentId={instrument.id} klass={klass} name={instrument.name ?? instrument.id} online={online} />
            ))}
            {!arrange && !viewOnly && <ParkButton instrument={instrument} registry={registry} onParked={win.onParked} />}
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
            {perChannel && multi && (
              <Selector label="Channel layout" value={stacked ? 'stacked' : 'row'} onChange={(v) => { if ((v === 'stacked') !== stacked) arrange.onStack() }}
                options={[
                  { value: 'row', label: <><span aria-hidden="true">⇆</span><span className="bench-sr">Side by side</span></>, title: 'Channels side by side' },
                  { value: 'stacked', label: <><span aria-hidden="true">⇅</span><span className="bench-sr">Stacked</span></>, title: 'Channels stacked' },
                ]} />
            )}
            <button type="button" className="bench-icon" onClick={arrange.onHide} aria-label="Hide panel" title="Hide panel">✕</button>
          </span>
        )}
      </header>
      {folded ? null : <div className="bench-panel__body" {...(viewOnly ? { inert: '' } : {})}>{!meta ? (
        <span className="bench-hint">Loading…</span>
      ) : !perChannel ? (
        // one screen for all channels: the scope panel is per instrument, not per channel
        <SignalScope value={{ deviceId: instrument.id, klass, name: instrument.name ?? instrument.id, multi: channels.length > 1 }}>
          <OscPanel id={instrument.id} channels={channels} registry={registry} online={online}
            profile={oscProfile(meta.features, meta.setters, channels.length)} />
        </SignalScope>
      ) : (
        <div className={`bench-panel__channels${multi ? (stacked ? ' bench-panel__channels--stacked' : ' bench-panel__channels--multi') : ''}`}>
          {channels.map((ch) => (
            <section key={ch} className="bench-panel__channel" aria-label={channels.length > 1 ? `Channel ${ch}` : undefined}>
              {channels.length > 1 && klass.toUpperCase() !== 'AWG' && <span className="kit-label">Channel {ch}</span>}
              <ChannelBody instrument={instrument} klass={klass} ch={ch} multi={channels.length > 1} registry={registry} features={meta.features} setters={meta.setters} online={online} />
            </section>
          ))}
        </div>
      )}</div>}
      {!online && !folded && (
        <div className="bench-panel__offline" role="status">
          Offline - reconnecting
          {offlineReason(instrument.health) && <ProblemText className="bench-panel__offline-why" text={offlineReason(instrument.health)!} />}
        </div>
      )}
      {showConsole && !folded && (
        <WireConsole meta={{ instrument: instrument.id, klass, model: instrument.model ?? modelOf(idn), idn, support: instrument.support,
          link: instrument.port ? `${instrument.transport ?? ''} ${instrument.port}`.trim() : undefined }} />
      )}
    </article>
  )
}
