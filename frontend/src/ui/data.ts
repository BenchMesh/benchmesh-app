/** Data shared by every window: the instrument list (polled, ETag) and the live registry (WebSocket). */
import { useEffect, useMemo, useState } from 'react'
import { Instrument } from './types'
import { apiBaseUrl, wsUrl } from '../api/base'

export function useApiBase() {
  // API is served by FastAPI app; assume same origin during production.
  // For local dev, we can compute from window.location.
  return useMemo(() => {
    return apiBaseUrl()
  }, [])
}

export function useInstruments(apiBase: string, refreshTrigger: number) {
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

/** The registry stream: `connecting`, `open` (no data yet), `receiving`, `stale` (nothing for 1 s), `closed` (reconnecting). */
export type StreamState = 'connecting' | 'open' | 'receiving' | 'stale' | 'closed'

export interface WsDiag { state: StreamState, last: number | null }

export function useRegistrySocket(apiBase: string) {
  const [registry, setRegistry] = useState<any>({})
  const [wsDiag, setWsDiag] = useState<WsDiag>({ state: 'connecting', last: null })

  useEffect(() => {
    let ws: WebSocket | null = null
    let aliveTimer: any
    function connect() {
      const url = wsUrl(`/ws/registry`)
      try {
        ws = new WebSocket(url)
      } catch {
        setWsDiag((d) => ({ ...d, state: 'closed' }))
        setTimeout(connect, 1000)
        return
      }
      setWsDiag((d) => ({ ...d, state: 'connecting' }))
      ws.onopen = () => setWsDiag((d) => ({ ...d, state: 'open' }))
      ws.onclose = () => {
        setWsDiag((d) => ({ ...d, state: 'closed' }))
        setTimeout(connect, 1000)
      }
      ws.onerror = () => setWsDiag((d) => ({ ...d, state: 'closed' }))
      ws.onmessage = (ev) => {
        try {
          const obj = JSON.parse(ev.data)
          setRegistry(obj)
          setWsDiag({ state: 'receiving', last: Date.now() })
          if (aliveTimer) clearTimeout(aliveTimer)
          // no update for 1 s: what the panels show may be old
          aliveTimer = setTimeout(() => setWsDiag((d) => ({ ...d, state: 'stale' })), 1000)
        } catch {}
      }
    }
    connect()
    return () => { if (ws) ws.close(); if (aliveTimer) clearTimeout(aliveTimer) }
  }, [apiBase])

  return { registry, wsDiag }
}
