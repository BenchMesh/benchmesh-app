import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, act, within } from '@testing-library/react'
import { metricRows, type DeviceMetric } from '../metrics'
import { MetricsView } from '../MetricsView'

const m = (o: Partial<DeviceMetric> = {}): DeviceMetric => ({
  device_id: 'rigol-dho804', window_duration_s: 30, utilization_pct: 12.5, qps: 2.5, api_request_count: 5,
  api_latency_p95_ms: 11.2, api_latency_p99_ms: 140, avg_queue_depth: 0, avg_poll_duration_ms: 120.4, total_operations: 75, ...o,
})

describe('metric rows', () => {
  it('formats and judges the figures, leaving out what was not measured', () => {
    const rows = metricRows(m())
    expect(rows.map((r) => [r.label, r.value, r.state])).toEqual([
      ['Link busy', '12.5 %', 'ok'], ['Exchanges', '2.50 /s', null], ['Operations', '75', null], ['API requests', '5', null],
      ['API latency p95', '11.2 ms', 'ok'], ['API latency p99', '140.0 ms', 'warn'], ['Poll duration', '120 ms', null],
    ])
    expect(rows[2].hint).toBe('In the last 30 s')
  })
  it('a busy link and a queue are warnings', () => {
    const rows = metricRows(m({ utilization_pct: 91, avg_queue_depth: 3, api_latency_p95_ms: null, api_latency_p99_ms: null }))
    expect(rows.find((r) => r.label === 'Link busy')?.state).toBe('warn')
    expect(rows.find((r) => r.label === 'Queue depth')).toMatchObject({ value: '3.00', state: 'warn' })
    expect(rows.some((r) => r.label.startsWith('API latency'))).toBe(false)
  })
})

describe('MetricsView', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('shows a group per instrument, named from the configuration', () => {
    const sockets: any[] = []
    vi.stubGlobal('WebSocket', class { onopen?: () => void; onmessage?: (e: { data: string }) => void; onclose?: () => void; constructor() { sockets.push(this) } close() {} })
    render(<MetricsView instruments={[{ id: 'rigol-dho804', name: 'RIGOL DHO804', classes: [] }]} />)
    expect(screen.getByText(/No figures yet/)).toBeInTheDocument()
    act(() => { sockets[0].onopen(); sockets[0].onmessage({ data: JSON.stringify({ 'rigol-dho804': m(), 'other': m({ device_id: 'other' }) }) }) })
    const rigol = screen.getByRole('group', { name: 'RIGOL DHO804' })
    expect(within(rigol).getByText('rigol-dho804')).toBeInTheDocument()
    expect(within(rigol).getByText('12.5 %')).toHaveAttribute('data-state', 'ok')
    expect(screen.getByRole('group', { name: 'other' })).toBeInTheDocument()
    expect(screen.getByText('live - updates every 30 s')).toBeInTheDocument()
  })
})
