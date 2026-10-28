import React from 'react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { InstrumentPanel } from '../InstrumentPanel'
import { PanelWindow, splitPanelKey } from '../PanelWindow'
import { detachedKeys } from '../detach'
import { RequestLogProvider } from '../../RequestLogContext'

const tenma = { id: 'tenma-72-2540', name: 'Tenma', IDN: 'TENMA 72-2540 V2.1', classes: [{ class: 'PSU', channels: ['/instruments/PSU/tenma-72-2540/1'] }] }
const registry = { 'tenma-72-2540': { IDN: 'TENMA 72-2540 V2.1', PSU: { status_ch1: { OUTPUT: 'OFF', REMOTE: 'ON' } } } }

class MockWebSocket {
  onopen: any; onmessage: any; onclose: any; onerror: any
  constructor() { setTimeout(() => this.onmessage?.({ data: JSON.stringify(registry) }), 0) }
  close() {}
}

beforeEach(() => {
  localStorage.clear()
  ;(global as any).WebSocket = MockWebSocket
  global.fetch = vi.fn(async (url: string) => {
    const body = String(url).endsWith('/instruments') ? [tenma] : {}
    return new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } })
  }) as any
})

const wrap = (ui: React.ReactElement) => render(<RequestLogProvider>{ui}</RequestLogProvider>)

describe('panel header window controls', () => {
  it('folds to the header line and offers detaching', () => {
    const onFold = vi.fn()
    const onDetach = vi.fn()
    const { rerender } = wrap(<InstrumentPanel instrument={tenma} klass="PSU" registry={registry} win={{ folded: false, onFold, onDetach }} />)
    fireEvent.click(screen.getByRole('button', { name: 'Fold' }))
    expect(onFold).toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Open in own window' }))
    expect(onDetach).toHaveBeenCalledWith(expect.objectContaining({ width: expect.any(Number), height: expect.any(Number) }))
    rerender(<RequestLogProvider><InstrumentPanel instrument={tenma} klass="PSU" registry={registry} win={{ folded: true, onFold, onDetach }} /></RequestLogProvider>)
    expect(screen.getByRole('button', { name: 'Unfold' })).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText('Loading…')).toBeNull() // folded: header only
    expect(screen.getByRole('article')).toHaveClass('bench-panel--folded')
  })
})

describe('offline panel', () => {
  it('says why: a silent port is most likely an instrument that is switched off', () => {
    const off = { ...tenma, health: { status: 'dead', is_alive: false, last_error: 'timeout (0 bytes received)' } }
    wrap(<InstrumentPanel instrument={off} klass="PSU" registry={{}} />)
    const label = screen.getByRole('status')
    expect(label).toHaveTextContent('Offline - reconnecting')
    expect(label).toHaveTextContent('No answer. Is it switched on?')
  })
})

describe('PanelWindow (an instrument in its own window)', () => {
  it('splits panel keys at the last colon', () => {
    expect(splitPanelKey('tenma-72-2540:PSU')).toEqual(['tenma-72-2540', 'PSU'])
    expect(splitPanelKey('a:b:OSC')).toEqual(['a:b', 'OSC'])
  })

  it('shows the one panel with fold and close, and keeps it registered as detached', async () => {
    const close = vi.spyOn(window, 'close').mockImplementation(() => {})
    wrap(<PanelWindow panelKey="tenma-72-2540:PSU" />)
    expect(await screen.findByRole('article', { name: 'Power Supply Unit: Tenma' })).toBeInTheDocument()
    expect(detachedKeys()).toEqual(['tenma-72-2540:PSU'])
    expect(screen.getByRole('button', { name: 'Fold' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Open in own window' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Keep on top' })).toBeNull() // browser: no always-on-top
    await waitFor(() => expect(document.title).toBe('Power Supply Unit · Tenma'))
    fireEvent.click(screen.getByRole('button', { name: 'Close window' }))
    expect(close).toHaveBeenCalled()
  })

  it('explains an instrument that is no longer configured', async () => {
    wrap(<PanelWindow panelKey="gone:PSU" />)
    expect(await screen.findByText('gone:PSU is no longer configured.')).toBeInTheDocument()
  })
})

describe('panel header support status', () => {
  it('marks models that still await confirmation, not tested ones', () => {
    const { rerender } = wrap(<InstrumentPanel instrument={{ ...tenma, model: '72-2540', support: 'tested' }} klass="PSU" registry={registry} />)
    expect(screen.queryByTestId('support-awaiting')).toBeNull()
    rerender(<RequestLogProvider><InstrumentPanel instrument={{ ...tenma, model: '72-2545', support: 'awaiting' }} klass="PSU" registry={registry} /></RequestLogProvider>)
    expect(screen.getByTestId('support-awaiting').textContent).toBe('awaiting confirmation')
  })
  it('marks a reported problem: under investigation, or fixed in a newer version', () => {
    const { rerender } = wrap(<InstrumentPanel instrument={{ ...tenma, model: '72-2540', support: 'tested' }} klass="PSU" registry={registry} />)
    expect(screen.queryByTestId('community-issue')).toBeNull()
    rerender(<RequestLogProvider><InstrumentPanel instrument={{ ...tenma, issue: { state: 'investigating', reports: 3 } }} klass="PSU" registry={registry} /></RequestLogProvider>)
    expect(screen.getByTestId('community-issue').textContent).toBe('Issue reported - investigation pending')
    expect(screen.getByTestId('community-issue').getAttribute('title')).toContain('3 user reports')
    rerender(<RequestLogProvider><InstrumentPanel instrument={{ ...tenma, issue: { state: 'fixed', fixed_in: '0.1.20', available: true } }} klass="PSU" registry={registry} /></RequestLogProvider>)
    expect(screen.getByTestId('community-issue').textContent).toBe('Fixed in 0.1.20 - update')
  })
})

describe('instrument console', () => {
  it('opens below the panel from the header and is remembered', () => {
    const { unmount } = wrap(<InstrumentPanel instrument={tenma} klass="PSU" registry={registry} />)
    expect(screen.queryByTestId('wire-console')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Instrument console' }))
    expect(screen.getByTestId('wire-console')).toBeTruthy()
    unmount()
    wrap(<InstrumentPanel instrument={tenma} klass="PSU" registry={registry} />)
    expect(screen.getByTestId('wire-console')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Instrument console' }))
    expect(screen.queryByTestId('wire-console')).toBeNull()
  })
})
