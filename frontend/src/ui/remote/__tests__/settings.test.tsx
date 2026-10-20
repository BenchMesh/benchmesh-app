import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { RemoteProvider } from '../RemoteProvider'
import { RemoteSettings, passwordProblem, stateText } from '../RemoteSettings'
import { RemoteBadge } from '../RemoteBadge'
import { SettingsTab } from '../../automation/SettingsTab'
import { automationClient } from '../../automation/client'
import { InstallDriverButton } from '../../drawers/InstallDriverButton'
import { fakeRemote, SESSIONS } from './fakeClient'
import type { RemoteClient } from '../client'

afterEach(() => vi.restoreAllMocks())

const withRemote = (client: RemoteClient, ui: React.ReactNode) => render(<RemoteProvider client={client} onSessionEnded={() => {}}>{ui}</RemoteProvider>)

describe('password rules', () => {
  it('needs 10 characters and two equal entries', () => {
    expect(passwordProblem('short', 'short')).toMatch(/At least 10/)
    expect(passwordProblem('0123456789', '012345678x')).toMatch(/do not match/)
    expect(passwordProblem('0123456789', '0123456789')).toBeNull()
  })

  it('describes the listener state', () => {
    const base = { enabled: true, port: 57667, password_set: true, listening: false, error: null, urls: [], fingerprint: null, sessions: [] }
    expect(stateText({ ...base, listening: true })).toBe('Listening on port 57667')
    expect(stateText({ ...base, error: 'address in use' })).toBe('Not listening: address in use')
    expect(stateText({ ...base, enabled: false, password_set: false })).toMatch(/Set a password/)
  })
})

