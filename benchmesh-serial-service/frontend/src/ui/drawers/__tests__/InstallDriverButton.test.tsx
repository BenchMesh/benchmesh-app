import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { InstallDriverButton, installDriver, REENUMERATE_MS } from '../InstallDriverButton'

function reply(status: number, body: unknown) {
  return Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))
}

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers() })

describe('installDriver', () => {
  it('posts the device path', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(() => reply(200, { device: 'usb:1ab1:044d:X', installed: true }))
    await installDriver('/api', 'usb:1ab1:044d:X')
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe('/api/usbtmc-devices/driver')
    expect(init?.method).toBe('POST')
    expect(JSON.parse(String(init?.body))).toEqual({ device: 'usb:1ab1:044d:X' })
  })

  it('throws the server detail', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => reply(403, { detail: 'the administrator approval was declined' }))
    await expect(installDriver('/api', 'usb:1ab1:044d:X')).rejects.toThrow('approval was declined')
  })
})

describe('InstallDriverButton', () => {
  it('waits for the approval, then refreshes once Windows re-enumerated the instrument', async () => {
    let answer: (r: Response) => void = () => {}
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => new Promise<Response>((res) => { answer = res }))
    const onInstalled = vi.fn()
    render(<InstallDriverButton apiBase="/api" device="usb:1ab1:044d:X" onInstalled={onInstalled} />)
    vi.useFakeTimers({ shouldAdvanceTime: true })
    fireEvent.click(screen.getByRole('button', { name: 'Install driver' }))
    expect(screen.getByRole('button').textContent).toBe('Waiting for Windows approval…')
    expect(screen.getByRole('button')).toBeDisabled()
    answer(new Response(JSON.stringify({ installed: true }), { status: 200 }))
    await waitFor(() => expect(screen.getByRole('button').textContent).toBe('✓ Driver installed'))
    expect(onInstalled).not.toHaveBeenCalled()
    vi.advanceTimersByTime(REENUMERATE_MS)
    expect(onInstalled).toHaveBeenCalledOnce()
  })

  it('shows why it failed and can be tried again', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => reply(403, { detail: 'the administrator approval was declined; the driver was not installed' }))
    const onInstalled = vi.fn()
    render(<InstallDriverButton apiBase="/api" device="usb:1ab1:044d:X" onInstalled={onInstalled} />)
    fireEvent.click(screen.getByRole('button', { name: 'Install driver' }))
    await screen.findByText(/approval was declined/)
    expect(screen.getByRole('button', { name: 'Install driver' })).not.toBeDisabled()
    expect(onInstalled).not.toHaveBeenCalled()
  })
})

describe('Detect offers the driver', () => {
  it('on a USBTMC row the server can give a driver', async () => {
    const { DetectView } = await import('../DetectView')
    const port = (path: string) => ({
      port: path, kind: 'usbtmc', usb: null, configured_as: null, locked: false, instrument: null, match: null, certainty: null,
      error: 'cannot open: Windows has no driver for this instrument. …',
    })
    vi.spyOn(globalThis, 'fetch').mockImplementation((url) => String(url).endsWith('/devices/detect')
      ? reply(200, { ports: [port('usb:1ab1:044d:DHO8A272907001'), port('usb:5345:1235:VISA-OWNED')] })
      : reply(200, [
        { device: 'usb:1ab1:044d:DHO8A272907001', name: 'x', note: 'no driver', driver_installable: true },
        { device: 'usb:5345:1235:VISA-OWNED', name: 'y', note: 'other', driver_installable: false },
      ]))
    render(<DetectView apiBase="/api" configured={[]} onChanged={() => {}} />)
    fireEvent.click(screen.getByTestId('detect-btn'))
    await screen.findByRole('button', { name: 'Install driver' })
    expect(screen.getAllByRole('button', { name: 'Install driver' })).toHaveLength(1)
    const rows = screen.getAllByTestId('detect-row')
    expect(rows[0].querySelector('[data-testid="install-driver"]')).not.toBeNull()
    expect(rows[1].querySelector('[data-testid="install-driver"]')).toBeNull()
  })
})
