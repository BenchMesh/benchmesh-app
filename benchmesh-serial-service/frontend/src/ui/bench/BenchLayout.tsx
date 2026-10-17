import React, { useEffect, useMemo, useState } from 'react'
import '../kit/kit.css'
import './bench.css'
import { Instrument } from '../types'
import { SettingsView } from '../drawers/SettingsView'
import { LogsPanel } from '../drawers/LogsPanel'
import { Rack } from './Rack'
import { clearDetached, markDetached, useDetachedKeys } from './detach'
import { closePanelWindow, focusPanelWindow, openPanelWindow } from '../desktop'
import { getClassDescription } from '../instrumentClasses'
import { Selector, SkinProvider, useSkin, SKINS, Skin } from '../kit'
import { BenchLayout as Layout, EMPTY_LAYOUT, loadLayout, panelsOf, reconcile, saveLayout } from './layout'

type DrawerKind = 'instruments' | 'log' | null

function Drawer({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])
  return (
    <div className="bench-drawer" role="dialog" aria-label={title}>
      <div className="bench-drawer__scrim" onClick={onClose} />
      <div className="bench-drawer__sheet">
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
  onOpenRecording: () => void
  onOpenMetrics: () => void
  onOpenDocs: () => void
}

/** Top-bar switch between the kit skins (persisted per browser). */
function SkinSwitch() {
  const { skin, setSkin } = useSkin()
  return (
    <div className="bench-skin">
      <span className="kit-label" aria-hidden="true">Skin</span>
      <Selector label="Skin" value={skin} onChange={(v) => setSkin(v as Skin)}
        options={SKINS.map((s) => ({ value: s.value, label: s.label, title: s.title }))} />
    </div>
  )
}

/** The application shell: top bar, the instrument rack, and drawers for setup and the request log. */
export function BenchLayout(props: Props) {
  return <SkinProvider><BenchShell {...props} /></SkinProvider>
}

function BenchShell({ instruments, registry, apiBase, loading, error, wsDiag, onConfigUpdated, onOpenRecording, onOpenMetrics, onOpenDocs }: Props) {
  const panels = useMemo(() => panelsOf(instruments), [instruments])
  const [saved, setLayout] = useState<Layout>(() => loadLayout())
  const [arranging, setArranging] = useState(false)
  const [drawer, setDrawer] = useState<DrawerKind>(null)
  const [autoAdd, setAutoAdd] = useState(false)

  // The layout follows the configured instruments: new ones appear, removed ones go.
  const layout = useMemo(() => (loading ? saved : reconcile(saved, panels)), [saved, panels, loading])
  useEffect(() => { if (layout !== EMPTY_LAYOUT) saveLayout(layout) }, [layout])

  const detached = useDetachedKeys()
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
    <div className="bench">
      <header className="bench-top">
        <span className="bench-brand">BenchMesh</span>
        <span className="bench-pill" data-ok={apiOk}><i />{apiOk ? 'API' : error ? 'API unreachable' : 'connecting…'}</span>
        <span className="bench-pill" data-ok={wsDiag.ok}><i />{wsDiag.ok ? 'live' : wsDiag.msg}</span>
        <div className="bench-tools">
          <SkinSwitch />
          <nav className="bench-actions" aria-label="Main">
            <button type="button" className="bench-btn" aria-pressed={arranging} onClick={() => setArranging((a) => !a)} disabled={!instruments.length}>
              {arranging ? 'Done' : 'Arrange'}
            </button>
            <button type="button" className="bench-btn" onClick={() => setDrawer('instruments')}>Instruments</button>
            <button type="button" className="bench-btn" onClick={onOpenRecording}>Recording</button>
            <button type="button" className="bench-btn" onClick={() => setDrawer('log')}>Log</button>
            <button type="button" className="bench-btn" onClick={onOpenMetrics}>Metrics</button>
            <button type="button" className="bench-btn" onClick={onOpenDocs}>API</button>
          </nav>
        </div>
      </header>

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
              detached={detached} onDetach={detach} />
          </>
        )}
      </main>

      {drawer === 'instruments' && (
        <Drawer title="Instruments" onClose={() => setDrawer(null)}>
          <SettingsView apiBase={apiBase} onConfigUpdated={onConfigUpdated} autoAddInstrument={autoAdd} onAutoAddComplete={() => setAutoAdd(false)} />
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
