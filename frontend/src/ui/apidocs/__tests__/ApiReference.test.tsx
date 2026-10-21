import React from 'react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { ApiReference } from '../ApiReference'

const spec = {
  openapi: '3.1.0',
  info: { title: 'BenchMesh API', version: '0.1.9' },
  tags: [{ name: 'system', description: 'Service status' }, { name: 'websockets' }],
  paths: {
    '/status': { get: { tags: ['system'], summary: 'Device connection counts', parameters: [] } },
    '/devices/{id}': { delete: { tags: ['system'], summary: 'Remove a device', parameters: [{ name: 'id', in: 'path' }] } },
    '/ws/registry': { get: { tags: ['websockets'], summary: 'WebSocket: live registry', parameters: [], 'x-websocket': true } },
  },
}

const instruments = [{ id: 'tenma-1', name: 'Tenma', classes: [{ class: 'PSU', channels: ['/instruments/PSU/tenma-1/1'] }] }] as any

let fetchMock: ReturnType<typeof vi.fn>
beforeEach(() => {
  fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    if (url.endsWith('/openapi.json')) return new Response(JSON.stringify(spec), { status: 200, headers: { 'content-type': 'application/json' } })
    if (url.endsWith('/status')) return new Response('{"connected":2}', { status: 200, statusText: 'OK', headers: { 'content-type': 'application/json' } })
    if (init?.method === 'DELETE') return new Response(null, { status: 204, statusText: 'No Content' })
    return new Response('{"detail":"nope"}', { status: 404, statusText: 'Not Found', headers: { 'content-type': 'application/json' } })
  })
  global.fetch = fetchMock as any
})
afterEach(() => vi.useRealTimers())

describe('ApiReference', () => {
  it('lists the endpoints by tag and sends a GET', async () => {
    render(<ApiReference apiBase="" instruments={instruments} />)
    expect(await screen.findByText('BenchMesh API')).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'system' })).toBeInTheDocument()
    expect(screen.getByText('3 endpoints')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /GET \/status/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    const response = await screen.findByLabelText('Response')
    expect(response).toHaveTextContent('200 OK')
    expect(response.querySelector('pre')?.textContent).toBe('{\n  "connected": 2\n}')
  })

  it('a request that changes the server takes a second press (which expires)', async () => {
    render(<ApiReference apiBase="" instruments={instruments} />)
    fireEvent.click(await screen.findByRole('button', { name: /DELETE \/devices/ }))
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
    fireEvent.change(screen.getByLabelText('id'), { target: { value: 'tenma-1' } })
    vi.useFakeTimers()
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    expect(screen.getByRole('button', { name: 'Press again: DELETE' })).toBeInTheDocument()
    act(() => { vi.advanceTimersByTime(4100) })
    expect(screen.getByRole('button', { name: 'Send' })).toBeInTheDocument()
    vi.useRealTimers()
    fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    fireEvent.click(screen.getByRole('button', { name: 'Press again: DELETE' }))
    expect(await screen.findByText('204 No Content')).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith('/devices/tenma-1', expect.objectContaining({ method: 'DELETE' }))
  })

  it('suggests configured instrument ids for {id}', async () => {
    render(<ApiReference apiBase="" instruments={instruments} />)
    fireEvent.click(await screen.findByRole('button', { name: /DELETE \/devices/ }))
    const list = document.getElementById(screen.getByLabelText('id').getAttribute('list')!)
    expect([...list!.querySelectorAll('option')].map((o) => o.value)).toEqual(['tenma-1'])
  })

  it('shows a readable error and retries when the document cannot be loaded', async () => {
    fetchMock.mockResolvedValueOnce(new Response('x', { status: 500 }))
    render(<ApiReference apiBase="" />)
    expect(await screen.findByRole('alert')).toHaveTextContent('The server answered 500 for /openapi.json')
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
    expect(await screen.findByText('BenchMesh API')).toBeInTheDocument()
  })

  it('peeks at a WebSocket stream: first message, then closes', async () => {
    const sockets: any[] = []
    class WS { url: string; onmessage: any; onerror: any; close = vi.fn(); constructor(u: string) { this.url = u; sockets.push(this) } }
    ;(global as any).WebSocket = WS
    render(<ApiReference apiBase="" />)
    fireEvent.click(await screen.findByRole('button', { name: /WS \/ws\/registry/ }))
    expect(screen.getByText(/^ws:\/\/.*\/ws\/registry$/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Show the first message' }))
    expect(sockets[0].url).toMatch(/^ws:\/\/.*\/ws\/registry$/)
    sockets[0].onmessage({ data: '{"tenma-1":{}}' })
    await waitFor(() => expect(screen.getByLabelText('First message')).toHaveTextContent('"tenma-1": {}'))
    expect(sockets[0].close).toHaveBeenCalled()
  })
})
