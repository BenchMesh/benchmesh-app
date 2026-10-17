import React, { useEffect, useLayoutEffect, useRef, useState } from 'react'
import '../kit/kit.css'
import './bench.css'
import { useApiBase, useInstruments, useRegistrySocket } from '../data'
import { getClassDescription } from '../instrumentClasses'
import { closeWindow, isDesktop, resizeWindow, setWindowOnTop, startWindowDrag, startWindowResize } from '../desktop'
import { InstrumentPanel } from './InstrumentPanel'
import { clearDetached, useDetachedHeartbeat } from './detach'

/** `instrumentId:CLASS` → its parts (ids may contain colons). */
export function splitPanelKey(key: string): [string, string] {
  const i = key.lastIndexOf(':')
  return i < 0 ? [key, ''] : [key.slice(0, i), key.slice(i + 1)]
}

/**
 * One instrument panel in its own undecorated window (`/ui/?panel=<key>`). The panel header is
 * the title bar: drag it to move the window, fold it to the header line, keep it on top, or
 * close it - the panel then returns to the bench. While this window lives, the bench leaves the
 * panel out (heartbeat), so the instrument is never drawn twice.
 */
export function PanelWindow({ panelKey }: { panelKey: string }) {
  const apiBase = useApiBase()
  const { data: instruments, loading } = useInstruments(apiBase, 0)
  const { registry } = useRegistrySocket(apiBase)
  useDetachedHeartbeat(panelKey)
  const [instrumentId, klass] = splitPanelKey(panelKey)
  const instrument = instruments.find((i) => i.id === instrumentId)
  const [folded, setFolded] = useState(false)
  const [onTop, setOnTop] = useState(false)
  const unfoldedSize = useRef<{ w: number; h: number } | null>(null)
  const root = useRef<HTMLDivElement>(null)

  useEffect(() => {
    document.title = `${getClassDescription(klass)} · ${instrument?.name ?? instrumentId}`
  }, [klass, instrument?.name, instrumentId])

  // folded: the window shrinks to the header line; unfolded: back to its previous size
  useLayoutEffect(() => {
    if (folded) {
      unfoldedSize.current = { w: window.innerWidth, h: window.innerHeight }
      const h = root.current?.querySelector('article')?.getBoundingClientRect().height ?? 48
      void resizeWindow(window.innerWidth, Math.ceil(h))
    } else if (unfoldedSize.current) {
      void resizeWindow(unfoldedSize.current.w, unfoldedSize.current.h)
      unfoldedSize.current = null
    }
  }, [folded])

  const toggleOnTop = () => {
    const next = !onTop
    setWindowOnTop(next).then(() => setOnTop(next)).catch(() => {})
  }

  return (
    <div ref={root} className={`bench bench-window${folded ? ' bench-window--folded' : ''}`}>
      {instrument ? (
        <InstrumentPanel instrument={instrument} klass={klass} registry={registry} win={{
          folded,
          onFold: () => setFolded((f) => !f),
          onTop: isDesktop() ? { on: onTop, toggle: toggleOnTop } : undefined,
          // deregister first: the bench shows the panel again at once, without waiting for
          // the page-hide event or the heartbeat to go stale
          onClose: () => { clearDetached(panelKey); closeWindow() },
          onDragStart: startWindowDrag,
        }} />
      ) : (
        <div className="bench-window__msg">
          <span>{loading ? 'Connecting…' : `${panelKey} is no longer configured.`}</span>
          <button type="button" className="bench-btn" onClick={closeWindow}>Close</button>
        </div>
      )}
      {!folded && isDesktop() && (
        <button type="button" className="bench-window__grip" aria-label="Resize window" title="Resize"
          onPointerDown={(e) => { if (e.button === 0) startWindowResize() }} />
      )}
    </div>
  )
}
