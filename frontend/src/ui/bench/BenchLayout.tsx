import React, { lazy, Suspense, useEffect, useMemo, useRef, useState } from 'react'
import '../kit/kit.css'
import './bench.css'
import { Instrument } from '../types'
import { SettingsView } from '../drawers/SettingsView'
import { LogsPanel } from '../drawers/LogsPanel'
import { Rack } from './Rack'
import { ParkingStrip } from './ParkingStrip'
import { isParked } from './parking'
import { splitPanelKey } from './PanelWindow'
import { clearDetached, markDetached, useDetachedKeys } from './detach'
import { closePanelWindow, focusPanelWindow, openPanelWindow } from '../desktop'
import { getClassDescription } from '../instrumentClasses'
import { SkinProvider } from '../kit'
import { TitleBar } from './TitleBar'
import { RecorderDock } from '../recording/RecorderDock'
import { UpdatePrompt } from '../update/UpdatePrompt'
import { RunIndicator, RunProvider } from '../automation/RunProvider'
import { RemoteProvider } from '../remote/RemoteProvider'
import { RemoteBadge } from '../remote/RemoteBadge'
import { AboutView } from '../about/AboutView'
import { useAppVersion } from '../about/version'
import { BenchLayout as Layout, EMPTY_LAYOUT, loadLayout, panelsOf, reconcile, saveLayout } from './layout'

// loaded when first opened (with its chart)
const RecordingsDrawer = lazy(() => import('../recording/RecordingsDrawer').then((m) => ({ default: m.RecordingsDrawer })))
const AutomationDrawer = lazy(() => import('../automation/AutomationDrawer').then((m) => ({ default: m.AutomationDrawer })))

type DrawerKind = 'instruments' | 'log' | 'recordings' | 'automation' | 'about' | null

function Drawer({ title, onClose, children, wide }: { title: string; onClose: () => void; children: React.ReactNode; wide?: boolean }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])
  return (
    <div className="bench-drawer" role="dialog" aria-label={title}>
      <div className="bench-drawer__scrim" onClick={onClose} />
      <div className={`bench-drawer__sheet${wide ? ' bench-drawer__sheet--wide' : ''}`}>
        <div className="bench-drawer__head">
          <h2>{title}</h2>
          <button type="button" className="bench-icon" onClick={onClose} aria-label="Close">✕</button>
        </div>
        <div className="bench-drawer__body">{children}</div>
      </div>
    </div>
  )
}

interface Props {
  instruments: Instrument[]
  registry: any
  apiBase: string
  loading: boolean
  error: string | null
  wsDiag: { ok: boolean; msg: string }
  onConfigUpdated: () => void
  onOpenDocs: () => void
}

/** The application shell: title bar (brand, version, Arrange, window controls), top bar (with the automation run indicator and the remote access badge), the instrument rack, the recorder dock, and drawers for setup, recordings, automation, the request log and about / feedback. */
export function BenchLayout(props: Props) {
  return <SkinProvider><RemoteProvider><RunProvider><BenchShell {...props} /></RunProvider></RemoteProvider></SkinProvider>
}

