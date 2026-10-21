/** Metrics tab of the Instruments drawer: per instrument, how busy its connection is (`/ws/metrics`). */
import React, { useEffect, useState } from 'react'
import { wsUrl } from '../../api/base'
import type { Instrument } from '../types'
import { metricRows, type DeviceMetric } from './metrics'

function useMetricsSocket() {
  const [metrics, setMetrics] = useState<Record<string, DeviceMetric>>({})
  const [connected, setConnected] = useState(false)
  useEffect(() => {
    let ws: WebSocket | null = null
    let retry: ReturnType<typeof setTimeout> | undefined
    let live = true
    const connect = () => {
      try {
        ws = new WebSocket(wsUrl('/ws/metrics'))
      } catch {
        retry = setTimeout(connect, 2000)
        return
      }
      ws.onopen = () => setConnected(true)
      ws.onclose = () => {
        setConnected(false)
        if (live) retry = setTimeout(connect, 2000)
      }
      ws.onmessage = (ev) => {
        try { setMetrics(JSON.parse(ev.data)) } catch { /* a broken message: keep the last summary */ }
      }
    }
    connect()
    return () => { live = false; clearTimeout(retry); ws?.close() }
  }, [])
  return { metrics, connected }
}

export function MetricsView({ instruments = [] }: { instruments?: Instrument[] }) {
  const { metrics, connected } = useMetricsSocket()
  const ids = Object.keys(metrics)
  const nameOf = (id: string) => instruments.find((i) => i.id === id)?.name
  return (
    <section className="bench-page" aria-label="Metrics" data-testid="metrics-view">
      <h3 className="settings-view__section-title">Metrics</h3>
      <p className="bench-hint">
        How busy each instrument's connection is and how fast the API answers, summed up every 30 s.
      </p>
      <span className="bench-pill" data-ok={connected}><i />{connected ? 'live - updates every 30 s' : 'not connected'}</span>
      {ids.length === 0 ? (
        <p className="bench-hint">No figures yet: they appear once the instruments have been polled for a while.</p>
      ) : ids.map((id) => (
        <div key={id} className="bench-group" role="group" aria-label={nameOf(id) ?? id}>
          <div className="bench-group__row">
            <span className="kit-label">{nameOf(id) ?? id}</span>
            {nameOf(id) && <code className="bench-hint">{id}</code>}
          </div>
          <dl className="bench-metrics">
            {metricRows(metrics[id]).map((r) => (
              <div key={r.label} className="bench-metric" title={r.hint}>
                <dt className="kit-label">{r.label}</dt>
                <dd className="bench-metric__value" data-state={r.state ?? undefined}>{r.value}</dd>
              </div>
            ))}
          </dl>
        </div>
      ))}
    </section>
  )
}
