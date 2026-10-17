import React, { useEffect, useMemo, useState, lazy, Suspense } from 'react'
import { Instrument } from './types'
import { RequestLogProvider } from './RequestLogContext'
import { BenchLayout } from './bench/BenchLayout'
import { DisclaimerModal } from './DisclaimerModal'
import { apiBaseUrl, wsUrl } from '../api/base'

// Lazy load modal components to reduce initial bundle size
const DocsViewer = lazy(() => import('./DocsViewer').then(m => ({ default: m.DocsViewer })))
const MetricsViewer = lazy(() => import('./MetricsViewer').then(m => ({ default: m.MetricsViewer })))
const RecordingModal = lazy(() => import('./recording/RecordingModal').then(m => ({ default: m.RecordingModal })))

function useApiBase() {
  // API is served by FastAPI app; assume same origin during production.
  // For local dev, we can compute from window.location.
  return useMemo(() => {
    return apiBaseUrl()
  }, [])
}

function useInstruments(apiBase: string, refreshTrigger: number) {
  const [data, setData] = useState<Instrument[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    let timer: any
    let lastETag: string | null = null

    async function load() {
      let ok = false
      try {
        // Send If-None-Match header if we have an ETag
        const headers: HeadersInit = {}
        if (lastETag) {
          headers['If-None-Match'] = lastETag
        }

        const resp = await fetch(`${apiBase}/instruments`, { headers })

        // 304 Not Modified - data hasn't changed, skip update
        if (resp.status === 304) {
          if (!cancelled) {
            setError(null)
            ok = true
          }
          return
        }

        if (!resp.ok) throw new Error(`HTTP ${resp.status}`)

        const json = await resp.json()
        const newETag = resp.headers.get('ETag')

        if (!cancelled) {
          // Only update state if data actually changed (deep equality check)
          const dataChanged = JSON.stringify(data) !== JSON.stringify(json)
          if (dataChanged) {
            setData(json)
          }
          setError(null)
          if (newETag) {
            lastETag = newETag
          }
          ok = true
        }
      } catch (e: any) {
        if (!cancelled) setError(String(e))
      } finally {
        if (!cancelled) {
          setLoading(false)
          // Normal: poll every 5s; on failure: retry after 1s
          const delay = ok ? 5000 : 1000
          timer = setTimeout(load, delay)
        }
      }
    }
    load()
    return () => { cancelled = true; if (timer) clearTimeout(timer) }
  }, [apiBase, refreshTrigger])

  return { data, loading, error }
}

function useRegistrySocket(apiBase: string) {
  const [registry, setRegistry] = useState<any>({})
  const [wsDiag, setWsDiag] = useState<{ ok: boolean, msg: string, last: number | null }>({ ok: false, msg: 'connecting…', last: null })

  useEffect(() => {
    let ws: WebSocket | null = null
    let aliveTimer: any
    function connect() {
      const url = wsUrl(`/ws/registry`)
      try {
        ws = new WebSocket(url)
      } catch (e: any) {
        setWsDiag({ ok: false, msg: 'ws create failed', last: wsDiag.last })
        setTimeout(connect, 1000)
        return
      }
      setWsDiag({ ok: false, msg: 'connecting…', last: wsDiag.last })
      ws.onopen = () => setWsDiag((d) => ({ ...d, ok: true, msg: 'connected' }))
      ws.onclose = () => {
        setWsDiag((d) => ({ ...d, ok: false, msg: 'disconnected' }))
        setTimeout(connect, 1000)
      }
      ws.onerror = () => setWsDiag((d) => ({ ...d, ok: false, msg: 'error' }))
      ws.onmessage = (ev) => {
        try {
          const obj = JSON.parse(ev.data)
          setRegistry(obj)
          const now = Date.now()
          setWsDiag({ ok: true, msg: 'receiving', last: now })
          if (aliveTimer) clearTimeout(aliveTimer)
          aliveTimer = setTimeout(() => {
            // if no updates in 1s, mark as stale
            setWsDiag((d) => ({ ...d, ok: false, msg: 'no data >1s' }))
          }, 1000)
        } catch {}
      }
    }
    connect()
    return () => { if (ws) ws.close(); if (aliveTimer) clearTimeout(aliveTimer) }
  }, [apiBase])

  return { registry, wsDiag }
}

export default function App() {
  const apiBase = useApiBase()
  const [refreshTrigger, setRefreshTrigger] = useState(0)
  const { data: instruments, loading, error } = useInstruments(apiBase, refreshTrigger)
  const { registry, wsDiag } = useRegistrySocket(apiBase)
  // ?openDocs=true opens the API docs directly
  const [docsModalOpen, setDocsModalOpen] = useState(() => new URLSearchParams(window.location.search).get('openDocs') === 'true')
  const [metricsModalOpen, setMetricsModalOpen] = useState(false)
  const [recordingModalOpen, setRecordingModalOpen] = useState(false)

  // Disclaimer modal state - shows on first app load if not previously accepted
  const [disclaimerOpen, setDisclaimerOpen] = useState(() => {
    try {
      return localStorage.getItem('benchmesh:disclaimer-accepted') !== 'true'
    } catch {
      return true
    }
  })

  function handleConfigUpdated() {
    setRefreshTrigger(prev => prev + 1)
  }

  function handleDisclaimerAccept(dontShowAgain: boolean) {
    setDisclaimerOpen(false)
    if (dontShowAgain) {
      try { localStorage.setItem('benchmesh:disclaimer-accepted', 'true') } catch { /* storage unavailable */ }
    }
  }

  return (
    <RequestLogProvider>
        <BenchLayout
          instruments={instruments || []}
          registry={registry}
          apiBase={apiBase}
          loading={loading}
          error={error}
          wsDiag={wsDiag}
          onConfigUpdated={handleConfigUpdated}
          onOpenRecording={() => setRecordingModalOpen(true)}
          onOpenMetrics={() => setMetricsModalOpen(true)}
          onOpenDocs={() => setDocsModalOpen(true)}
        />

        <Suspense fallback={<div className="modal-loading">Loading...</div>}>
          <RecordingModal
            isOpen={recordingModalOpen}
            onClose={() => setRecordingModalOpen(false)}
            apiBase={apiBase}
            instruments={instruments || []}
          />
        </Suspense>

        {docsModalOpen && (
          <Suspense fallback={<div className="modal-loading">Loading...</div>}>
            <DocsViewer onClose={() => setDocsModalOpen(false)} apiBase={apiBase} />
          </Suspense>
        )}

        {metricsModalOpen && (
          <Suspense fallback={<div className="modal-loading">Loading...</div>}>
            <MetricsViewer onClose={() => setMetricsModalOpen(false)} apiBase={apiBase} />
          </Suspense>
        )}

        {/* Disclaimer modal - highest priority, shown on first app load */}
        <DisclaimerModal isOpen={disclaimerOpen} onAccept={handleDisclaimerAccept} />
    </RequestLogProvider>
  )
}
