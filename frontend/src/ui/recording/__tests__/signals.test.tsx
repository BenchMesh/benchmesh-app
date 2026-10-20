import React from 'react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import fixtures from '../../bench/__tests__/bench-fixtures.json'
import { dmmProfile, psuProfile, setterMap } from '../../bench/profiles'
import { PsuPanel } from '../../bench/panels/PsuPanel'
import { DmmPanel } from '../../bench/panels/DmmPanel'
import { RecordingsProvider } from '../RecordingsProvider'
import { SignalScope, signalOf } from '../signal'
import { fakeClient, MockSocket, recording, series } from './fakes'

const fx = (key: keyof typeof fixtures) => {
  const f = fixtures[key] as { features: any; methods: any[] }
  return [f.features, setterMap(f.methods)] as const
}
const si = (val: string) => ({ val, sci: val, si: { number: val } })
const api = () => ({ get: vi.fn(async () => null), set: vi.fn(async () => {}) })

beforeEach(() => {
  MockSocket.sockets = []
  ;(globalThis as any).WebSocket = MockSocket
})

function renderPsu(client: ReturnType<typeof fakeClient>, multi = false) {
  const profile = psuProfile(...fx('tenma-72-2540:PSU'))
  return render(
    <RecordingsProvider client={client}>
      <SignalScope value={{ deviceId: 'tenma-72-2540', klass: 'PSU', channel: '1', name: 'Tenma', multi }}>
        <PsuPanel status={{ VOUT: si('4.93'), IOUT: si('0.4'), OUTPUT: 'ON' }} profile={profile} api={api()} online />
      </SignalScope>
    </RecordingsProvider>,
  )
}

