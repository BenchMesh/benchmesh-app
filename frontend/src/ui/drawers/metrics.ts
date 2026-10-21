/**
 * Connection metrics (`/ws/metrics`, a summary per instrument every 30 s): how busy each
 * instrument's link is and how fast the API answers. Pure helpers: what the Metrics tab shows.
 */

export interface DeviceMetric {
  device_id: string
  window_duration_s: number
  utilization_pct: number
  qps: number
  api_request_count: number
  api_latency_p95_ms: number | null
  api_latency_p99_ms: number | null
  avg_queue_depth: number
  avg_poll_duration_ms: number
  total_operations: number
}

/** `ok` / `warn` colour a value against its limit; `null` is a plain value. */
export type MetricState = 'ok' | 'warn' | null

export interface MetricRow { label: string; value: string; state: MetricState; hint: string }

const judged = (ok: boolean): MetricState => (ok ? 'ok' : 'warn')

/** The rows of one instrument, in reading order; values not measured in the window are left out. */
export function metricRows(m: DeviceMetric): MetricRow[] {
  const rows: MetricRow[] = [
    { label: 'Link busy', value: `${m.utilization_pct.toFixed(1)} %`, state: judged(m.utilization_pct < 80), hint: 'Share of the window the connection was exchanging data (below 80 % leaves room)' },
    { label: 'Exchanges', value: `${m.qps.toFixed(2)} /s`, state: null, hint: 'Commands and queries per second' },
    { label: 'Operations', value: String(m.total_operations), state: null, hint: `In the last ${m.window_duration_s.toFixed(0)} s` },
    { label: 'API requests', value: String(m.api_request_count), state: null, hint: `In the last ${m.window_duration_s.toFixed(0)} s` },
  ]
  if (m.api_latency_p95_ms != null) rows.push({ label: 'API latency p95', value: `${m.api_latency_p95_ms.toFixed(1)} ms`, state: judged(m.api_latency_p95_ms < 50), hint: '95 % of the API requests were answered within this' })
  if (m.api_latency_p99_ms != null) rows.push({ label: 'API latency p99', value: `${m.api_latency_p99_ms.toFixed(1)} ms`, state: judged(m.api_latency_p99_ms < 100), hint: '99 % of the API requests were answered within this' })
  if (m.avg_queue_depth > 0) rows.push({ label: 'Queue depth', value: m.avg_queue_depth.toFixed(2), state: judged(m.avg_queue_depth < 2), hint: 'Requests waiting for the instrument, on average' })
  if (m.avg_poll_duration_ms > 0) rows.push({ label: 'Poll duration', value: `${m.avg_poll_duration_ms.toFixed(0)} ms`, state: null, hint: 'Time for one round of status queries, on average' })
  return rows
}
