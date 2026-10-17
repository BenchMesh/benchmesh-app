import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import fixtures from './bench-fixtures.json'
import { psuProfile, ellProfile, dmmProfile, awgProfile, setterMap } from '../profiles'
import { PsuPanel } from '../panels/PsuPanel'
import { EllPanel } from '../panels/EllPanel'
import { DmmPanel, fullScale } from '../panels/DmmPanel'
import { AwgChannel } from '../panels/AwgPanel'
import { GenericPanel } from '../panels/GenericPanel'
import type { ChannelApi } from '../api'

const fx = (key: keyof typeof fixtures) => {
  const f = fixtures[key] as { features: any; methods: any[] }
  return [f.features, setterMap(f.methods)] as const
}
const si = (val: string) => ({ val, sci: val, si: { number: val } })

function mockApi(values: Record<string, unknown> = {}): ChannelApi & { set: ReturnType<typeof vi.fn> } {
  return {
    get: vi.fn(async (op: string) => values[op] ?? null),
    set: vi.fn(async () => {}),
  }
}

describe('PsuPanel', () => {
  const profile = psuProfile(...fx('tenma-72-2540:PSU'))
  const status = { VOUT: si('4.93'), IOUT: si('0.412'), POUT: si('2.03'), OUTPUT: 'ON', REMOTE: 'ON', CV: true, CC: false }

  it('shows readings, regulation flags and the setpoints read from the instrument', async () => {
    render(<PsuPanel status={status} profile={profile} api={mockApi({ voltage: si('4.94'), current: si('0.988') })} online />)
    expect(screen.getByRole('img', { name: 'Output voltage 04.93' })).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'Output current 0.412' })).toBeInTheDocument()
    expect(screen.getByLabelText('CV on')).toBeInTheDocument()
    expect(screen.getByLabelText('OUT on')).toBeInTheDocument()
    await waitFor(() => expect(screen.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveAttribute('aria-valuenow', '4.94'))
  })

  it('sends a setpoint only when applied', async () => {
    const api = mockApi({ voltage: si('4.94'), current: si('0.988') })
    render(<PsuPanel status={status} profile={profile} api={api} online />)
    const ed = screen.getByRole('spinbutton', { name: 'Voltage setpoint' })
    await waitFor(() => expect(ed).toHaveAttribute('aria-valuenow', '4.94'))
    fireEvent.focus(ed)
    fireEvent.keyDown(ed, { key: 'ArrowUp' })
    expect(api.set).not.toHaveBeenCalled()
    fireEvent.keyDown(ed, { key: 'Enter' })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('voltage', 5.94))
  })

  it('switches the output off with one press (it is on)', async () => {
    const api = mockApi()
    render(<PsuPanel status={status} profile={profile} api={api} online />)
    fireEvent.click(screen.getByRole('button', { name: /Output/ }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('output', 'OFF'))
  })

  it('disables control while offline', () => {
    render(<PsuPanel status={{}} profile={profile} api={mockApi()} online={false} />)
    expect(screen.getByRole('button', { name: /Output/ })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveAttribute('aria-disabled', 'true')
  })
})

describe('EllPanel', () => {
  const profile = ellProfile(...fx('owon-oel1515:ELL'))

  it('locks control until the load is in remote mode', () => {
    render(<EllPanel status={{ MODE: 'CURR', REMOTE: 'OFF', INPUT: 'OFF' }} profile={profile} api={mockApi()} online />)
    expect(screen.getByText(/Switch Remote on/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Input/ })).toBeDisabled()
    expect(screen.getByRole('switch', { name: /Remote/ })).not.toBeDisabled()
  })

  it('edits the setpoint of the active mode and changes mode', async () => {
    const api = mockApi({ res: '22' })
    render(<EllPanel status={{ MODE: 'RES', REMOTE: 'ON', INPUT: 'OFF' }} profile={profile} api={api} online />)
    const ed = screen.getByRole('spinbutton', { name: 'CR setpoint' })
    await waitFor(() => expect(ed).toHaveAttribute('aria-valuenow', '22'))
    fireEvent.click(screen.getByRole('button', { name: 'CC' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('mode', 'CURR'))
  })

  it('shows why a request was refused', async () => {
    const api = mockApi()
    api.set.mockRejectedValueOnce(new Error('Load is not in remote mode'))
    render(<EllPanel status={{ MODE: 'CURR', REMOTE: 'ON' }} profile={profile} api={api} online />)
    fireEvent.click(screen.getByRole('button', { name: 'CV' }))
    expect(await screen.findByText('Load is not in remote mode')).toBeInTheDocument()
  })
})

describe('DmmPanel', () => {
  it('shows the reading with an SI prefix and switches function from the dial', async () => {
    const api = mockApi()
    render(<DmmPanel status={{ MEAS: si('-0.0000978934'), MODE: 'VOLT', RANGE: null }} profile={dmmProfile(...fx('owon-xdm1241:DMM'))} api={api} online />)
    expect(screen.getByRole('img', { name: /-97\.893/ })).toBeInTheDocument()
    expect(screen.getByText('µV')).toBeInTheDocument()
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Function' }), { key: 'ArrowRight' })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('mode', 'VOLT_AC'))
  })

  it('selects a range', async () => {
    const api = mockApi()
    render(<DmmPanel status={{ MEAS: si('3.3'), MODE: 'VOLT_DC' }} profile={dmmProfile(...fx('owon-spm3103:DMM'))} api={api} online />)
    fireEvent.click(screen.getByRole('button', { name: '20V' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('range', '20'))
  })

  it('computes the full scale of the bar', () => {
    expect(fullScale('20', 3.3)).toBe(20)
    expect(fullScale('AUTO', 3.3)).toBe(10)
    expect(fullScale('AUTO', -9.7e-5)).toBeCloseTo(1e-4)
    expect(fullScale(null, null)).toBeNull()
  })
})

describe('AwgChannel', () => {
  it('sets the waveform and frequency', async () => {
    const api = mockApi()
    render(<AwgChannel ch="1" status={{ OUTPUT: 'OFF', SHAPE: 'SIN', FREQ: si('1000'), AMPL: si('1'), OFFS: si('0') }}
      profile={awgProfile(...fx('owon-dge2070:AWG'))} api={api} online />)
    fireEvent.click(screen.getByRole('button', { name: 'SQU' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('shape', 'SQU'))
    const freq = screen.getByRole('spinbutton', { name: 'Frequency' })
    expect(freq).toHaveAttribute('aria-valuenow', '1000')
    fireEvent.focus(freq)
    fireEvent.keyDown(freq, { key: 'ArrowUp' })
    fireEvent.keyDown(freq, { key: 'Enter' })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('frequency', 1001))
  })
})

describe('GenericPanel', () => {
  it('renders any class status', () => {
    render(<GenericPanel status={{ FREQ: si('1000'), LOCKED: true, MODE: 'RUN' }} />)
    expect(screen.getByRole('img', { name: /FREQ/ })).toBeInTheDocument()
    expect(screen.getByLabelText('LOCKED on')).toBeInTheDocument()
    expect(screen.getByText('RUN')).toBeInTheDocument()
  })
})
