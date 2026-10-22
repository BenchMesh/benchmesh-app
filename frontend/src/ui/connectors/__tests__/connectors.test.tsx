import React from 'react'
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { ConnectorsView } from '../ConnectorsView'
import { asScpiStatus, basePortProblem, portState, scpiSnippets, type ScpiClient, type ScpiStatus } from '../client'

const STATUS: ScpiStatus = {
  access: 'control',
  base_port: 5025,
  host: '127.0.0.1',
  instruments: [
    { id: 'tenma', name: 'Tenma', port: 5025, address: 'TCPIP0::127.0.0.1::5025::SOCKET', listening: true, error: null },
    { id: 'xdm', name: 'OWON XDM', port: 5026, address: 'TCPIP0::127.0.0.1::5026::SOCKET', listening: false, error: 'cannot listen on port 5026: in use' },
  ],
}

function fakeClient(start: ScpiStatus) {
  let current = start
  const client = {
    status: vi.fn(async () => current),
    update: vi.fn(async (u: { access?: ScpiStatus['access']; base_port?: number }) => {
      current = { ...current, ...u }
      return current
    }),
  }
  return client as unknown as ScpiClient & typeof client
}

describe('connectors model', () => {
  it('reads a /scpi reply defensively', () => {
    expect(asScpiStatus(null)).toBeNull()
    expect(asScpiStatus({ access: 'all' })).toBeNull()
    const s = asScpiStatus({ access: 'read', instruments: [{ id: 'a', port: 5025, address: 'X', listening: true }, { bad: 1 }] })!
    expect(s.base_port).toBe(5025)
    expect(s.instruments).toEqual([{ id: 'a', name: 'a', port: 5025, address: 'X', listening: true, error: null }])
  })

  it('checks the base port', () => {
    expect(basePortProblem('5025')).toBeNull()
    expect(basePortProblem('80')).toMatch(/1024 to 65000/)
    expect(basePortProblem('70000')).toMatch(/1024 to 65000/)
    expect(basePortProblem('50x')).toMatch(/1024 to 65000/)
  })

  it('describes each port', () => {
    expect(portState(STATUS, STATUS.instruments[0])).toBe('Listening')
    expect(portState(STATUS, STATUS.instruments[1])).toMatch(/in use/)
    expect(portState({ ...STATUS, access: 'off' }, STATUS.instruments[0])).toBe('Off')
  })

  it('gives code with the instrument address', () => {
    const s = scpiSnippets(STATUS.instruments[0])
    expect(s.map((x) => x.title)).toEqual(['MATLAB (Instrument Control Toolbox)', 'MATLAB (no toolbox)', 'Python (PyVISA)', 'LabVIEW, TestStand, other VISA programs'])
    expect(s[0].text).toContain('visadev("TCPIP0::127.0.0.1::5025::SOCKET")')
    expect(s[1].text).toContain('tcpclient("127.0.0.1", 5025)')
    expect(s[2].text).toContain('open_resource("TCPIP0::127.0.0.1::5025::SOCKET"')
    expect(scpiSnippets({ address: null, port: null })).toEqual([])
  })
})

describe('Connectors view', () => {
  it('shows the access, the addresses with their state and code for the chosen instrument', async () => {
    const client = fakeClient(STATUS)
    render(<ConnectorsView client={client} />)
    expect(await screen.findByRole('radio', { name: /Control/ })).toBeChecked()
    const list = screen.getByRole('list', { name: 'Virtual SCPI addresses' })
    expect(within(list).getByText('TCPIP0::127.0.0.1::5025::SOCKET')).toBeTruthy()
    expect(screen.getByTestId('scpi-state-tenma').textContent).toBe('Listening')
    expect(screen.getByTestId('scpi-state-xdm').textContent).toMatch(/in use/)
    expect(screen.getByText('MATLAB (Instrument Control Toolbox) - Tenma')).toBeTruthy()
    fireEvent.click(within(list).getByRole('button', { name: /OWON XDM/ }))
    expect(screen.getByText('Python (PyVISA) - OWON XDM')).toBeTruthy()
    expect(screen.getByText(/pip install benchmesh/)).toBeTruthy()
  })

  it('switches access and hides the addresses when off', async () => {
    const client = fakeClient(STATUS)
    render(<ConnectorsView client={client} />)
    fireEvent.click(await screen.findByRole('radio', { name: /Off/ }))
    await waitFor(() => expect(client.update).toHaveBeenCalledWith({ access: 'off' }))
    await waitFor(() => expect(screen.queryByRole('list', { name: 'Virtual SCPI addresses' })).toBeNull())
  })

  it('saves a valid first port only', async () => {
    const client = fakeClient(STATUS)
    render(<ConnectorsView client={client} />)
    const input = await screen.findByLabelText('First port')
    const save = screen.getByRole('button', { name: 'Save' })
    expect(save).toBeDisabled()
    fireEvent.change(input, { target: { value: '80' } })
    expect(screen.getByText('A port from 1024 to 65000.')).toBeTruthy()
    expect(save).toBeDisabled()
    fireEvent.change(input, { target: { value: '6000' } })
    fireEvent.click(save)
    await waitFor(() => expect(client.update).toHaveBeenCalledWith({ base_port: 6000 }))
  })

  it('shows a server error', async () => {
    const client = fakeClient(STATUS)
    client.update.mockRejectedValueOnce(new Error('saving SCPI settings: disk full'))
    render(<ConnectorsView client={client} />)
    fireEvent.click(await screen.findByRole('radio', { name: /Read/ }))
    expect((await screen.findByRole('alert')).textContent).toMatch(/disk full/)
  })
})