function BenchShell({ instruments, registry, apiBase, loading, error, wsDiag, onConfigUpdated, onOpenDocs }: Props) {
  const panels = useMemo(() => panelsOf(instruments), [instruments])
  const [saved, setLayout] = useState<Layout>(() => loadLayout())
  const [arranging, setArranging] = useState(false)
  const [drawer, setDrawer] = useState<DrawerKind>(null)
  const [autoAdd, setAutoAdd] = useState(false)
  const version = useAppVersion()

  // The layout follows the configured instruments: new ones appear, removed ones go.
  const layout = useMemo(() => (loading ? saved : reconcile(saved, panels)), [saved, panels, loading])
  useEffect(() => { if (layout !== EMPTY_LAYOUT) saveLayout(layout) }, [layout])

  // parked instruments: out of the rack (keeping their place), into the parking strip
  const parked = useMemo(() => instruments.filter((i) => isParked(i, registry)), [instruments, registry])
  const parkedIds = useMemo(() => new Set(parked.map((i) => i.id)), [parked])
  // the parking strip stays in view below the (sticky, wrapping) head
  const top = useRef<HTMLDivElement>(null)
  const [topHeight, setTopHeight] = useState(96)
  useEffect(() => {
    const el = top.current
    if (!el || typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(() => setTopHeight(el.offsetHeight))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const detached = useDetachedKeys().filter((k) => !parkedIds.has(splitPanelKey(k)[0]))
  const [detachError, setDetachError] = useState<string | null>(null)
  const detach = (key: string, title: string, size: { width: number; height: number }) => {
    setDetachError(null)
    markDetached(key) // off the bench at once; the window keeps it registered while it lives
    openPanelWindow(key, title, size.width, size.height).catch((e) => setDetachError(`Could not open a window: ${e instanceof Error ? e.message : e}`))
  }
  const panelTitle = (key: string) => {
    const p = panels.find((x) => x.key === key)
    const ins = instruments.find((i) => i.id === p?.instrumentId)
    return p ? `${getClassDescription(p.klass)} · ${ins?.name ?? p.instrumentId}` : key
  }

  const apiOk = !loading && !error
  const empty = !loading && !error && instruments.length === 0

  return (
    <div className="bench" style={{ '--bench-top-h': `${topHeight}px` } as React.CSSProperties}>
      <UpdatePrompt />
      <div ref={top} className="bench-head">
        <TitleBar version={version?.version} onAbout={() => setDrawer('about')}
          arranging={arranging} canArrange={instruments.length > 0} onArrange={() => setArranging((a) => !a)} />
        <header className="bench-top">
          <span className="bench-pill" data-ok={apiOk}><i />{apiOk ? 'API' : error ? 'API unreachable' : 'connecting…'}</span>
          <span className="bench-pill" data-ok={wsDiag.ok}><i />{wsDiag.ok ? 'live' : wsDiag.msg}</span>
          <RunIndicator onOpen={() => setDrawer('automation')} />
          <RemoteBadge />
          <div className="bench-tools">
            <nav className="bench-actions" aria-label="Main">
              <button type="button" className="bench-btn" aria-pressed={drawer === 'instruments'} onClick={() => setDrawer('instruments')}>Instruments</button>
              <button type="button" className="bench-btn" aria-pressed={drawer === 'recordings'} onClick={() => setDrawer('recordings')}>Recording</button>
              <button type="button" className="bench-btn" aria-pressed={drawer === 'automation'} onClick={() => setDrawer('automation')}>Automation</button>
              <button type="button" className="bench-btn" aria-pressed={drawer === 'log'} onClick={() => setDrawer('log')}>Log</button>
              <button type="button" className="bench-btn" onClick={onOpenDocs}>API</button>
              <button type="button" className="bench-btn" aria-pressed={drawer === 'about'} onClick={() => setDrawer('about')}>Feedback</button>
            </nav>
          </div>
        </header>
      </div>

      <div className="bench-body">
      {!empty && <ParkingStrip parked={parked} registry={registry} />}
      <main className="bench-main">
        {empty ? (
          <div className="bench-empty">
            <h2>No instruments yet</h2>
            <p>Add the instruments on your bench; each one appears here as a panel for its class.</p>
            <button type="button" className="bench-btn bench-btn--primary" onClick={() => { setAutoAdd(true); setDrawer('instruments') }}>Add instrument</button>
          </div>
        ) : (
          <>
            {(detached.length > 0 || detachError) && (
              <div className="bench-detached" aria-label="Panels in their own windows">
                {detached.length > 0 && <span className="kit-label">In own windows</span>}
                {detached.map((k) => (
                  <span key={k} className="bench-detached__item">
                    <button type="button" className="bench-chip" onClick={() => void focusPanelWindow(k)} title="Bring the window to the front">↗ {panelTitle(k)}</button>
                    <button type="button" className="bench-icon" onClick={() => { clearDetached(k); void closePanelWindow(k) }}
                      aria-label={`Back to the bench: ${panelTitle(k)}`} title="Close the window and show the panel here">↩</button>
                  </span>
                ))}
                {detachError && <span className="kit-error" role="alert">{detachError}</span>}
              </div>
            )}
            <Rack panels={panels} instruments={instruments} registry={registry} layout={layout} onLayout={setLayout} arranging={arranging}
              detached={detached} parked={parkedIds} onDetach={detach} />
          </>
        )}
      </main>
      </div>

      <RecorderDock />

      {drawer === 'instruments' && (
        <Drawer title="Instruments" onClose={() => setDrawer(null)}>
          <SettingsView apiBase={apiBase} onConfigUpdated={onConfigUpdated} autoAddInstrument={autoAdd} onAutoAddComplete={() => setAutoAdd(false)} instruments={instruments} />
        </Drawer>
      )}
      {drawer === 'recordings' && (
        <Drawer title="Recordings" onClose={() => setDrawer(null)}>
          <Suspense fallback={<span className="bench-hint">Loading…</span>}><RecordingsDrawer /></Suspense>
        </Drawer>
      )}
      {drawer === 'automation' && (
        <Drawer title="Automation" wide onClose={() => setDrawer(null)}>
          <Suspense fallback={<span className="bench-hint">Loading…</span>}><AutomationDrawer onOpenRecordings={() => setDrawer('recordings')} /></Suspense>
        </Drawer>
      )}
      {drawer === 'about' && (
        <Drawer title="About and feedback" onClose={() => setDrawer(null)}>
          <AboutView />
        </Drawer>
      )}
      {drawer === 'log' && (
        <Drawer title="Request log" onClose={() => setDrawer(null)}>
          <LogsPanel />
        </Drawer>
      )}
    </div>
  )
}
