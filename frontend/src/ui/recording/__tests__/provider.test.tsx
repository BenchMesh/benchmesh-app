import React, { useEffect } from 'react'
import { describe, it, expect, beforeEach } from 'vitest'
import { act, render, screen, waitFor } from '@testing-library/react'
import { RecordingsProvider, seriesOf, useRecordings } from '../RecordingsProvider'
import { appendSamples, applyMessage, EMPTY_STATE } from '../store'
import { fakeClient, MockSocket, recording, series, T0 } from './fakes'

beforeEach(() => {
  MockSocket.sockets = []
  ;(globalThis as any).WebSocket = MockSocket
})

function Probe() {
  const rec = useRecordings()!
  return (
    <div>
      <span data-testid="names">{rec.recordings.map((r) => `${r.name}:${r.state}:${r.series.length}`).join(',')}</span>
      <span data-testid="live">{JSON.stringify(rec.live)}</span>
      <span data-testid="epoch">{rec.epoch}</span>
      <span data-testid="conn">{String(rec.connected)}</span>
    </div>
  )
}

describe('RecordingsProvider', () => {
  it('fills in from the REST list, then follows the WebSocket', async () => {
    const client = fakeClient([recording({ id: 1, name: 'A' })])
    render(<RecordingsProvider client={client}><Probe /></RecordingsProvider>)
    await waitFor(() => expect(screen.getByTestId('names')).toHaveTextContent('A:recording:0'))
    await waitFor(() => expect(screen.getByTestId('conn')).toHaveTextContent('true'))
    const ws = MockSocket.last()
    expect(ws.url).toMatch(/^ws:\/\/.*\/ws\/recordings$/)

    act(() => ws.emit({ type: 'snapshot', recordings: [recording({ id: 1, name: 'A' }), recording({ id: 2, name: 'B', started_at: T0 + 5000 })] }))
    expect(screen.getByTestId('names')).toHaveTextContent('B:recording:0,A:recording:0') // newest first
    expect(screen.getByTestId('epoch')).toHaveTextContent('1')

    act(() => ws.emit({ type: 'recording', recording: recording({ id: 1, name: 'A', state: 'paused', series: [series()] }) }))
    expect(screen.getByTestId('names')).toHaveTextContent('B:recording:0,A:paused:1')

    act(() => ws.emit({ type: 'samples', recording_id: 1, t: T0 + 1000, values: { 11: 4.93 } }))
    act(() => ws.emit({ type: 'samples', recording_id: 1, t: T0 + 2000, values: { 11: null, 12: 0.4 } }))
    expect(JSON.parse(screen.getByTestId('live').textContent!)).toEqual({ 1: { t: [T0 + 1000, T0 + 2000], v: { 11: [4.93, null], 12: [null, 0.4] } } })

    act(() => ws.emit({ type: 'deleted', recording_id: 1 }))
    expect(screen.getByTestId('names')).toHaveTextContent(/^B:recording:0$/)
    expect(screen.getByTestId('live')).toHaveTextContent('{}')

    // not a recordings message: ignored
    act(() => ws.emit({ 'tenma-72-2540': { IDN: 'x' } }))
    expect(screen.getByTestId('names')).toHaveTextContent(/^B:recording:0$/)
  })

  it('applies action replies at once', async () => {
    const client = fakeClient([])
    const box: { api: ReturnType<typeof useRecordings> } = { api: null }
    function Grab() { const a = useRecordings(); useEffect(() => { box.api = a }); return <Probe /> }
    render(<RecordingsProvider client={client}><Grab /></RecordingsProvider>)
    await act(async () => { await box.api!.create({ name: 'New', interval_s: 2 }) })
    expect(screen.getByTestId('names')).toHaveTextContent('New:recording:0')
    const id = box.api!.recordings[0].id
    await act(async () => { await box.api!.addSeries(id, { device_id: 'd', class: 'DMM', channel: 1, field: 'MEAS', label: 'D · MEAS', unit: 'V' }) })
    expect(screen.getByTestId('names')).toHaveTextContent('New:recording:1')
    const sid = box.api!.recordings[0].series[0].id
    await act(async () => { await box.api!.removeSeries(id, sid) })
    expect(box.api!.recordings[0].series[0].removed_at).not.toBeNull()
    await act(async () => { await box.api!.stop(id) })
    expect(screen.getByTestId('names')).toHaveTextContent('New:stopped:1')
    await act(async () => { await box.api!.remove(id) })
    expect(screen.getByTestId('names')).toHaveTextContent(/^$/)
  })
})

describe('store', () => {
  it('ignores repeated ticks and trims to capacity', () => {
    let b = appendSamples(undefined, 1000, { 1: 1 }, 10)
    b = appendSamples(b, 1000, { 1: 2 }, 10)
    expect(b.t).toEqual([1000])
    for (let i = 2; i <= 12; i++) b = appendSamples(b, i * 1000, { 1: i }, 10)
    expect(b.t.length).toBe(10)
    expect(b.t[0]).toBe(3000)
    expect(b.v[1][0]).toBe(3)
  })

  it('a snapshot replaces the list and drops live buffers of gone recordings', () => {
    let s = applyMessage(EMPTY_STATE, { type: 'snapshot', recordings: [recording({ id: 1 })] })
    s = applyMessage(s, { type: 'samples', recording_id: 1, t: 1000, values: { 11: 1 } })
    s = applyMessage(s, { type: 'snapshot', recordings: [recording({ id: 2 })] })
    expect(s.recordings.map((r) => r.id)).toEqual([2])
    expect(s.live).toEqual({})
  })

  it('finds the recordings a signal is active in', () => {
    const rs = [recording({ id: 1, series: [series({ id: 5 })] }), recording({ id: 2, series: [series({ id: 6, removed_at: T0 })] })]
    const m = seriesOf(rs, { device_id: 'tenma-72-2540', class: 'PSU', channel: 1, field: 'VOUT' })
    expect([...m.keys()]).toEqual([1])
  })
})
