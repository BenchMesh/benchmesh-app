import React from 'react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { InstrumentPanel } from '../InstrumentPanel'
import { ParkingStrip, presenceText } from '../ParkingStrip'
import { PanelWindow } from '../PanelWindow'
import { RequestLogProvider } from '../../RequestLogContext'
import { RunProvider } from '../../automation/RunProvider'
import { AutomationClient } from '../../automation/client'
import { Instrument } from '../../types'

const tenma: Instrument = { id: 'tenma', name: 'Tenma', IDN: 'TENMA 72-2540 V2.1', classes: [{ class: 'PSU', channels: ['/instruments/PSU/tenma/1'] }] }
const live = (output: string) => ({ tenma: { IDN: 'TENMA 72-2540 V2.1', PSU: { status_ch1: { OUTPUT: output } } } })

let calls: { url: string; method: string }[] = []
let reply: { status: number; body: unknown } = { status: 200, body: { id: 'tenma', parked: true } }
let wsRegistry: unknown = live('OFF')

class MockWebSocket {
  onopen: any; onmessage: any; onclose: any; onerror: any
  constructor() { setTimeout(() => this.onmessage?.({ data: JSON.stringify(wsRegistry) }), 0) }
  close() {}
}

beforeEach(() => {
  localStorage.clear()
  calls = []
  reply = { status: 200, body: { id: 'tenma', parked: true } }
  wsRegistry = live('OFF')
  ;(global as any).WebSocket = MockWebSocket
  global.fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const u = String(url)
    if (u.includes('/park')) {
      calls.push({ url: u, method: init?.method ?? 'GET' })
      return new Response(JSON.stringify(reply.body), { status: reply.status, headers: { 'Content-Type': 'application/json' } })
    }
    const body = u.endsWith('/instruments') ? [tenma] : {}
    return new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } })
  }) as any
})

const wrap = (ui: React.ReactElement) => render(<RequestLogProvider>{ui}</RequestLogProvider>)
const panel = (registry: any, onParked = vi.fn()) =>
  <InstrumentPanel instrument={tenma} klass="PSU" registry={registry} win={{ folded: false, onFold: vi.fn(), onParked }} />

describe('park control in the panel header', () => {
  it('sits before Fold and parks the instrument', async () => {
    const onParked = vi.fn()
    wrap(panel(live('OFF'), onParked))
    const buttons = screen.getAllByRole('button').map((b) => b.getAttribute('aria-label'))
    expect(buttons.indexOf('Park instrument')).toBe(buttons.indexOf('Fold') - 1)
    fireEvent.click(screen.getByRole('button', { name: 'Park instrument' }))
    await waitFor(() => expect(onParked).toHaveBeenCalled())
    expect(calls).toEqual([{ url: expect.stringMatching(/\/devices\/tenma\/park$/), method: 'POST' }])
  })

  it('asks first while an output is on', async () => {
    wrap(panel(live('ON')))
    fireEvent.click(screen.getByRole('button', { name: 'Park instrument' }))
    expect(screen.getByRole('alert').textContent).toContain('PSU ch1 OUTPUT is on and stays on, unwatched, while parked')
    expect(calls).toEqual([])
    fireEvent.click(screen.getByRole('button', { name: 'Park anyway' }))
    await waitFor(() => expect(calls).toHaveLength(1))
  })

  it("shows the server's refusal", async () => {
    reply = { status: 409, body: { detail: 'tenma is used by the running flow "Sweep": stop the flow before parking it' } }
    wrap(panel(live('OFF')))
    fireEvent.click(screen.getByRole('button', { name: 'Park instrument' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('used by the running flow "Sweep"')
  })

  it('is disabled while the running flow uses the instrument', async () => {
    const client = { runStatus: async () => ({ id: 1, state: 'running', flow: { id: null, name: 'Sweep' }, devices: ['tenma'], events: [] }) } as unknown as AutomationClient
    wrap(<RunProvider client={client}>{panel(live('OFF'))}</RunProvider>)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Park instrument' })).toBeDisabled())
    expect(screen.getByRole('button', { name: 'Park instrument' }).title).toContain('Used by the running flow "Sweep"')
  })
})

describe('parking strip', () => {
  const parked = { ...tenma, parked: true }

  it('is not there while nothing is parked', () => {
    const { container } = wrap(<ParkingStrip parked={[]} registry={{}} />)
    expect(container.querySelector('.bench-parking')).toBeNull()
  })

  it('shows name and presence closed, details and Unpark open', async () => {
    reply = { status: 200, body: { id: 'tenma', parked: false } }
    wrap(<ParkingStrip parked={[parked]} registry={{ tenma: { parked: { present: false, detail: 'not attached: no serial port matches usb 0416:5011' } } }} />)
    const tab = screen.getByRole('button', { name: 'Tenma, parked, not found' })
    expect(tab.querySelector('i')).toHaveAttribute('data-present', 'false')
    fireEvent.click(tab)
    const sheet = screen.getByRole('dialog', { name: 'Parking' })
    expect(sheet).toHaveTextContent('Not found: not attached: no serial port matches usb 0416:5011')
    fireEvent.click(screen.getByRole('button', { name: 'Unpark' }))
    await waitFor(() => expect(calls).toEqual([{ url: expect.stringMatching(/\/devices\/tenma\/park$/), method: 'DELETE' }]))
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('dialog', { name: 'Parking' })).toBeNull()
  })

  it('words presence for every state', () => {
    expect(presenceText({ present: true, detail: '/dev/ttyUSB0' })).toBe('Attached at /dev/ttyUSB0')
    expect(presenceText({ present: false, detail: 'unplugged' })).toBe('Not found: unplugged')
    expect(presenceText({ present: null, detail: '' })).toBe('Checking whether it is attached…')
  })
})

describe('own window of a parked instrument', () => {
  it('closes itself', async () => {
    wsRegistry = { tenma: { parked: { present: true, detail: 'sim' } } }
    const close = vi.spyOn(window, 'close').mockImplementation(() => {})
    wrap(<PanelWindow panelKey="tenma:PSU" />)
    await waitFor(() => expect(close).toHaveBeenCalled())
  })
})
