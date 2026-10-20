import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { DetectView } from '../DetectView'
import { detectSummary } from '../detect'

const base = { kind: 'serial', usb: null, configured_as: null, locked: false, instrument: null, match: null, certainty: null, error: null }
const reply = (body: unknown, status = 200) => Promise.resolve(new Response(JSON.stringify(body), { status }))

afterEach(() => vi.restoreAllMocks())

describe('detectSummary', () => {
  it('counts finished ports and seconds', () => {
    expect(detectSummary({ elapsed_ms: 14_400, ports: [
      { ...base, port: 'a', progress: { state: 'done' } }, { ...base, port: 'b', progress: { state: 'probing', tried: 2, total: 9 } },
    ] as any })).toBe('1 of 2 ports done · 14 s')
  })
})

describe('Detect progress', () => {
  it('shows the probe on the wire per port, then the results', async () => {
    const running = { running: true, elapsed_ms: 1200, ports: [
      { ...base, port: '/dev/ttyUSB1', progress: { state: 'probing', tried: 2, total: 9, current: '*IDN? · 115200 8N1 · owon_xdm and 3 more' } },
      { ...base, port: '/dev/usbtmc4', kind: 'usbtmc', progress: { state: 'queued' } },
    ] }
    const done = { running: false, elapsed_ms: 3100, ports: [
      { ...base, port: '/dev/ttyUSB1', error: 'answered "ACME,X", which no definition knows yet (tried 9 probes for 41 definitions)', progress: { state: 'done' } },
      { ...base, port: '/dev/usbtmc4', kind: 'usbtmc', progress: { state: 'done' },
        instrument: { driver: 'owon_dge', model: 'DGE2070', vendor: 'OWON', family: 'DGE', idn: 'OWON,DGE2070,1,2', classes: ['AWG'] },
        match: { usb: { vid: '5345', pid: '1235', serial: '25130086' }, idn_serial: '1' }, certainty: 'instrument' },
    ] }
    let gets = 0
    vi.spyOn(globalThis, 'fetch').mockImplementation((url, init) => {
      if (String(url).endsWith('/devices/detect') && init?.method === 'POST') return reply(running, 202)
      if (String(url).endsWith('/devices/detect')) return reply(gets++ === 0 ? running : done)
      return reply([])
    })
    render(<DetectView apiBase="/api" configured={[]} onChanged={() => {}} />)
    fireEvent.click(screen.getByTestId('detect-btn'))
    expect(await screen.findByText(/3\/9 \*IDN\? · 115200 8N1 · owon_xdm and 3 more/)).toBeTruthy()
    expect(screen.getByTestId('detect-summary').textContent).toBe('0 of 2 ports done · 1 s')
    expect(screen.getByText('waiting…')).toBeTruthy()
    expect(await screen.findByText(/which no definition knows yet/, {}, { timeout: 3000 })).toBeTruthy()
    expect(screen.getByTestId('detect-summary').textContent).toBe('2 of 2 ports done · 3 s')
    expect(screen.queryByTestId('detect-progress')).toBeNull()
    expect(screen.getByRole('button', { name: 'Add' })).toBeTruthy()
  })
})
