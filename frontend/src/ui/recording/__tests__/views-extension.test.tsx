/** The dock and the drawer offer the views a distribution adds and hand them the recording's data. */
import React from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { RecordingViewProps } from '../../../extensions/contract'
import { RecordingsProvider } from '../RecordingsProvider'
import { DOCK_STORAGE_KEY, RecorderDock } from '../RecorderDock'
import { RecordingsDrawer } from '../RecordingsDrawer'
import { fakeClient, MockSocket, recording, series } from './fakes'

const seen: RecordingViewProps[] = []
vi.mock('../../../extensions', () => ({
  extensions: {
    root: [],
    systemGroups: [],
    recordingViews: [{
      id: 'lanes', label: 'Lanes',
      component: (p: RecordingViewProps) => { seen.push(p); return <div role="img" aria-label={`Lanes of ${p.recording.name}`} /> },
    }],
  },
}))

beforeEach(() => {
  localStorage.clear()
  seen.length = 0
  MockSocket.sockets = []
  ;(globalThis as any).WebSocket = MockSocket
})

describe('recording views from an extension', () => {
  it('the dock switches its live chart to the view', async () => {
    localStorage.setItem(DOCK_STORAGE_KEY, JSON.stringify({ expanded: true, height: 240, selected: 1 }))
    render(<RecordingsProvider client={fakeClient([recording({ series: [series()] })])}><RecorderDock /></RecordingsProvider>)
    fireEvent.click(await screen.findByRole('button', { name: 'Lanes' }))
    expect(await screen.findByRole('img', { name: 'Lanes of PSU ramp' })).toBeInTheDocument()
    const p = seen.at(-1)!
    expect(p.live).toBe(true)
    expect(p.series.map((s) => s.label)).toEqual(['Tenma · VOUT'])
    expect(p.min).toBeTypeOf('number')
  })

  it('the drawer shows a recording in the view', async () => {
    const fc = fakeClient([recording({ state: 'stopped', series: [series()] })])
    const data = fc.data
    render(<RecordingsProvider client={fc}><RecordingsDrawer /></RecordingsProvider>)
    fireEvent.click(await screen.findByRole('button', { name: 'Open PSU ramp' }))
    fireEvent.click(screen.getByRole('button', { name: 'Lanes' }))
    await waitFor(() => expect(seen.at(-1)?.live).toBe(false))
    expect(screen.getByRole('img', { name: 'Lanes of PSU ramp' })).toBeInTheDocument()
    // a range the view shows is loaded at full resolution, like the chart's zoom
    const client = (seen.at(-1)!.onRange)!
    act(() => client(1000, 2000))
    await waitFor(() => expect(data).toHaveBeenCalledWith(1, { from: 1000, to: 2000, max_points: 2000 }), { timeout: 2000 })
  })
})
