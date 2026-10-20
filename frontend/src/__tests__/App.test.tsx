import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import App from '../ui/App'

// Mock fetch for GET /instruments
const mockInstruments = [
  { id: 'psu-1', IDN: 'ACME PSU-1000', classes: [{ class: 'PSU', channels: ['/instruments/PSU/psu-1/1'] }] },
]

global.fetch = vi.fn().mockResolvedValue({
  ok: true,
  status: 200,
  json: async () => mockInstruments,
  headers: {
    get: (name: string) => name === 'ETag' ? '"mock-etag"' : null
  }
} as any)

// Mock WebSocket
class MockWebSocket {
  url: string
  onopen: any; onmessage: any; onclose: any; onerror: any
  constructor(url: string) { this.url = url; setTimeout(() => this.onopen && this.onopen({}), 0) }
  send() {}
  close() { this.onclose?.({}) }
}
;(global as any).WebSocket = MockWebSocket as any


describe('App', () => {
  it('shows each instrument class as a panel on the bench', async () => {
    localStorage.setItem('benchmesh:disclaimer-accepted', 'true')
    render(<App />)
    const panel = await screen.findByRole('article', { name: 'Power Supply Unit: psu-1' })
    expect(panel).toHaveTextContent('ACME PSU-1000')
    expect(screen.getByRole('button', { name: 'Arrange' })).toBeInTheDocument()
  })
})