describe('Remote access settings', () => {
  it('sets a password, then switches on and shows the addresses and the fingerprint', async () => {
    const { client } = fakeRemote()
    withRemote(client, <RemoteSettings />)
    await screen.findByRole('heading', { name: 'Remote access' })
    expect(screen.getByText(/Control this bench from a browser on your local network/)).toBeTruthy()
    const sw = screen.getByRole('switch', { name: 'Remote access' })
    expect(sw).toBeDisabled()
    expect(screen.getByTestId('password-state').textContent).toBe('No password set yet.')

    const pw = screen.getByLabelText('Password')
    const rep = screen.getByLabelText('Repeat password')
    expect(pw).toHaveAttribute('type', 'password')
    fireEvent.change(pw, { target: { value: 'short' } })
    fireEvent.click(screen.getByRole('button', { name: 'Set password' }))
    expect((await screen.findByRole('alert')).textContent).toMatch(/At least 10/)
    fireEvent.change(pw, { target: { value: 'correct horse' } })
    fireEvent.change(rep, { target: { value: 'correct hors' } })
    fireEvent.click(screen.getByRole('button', { name: 'Set password' }))
    expect((await screen.findByRole('alert')).textContent).toMatch(/do not match/)
    expect(client.setPassword).not.toHaveBeenCalled()

    fireEvent.change(rep, { target: { value: 'correct horse' } })
    fireEvent.click(screen.getByRole('button', { name: 'Set password' }))
    await screen.findByText('Password saved.')
    expect(client.setPassword).toHaveBeenCalledWith('correct horse')
    expect(screen.getByTestId('password-state').textContent).toBe('Password set')
    expect(screen.getByLabelText('New password')).toHaveValue('')
    expect(document.body.innerHTML).not.toContain('correct horse')

    await waitFor(() => expect(screen.getByRole('switch', { name: 'Remote access' })).not.toBeDisabled())
    fireEvent.click(screen.getByRole('switch', { name: 'Remote access' }))
    await waitFor(() => expect(screen.getByRole('switch', { name: 'Remote access' })).toHaveAttribute('aria-checked', 'true'))
    expect(client.update).toHaveBeenCalledWith({ enabled: true })
    expect(screen.getByTestId('remote-state').textContent).toBe('Listening on port 57667')
    expect(screen.getByRole('link', { name: 'https://192.168.1.20:57667/' })).toHaveAttribute('href', 'https://192.168.1.20:57667/')
    expect(screen.getByTestId('remote-fingerprint').textContent).toBe('AB:CD:EF:01')
    expect(screen.getByText(/check that its SHA-256 fingerprint matches/)).toBeTruthy()

    const writeText = vi.fn(() => Promise.resolve())
    Object.assign(navigator, { clipboard: { writeText } })
    fireEvent.click(screen.getByRole('button', { name: 'Copy https://192.168.1.20:57667/' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('https://192.168.1.20:57667/'))
  })

  it('shows the 409 of switching on without a password as a reason', async () => {
    const { client } = fakeRemote({ password_set: true })
    ;(client.update as any).mockRejectedValueOnce(Object.assign(new (await import('../../automation/client')).ApiError('a password is needed', 409)))
    withRemote(client, <RemoteSettings />)
    fireEvent.click(await screen.findByRole('switch', { name: 'Remote access' }))
    expect(await screen.findByText('Set a password first.')).toBeTruthy()
  })

  it('lists sessions and disconnects one or all', async () => {
    const { client } = fakeRemote({ enabled: true, password_set: true, sessions: structuredClone(SESSIONS) })
    withRemote(client, <RemoteSettings />)
    const list = await screen.findByRole('list', { name: 'Remote sessions' })
    expect(within(list).getByText('Chrome on Windows')).toBeTruthy()
    expect(within(list).getAllByRole('listitem')).toHaveLength(2)
    fireEvent.click(screen.getByRole('button', { name: 'Disconnect 192.168.1.30' }))
    await waitFor(() => expect(within(screen.getByRole('list', { name: 'Remote sessions' })).getAllByRole('listitem')).toHaveLength(1))
    expect(client.disconnect).toHaveBeenCalledWith('s1')
    fireEvent.click(screen.getByRole('button', { name: 'Disconnect all' }))
    await screen.findByText('Nobody is connected.')
    expect(client.disconnectAll).toHaveBeenCalled()
  })

  it('is not shown to a remote browser', async () => {
    const { client } = fakeRemote({}, { principal: 'remote', address: '192.168.1.30', since: 1 })
    const { container } = withRemote(client, <RemoteSettings />)
    await waitFor(() => expect(client.session).toHaveBeenCalled())
    expect(container.innerHTML).toBe('')
    expect(client.status).not.toHaveBeenCalled()
  })
})

describe('Remote badge', () => {
  it('local: Remote: N opens the sessions; Disconnect all and switch off turns remote access off', async () => {
    const { client } = fakeRemote({ enabled: true, password_set: true, sessions: structuredClone(SESSIONS) })
    withRemote(client, <RemoteBadge />)
    const open = await screen.findByRole('button', { name: 'Remote: 2' })
    fireEvent.click(open)
    const pop = screen.getByRole('dialog', { name: 'Remote sessions' })
    expect(within(pop).getByText('192.168.1.31')).toBeTruthy()
    expect(within(pop).getByText('Firefox on Linux')).toBeTruthy()
    fireEvent.click(within(pop).getByRole('button', { name: 'Disconnect all and switch off' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: /Remote:/ })).toBeNull())
    expect(client.update).toHaveBeenCalledWith({ enabled: false })
  })

  it('local: nothing while nobody is connected', async () => {
    const { client } = fakeRemote({ enabled: true, password_set: true })
    const { container } = withRemote(client, <RemoteBadge />)
    await waitFor(() => expect(client.status).toHaveBeenCalled())
    expect(container.innerHTML).toBe('')
  })

  it('remote: a Remote label and a Log out form that posts to /remote/logout', async () => {
    const { client } = fakeRemote({}, { principal: 'remote', address: '192.168.1.30', since: 1 })
    withRemote(client, <RemoteBadge />)
    const out = await screen.findByRole('button', { name: 'Log out' })
    const form = out.closest('form')!
    expect(form.getAttribute('method')).toBe('post')
    expect(form.getAttribute('action')).toMatch(/\/remote\/logout$/)
    expect(screen.getByText('Remote')).toBeTruthy()
  })
})

describe('admin controls for a remote browser', () => {
  it('automation settings show a note instead of the assistant key and MCP access', async () => {
    const { client } = fakeRemote({}, { principal: 'remote', address: '192.168.1.30', since: 1 })
    const fetchSpy = vi.spyOn(globalThis, 'fetch')
    withRemote(client, <SettingsTab client={automationClient} />)
    expect(await screen.findAllByText(/Change this in the BenchMesh app on the bench computer/)).toHaveLength(2)
    expect(screen.queryByLabelText(/API key/)).toBeNull()
    expect(screen.queryByRole('radio')).toBeNull()
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('the Install driver button is not offered', async () => {
    const { client } = fakeRemote({}, { principal: 'remote', address: '192.168.1.30', since: 1 })
    withRemote(client, <InstallDriverButton apiBase="/api" device="usb:1ab1:044d:X" onInstalled={() => {}} />)
    await waitFor(() => expect(client.session).toHaveBeenCalled())
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Install driver' })).toBeNull())
  })
})
