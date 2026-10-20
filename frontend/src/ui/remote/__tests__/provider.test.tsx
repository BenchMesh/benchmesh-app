import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, render, screen } from '@testing-library/react'
import { ApiError } from '../../automation/client'
import { RemoteProvider, SESSION_CHECK_MS, STATUS_POLL_MS, useRemote } from '../RemoteProvider'
import { fakeRemote } from './fakeClient'

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers() })

function Probe() {
  const r = useRemote()
  return <span data-testid="probe">{r?.principal ?? 'unknown'}|{r?.status ? `on=${r.status.enabled}` : 'none'}</span>
}

const flush = () => act(async () => { await Promise.resolve(); await Promise.resolve() })

describe('RemoteProvider', () => {
  it('local: learns the principal once and polls GET /remote every 3 s', async () => {
    vi.useFakeTimers()
    const { client } = fakeRemote({ enabled: true, password_set: true })
    render(<RemoteProvider client={client}><Probe /></RemoteProvider>)
    await flush()
    expect(screen.getByTestId('probe').textContent).toBe('local|on=true')
    const polls = (client.status as any).mock.calls.length
    await act(async () => { vi.advanceTimersByTime(STATUS_POLL_MS * 2) })
    expect((client.status as any).mock.calls.length).toBe(polls + 2)
    expect(client.session).toHaveBeenCalledTimes(1)
  })

  it('local on an older server: 404 means the feature is absent; other errors keep the last state', async () => {
    vi.useFakeTimers()
    const { client } = fakeRemote({ enabled: true, password_set: true })
    render(<RemoteProvider client={client}><Probe /></RemoteProvider>)
    await flush()
    ;(client.status as any).mockRejectedValueOnce(new Error('network'))
    await act(async () => { vi.advanceTimersByTime(STATUS_POLL_MS) })
    await flush()
    expect(screen.getByTestId('probe').textContent).toBe('local|on=true')
    ;(client.status as any).mockRejectedValue(new ApiError('Not found', 404))
    await act(async () => { vi.advanceTimersByTime(STATUS_POLL_MS) })
    await flush()
    expect(screen.getByTestId('probe').textContent).toBe('local|none')
  })

  it('a session reply that is not one (or an unreachable server) is the local principal', async () => {
    const { client } = fakeRemote()
    ;(client.session as any).mockRejectedValue(new ApiError('Not found', 404))
    render(<RemoteProvider client={client}><Probe /></RemoteProvider>)
    expect(await screen.findByText(/^local\|/)).toBeTruthy()
  })

  it('panel windows do not poll the remote access state', async () => {
    const { client } = fakeRemote()
    render(<RemoteProvider client={client} watchStatus={false}><Probe /></RemoteProvider>)
    await screen.findByText('local|none')
    expect(client.status).not.toHaveBeenCalled()
  })

  it('remote: never reads the admin state, re-checks the session every 15 s and signs in again on 401', async () => {
    vi.useFakeTimers()
    const { client } = fakeRemote({}, { principal: 'remote', address: '192.168.1.30', since: 1 })
    const ended = vi.fn()
    render(<RemoteProvider client={client} onSessionEnded={ended}><Probe /></RemoteProvider>)
    await flush()
    expect(screen.getByTestId('probe').textContent).toBe('remote|none')
    await act(async () => { vi.advanceTimersByTime(SESSION_CHECK_MS) })
    expect(client.session).toHaveBeenCalledTimes(2)
    expect(ended).not.toHaveBeenCalled()
    ;(client.session as any).mockRejectedValue(new ApiError('signed out', 401))
    await act(async () => { vi.advanceTimersByTime(SESSION_CHECK_MS) })
    await flush()
    expect(ended).toHaveBeenCalledTimes(1)
    expect(client.status).not.toHaveBeenCalled()
  })

  it('remote without a session at start goes to the sign-in page', async () => {
    const { client } = fakeRemote()
    ;(client.session as any).mockRejectedValue(new ApiError('no session', 401))
    const ended = vi.fn()
    render(<RemoteProvider client={client} onSessionEnded={ended}><Probe /></RemoteProvider>)
    await vi.waitFor(() => expect(ended).toHaveBeenCalledTimes(1))
    expect(screen.getByTestId('probe').textContent).toBe('unknown|none')
  })
})