describe('recordable readings', () => {
  it('builds signals from the scope (multi-channel labels name the channel)', () => {
    const scope = { deviceId: 'psu', klass: 'psu', channel: '2', name: 'Bench PSU', multi: true }
    expect(signalOf(scope, 'VOUT', 'V')).toEqual({ device_id: 'psu', class: 'PSU', channel: 2, field: 'VOUT', label: 'Bench PSU · CH2 · VOUT', unit: 'V' })
    expect(signalOf({ ...scope, multi: false }, 'VPP', 'V', { channel: '3' }).label).toBe('Bench PSU · VPP')
  })

  it('PSU: VOUT, IOUT and POUT are recordable; right-click adds VOUT to a recording and marks it', async () => {
    const client = fakeClient([recording({ id: 1, name: 'Ramp' }), recording({ id: 2, name: 'Old', state: 'stopped' })])
    renderPsu(client)
    await waitFor(() => expect(client.list).toHaveBeenCalled())
    for (const name of ['Tenma · VOUT', 'Tenma · IOUT', 'Tenma · POUT']) expect(screen.getByRole('group', { name })).toBeInTheDocument()

    const vout = screen.getByRole('group', { name: 'Tenma · VOUT' })
    await waitFor(() => {
      fireEvent.contextMenu(vout, { clientX: 20, clientY: 20 })
      expect(screen.getByRole('menuitemcheckbox', { name: 'Ramp' })).toBeInTheDocument()
    })
    const menu = screen.getByRole('menu', { name: 'Record Tenma · VOUT' })
    // stopped recordings are not offered
    expect(within(menu).queryByRole('menuitemcheckbox', { name: 'Old' })).toBeNull()
    expect(within(menu).getByRole('menuitem', { name: 'New recording…' })).toBeInTheDocument()

    const item = within(menu).getByRole('menuitemcheckbox', { name: 'Ramp' })
    expect(item).toHaveAttribute('aria-checked', 'false')
    fireEvent.click(item)
    expect(item).toHaveAttribute('data-confirm', 'waiting') // in flight
    await waitFor(() => expect(item).toHaveAttribute('aria-checked', 'true'))
    expect(item).not.toHaveAttribute('data-confirm')
    expect(client.addSeries).toHaveBeenCalledWith(1, { device_id: 'tenma-72-2540', class: 'PSU', channel: 1, field: 'VOUT', label: 'Tenma · VOUT', unit: 'V' })
    expect(within(vout).getByRole('img', { name: 'Recorded in Ramp' })).toBeInTheDocument()
    expect(within(screen.getByRole('group', { name: 'Tenma · IOUT' })).queryByRole('img', { name: /Recorded/ })).toBeNull()

    // toggling again removes the series (its data stays on the server)
    fireEvent.click(item)
    await waitFor(() => expect(item).toHaveAttribute('aria-checked', 'false'))
    expect(client.removeSeries).toHaveBeenCalledWith(1, 1000)
    expect(within(vout).queryByRole('img', { name: /Recorded/ })).toBeNull()
  })

  it('PSU: Shift+F10 opens the menu; a refused request shows why', async () => {
    const client = fakeClient([recording({ id: 1, name: 'Ramp' })])
    client.addSeries.mockRejectedValueOnce(new Error('Signal is already active in this recording'))
    renderPsu(client)
    const iout = screen.getByRole('group', { name: 'Tenma · IOUT' })
    await waitFor(() => expect(client.list).toHaveBeenCalled())
    await waitFor(() => {
      fireEvent.keyDown(iout, { key: 'F10', shiftKey: true })
      expect(screen.getByRole('menuitemcheckbox', { name: 'Ramp' })).toBeInTheDocument()
    })
    fireEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Ramp' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Signal is already active')
    expect(screen.getByRole('menuitemcheckbox', { name: 'Ramp' })).toHaveAttribute('aria-checked', 'false')
  })

  it('PSU: a reading already recorded shows the dot from the start', async () => {
    const client = fakeClient([recording({ id: 1, name: 'Ramp', series: [series({ field: 'POUT' })] })])
    renderPsu(client)
    await waitFor(() => expect(within(screen.getByRole('group', { name: 'Tenma · POUT' })).getByRole('img', { name: 'Recorded in Ramp' })).toBeInTheDocument())
  })

  it('DMM: the main reading is recordable with the unit of the active function', async () => {
    const client = fakeClient([recording({ id: 7, name: 'Drift' })])
    const profile = dmmProfile(...fx('owon-xdm1241:DMM'))
    render(
      <RecordingsProvider client={client}>
        <SignalScope value={{ deviceId: 'owon-xdm1041', klass: 'DMM', channel: '1', name: 'XDM', multi: false }}>
          <DmmPanel status={{ MODE: 'VOLT', MEAS: si('4.98'), RANGE: 'AUTO' }} profile={profile} api={api()} online />
        </SignalScope>
      </RecordingsProvider>,
    )
    await waitFor(() => expect(client.list).toHaveBeenCalled())
    const meas = screen.getByRole('group', { name: 'XDM · MEAS' })
    await waitFor(() => {
      fireEvent.contextMenu(meas, { clientX: 5, clientY: 5 })
      expect(screen.getByRole('menuitemcheckbox', { name: 'Drift' })).toBeInTheDocument()
    })
    await act(async () => { fireEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Drift' })) })
    expect(client.addSeries).toHaveBeenCalledWith(7, expect.objectContaining({ device_id: 'owon-xdm1041', class: 'DMM', field: 'MEAS', unit: 'V', label: 'XDM · MEAS' }))
    await waitFor(() => expect(within(meas).getByRole('img', { name: 'Recorded in Drift' })).toBeInTheDocument())
  })

  it('"New recording…" creates a recording and adds the reading to it', async () => {
    const client = fakeClient([])
    renderPsu(client)
    await waitFor(() => expect(client.list).toHaveBeenCalled())
    fireEvent.contextMenu(screen.getByRole('group', { name: 'Tenma · VOUT' }), { clientX: 5, clientY: 5 })
    fireEvent.click(screen.getByRole('menuitem', { name: 'New recording…' }))
    const dialog = screen.getByRole('dialog', { name: 'New recording with Tenma · VOUT' })
    fireEvent.change(within(dialog).getByLabelText('Name'), { target: { value: 'Quick' } })
    await act(async () => { fireEvent.click(within(dialog).getByRole('button', { name: 'Start recording' })) })
    expect(client.create).toHaveBeenCalledWith({ name: 'Quick', description: undefined, interval_s: 1 })
    expect(client.addSeries).toHaveBeenCalledWith(100, expect.objectContaining({ field: 'VOUT' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(within(screen.getByRole('group', { name: 'Tenma · VOUT' })).getByRole('img', { name: 'Recorded in Quick' })).toBeInTheDocument()
  })

  it('without a recordings provider readings are plain', () => {
    const profile = psuProfile(...fx('tenma-72-2540:PSU'))
    render(<SignalScope value={{ deviceId: 'x', klass: 'PSU', channel: '1', name: 'X', multi: false }}>
      <PsuPanel status={{}} profile={profile} api={api()} online /></SignalScope>)
    expect(screen.queryByRole('group', { name: 'X · VOUT' })).toBeNull()
  })
})
