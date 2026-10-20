import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, waitFor, within, cleanup } from '@testing-library/react'
import fixtures from './bench-fixtures.json'
import { psuProfile, ellProfile, dmmProfile, awgProfile, setterMap } from '../profiles'
import { PsuPanel } from '../panels/PsuPanel'
import { EllPanel } from '../panels/EllPanel'
import { DmmPanel, fullScale, rangeValue } from '../panels/DmmPanel'
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

  it('a mode without a setpoint here says where to set it', () => {
    render(<EllPanel status={{ MODE: 'DYN', REMOTE: 'ON', INPUT: 'OFF' }} profile={profile} api={mockApi()} online />)
    expect(screen.getByText(/Dynamic: its parameters are set on the instrument/)).toBeInTheDocument()
    expect(screen.queryByRole('spinbutton')).toBeNull()
  })

  it('short circuit: read from the load, two presses to switch on, read back', async () => {
    const api = mockApi({ short: 'OFF' })
    render(<EllPanel status={{ MODE: 'CURR', REMOTE: 'ON', INPUT: 'OFF' }} profile={profile} api={api} online />)
    await waitFor(() => expect(api.get).toHaveBeenCalledWith('short'))
    const sw = screen.getByRole('button', { name: /Short/ })
    fireEvent.click(sw)
    expect(api.set).not.toHaveBeenCalledWith('short', 'ON')
    fireEvent.click(sw)
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('short', 'ON'))
    await waitFor(() => expect(api.get).toHaveBeenCalledTimes(3))
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

  it('keeps the dial in place, greyed out, while the meter settles after a range change', async () => {
    let finish: () => void = () => {}
    const api = mockApi()
    api.set.mockImplementation(() => new Promise<void>((r) => { finish = r }))
    const profile = dmmProfile(...fx('owon-xdm1241:DMM'))
    const { rerender } = render(<DmmPanel status={{ MEAS: si('4.7'), MODE: 'RES', RANGE: '5E6' }} profile={profile} api={api} online />)
    const dial = screen.getByRole('slider', { name: 'Function' })
    expect(dial).toHaveAttribute('aria-valuetext', 'Resistance')
    fireEvent.click(screen.getByRole('button', { name: '500KΩ' }))
    // request in flight: the dial is locked and faded, still on resistance
    expect(dial).toHaveAttribute('aria-disabled', 'true')
    expect(dial.className).toContain('kit-dial--busy')
    finish()
    // settling: the meter reports no function for a moment - the dial must not jump
    rerender(<DmmPanel status={{ MEAS: null, MODE: null, RANGE: '500E3' }} profile={profile} api={api} online />)
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('range', '500E3'))
    expect(dial).toHaveAttribute('aria-valuetext', 'Resistance')
    expect(dial.className).toContain('kit-dial--busy')
    rerender(<DmmPanel status={{ MEAS: si('4.7'), MODE: 'RES', RANGE: '500E3' }} profile={profile} api={api} online />)
    await waitFor(() => expect(dial).not.toHaveAttribute('aria-disabled'))
    expect(dial.className).not.toContain('kit-dial--busy')
  })

  it('computes the full scale of the bar', () => {
    expect(fullScale('20', 3.3)).toBe(20)
    expect(fullScale('AUTO', 3.3)).toBe(10)
    expect(fullScale('AUTO', -9.7e-5)).toBeCloseTo(1e-4)
    expect(fullScale(null, null)).toBeNull()
    // meters that report a range index: the label gives the full scale (Rigol DM3000 "2" = 20V)
    const idx = [{ value: 'AUTO', display: 'AUTO' }, { value: '1', display: '2V' }, { value: '2', display: '20V' }, { value: '0', display: '200mV' }]
    expect(fullScale('2', 3.3, idx)).toBe(20)
    expect(fullScale('0', 0.1, idx)).toBeCloseTo(0.2)
    expect(rangeValue('10000µF')).toBeCloseTo(0.01)
    expect(rangeValue('500KΩ')).toBe(5e5)
    expect(rangeValue('AUTO')).toBeNull()
  })

  it('labels a temperature with the scale chosen on the meter', async () => {
    const api = mockApi()
    const profile = dmmProfile(...fx('owon-xdm1241:DMM'))
    const temp = profile.modes.find((m) => m.key.toUpperCase().startsWith('TEMP'))!
    render(<DmmPanel status={{ MEAS: si('77.0'), MODE: temp.key }} profile={profile} api={api} online />)
    const f = profile.tempScales.find((t) => t.value === 'F')!
    fireEvent.click(screen.getByRole('button', { name: f.display }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('temp_scale', 'F'))
    await waitFor(() => expect(screen.getAllByText(f.display).length).toBeGreaterThan(1))
  })

  it('re-reads the rate when the function changes (meters keep one per function)', async () => {
    const api = mockApi({ rate: 'M' })
    const profile = dmmProfile(...fx('owon-xdm1241:DMM'))
    const { rerender } = render(<DmmPanel status={{ MEAS: si('1'), MODE: 'VOLT' }} profile={profile} api={api} online />)
    await waitFor(() => expect(api.get).toHaveBeenCalledTimes(1))
    rerender(<DmmPanel status={{ MEAS: si('1'), MODE: 'CURR' }} profile={profile} api={api} online />)
    await waitFor(() => expect(api.get).toHaveBeenCalledTimes(2))
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

  const awg = (status: Record<string, unknown>, api = mockApi()) => {
    render(<AwgChannel ch="1" status={{ OUTPUT: 'OFF', SHAPE: 'SIN', FREQ: si('1000'), AMPL: si('2'), OFFS: si('0.5'), ...status }}
      profile={awgProfile(...fx('owon-dge2070:AWG'))} api={api} online />)
    return api
  }
  const enter = (name: string, digits: string) => {
    const ed = screen.getByRole('spinbutton', { name })
    fireEvent.focus(ed)
    fireEvent.click(screen.getAllByTestId('digit').find((d) => ed.contains(d))!)
    for (const k of digits) fireEvent.keyDown(ed, { key: k })
    fireEvent.keyDown(ed, { key: 'Enter' })
  }

  it('enters the period and sends the frequency', async () => {
    localStorage.clear()
    const api = awg({})
    fireEvent.click(screen.getByRole('button', { name: 'T' }))
    const period = screen.getByRole('spinbutton', { name: 'Period' })
    expect(period).toHaveAttribute('aria-valuenow', '1')
    expect(period).toHaveAttribute('aria-valuetext', '1 ms')
    enter('Period', '002500')
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('frequency', 400))
    // the choice is kept
    expect(JSON.parse(localStorage.getItem('benchmesh:awg-entry')!)).toMatchObject({ freq: 'PERIOD' })
    localStorage.clear()
  })

  it('the entry choice applies to every channel', () => {
    localStorage.clear()
    const profile = awgProfile(...fx('owon-dge2070:AWG'))
    const st = { OUTPUT: 'OFF', SHAPE: 'SIN', FREQ: si('1000'), AMPL: si('2'), OFFS: si('0') }
    render(<><AwgChannel ch="1" status={st} profile={profile} api={mockApi()} online /><AwgChannel ch="2" status={st} profile={profile} api={mockApi()} online /></>)
    fireEvent.click(screen.getAllByRole('button', { name: 'T' })[0])
    expect(screen.getAllByRole('spinbutton', { name: 'Period' })).toHaveLength(2)
    localStorage.clear()
  })

  it('enters Vrms and high / low levels, refuses dBm at High-Z, shows what it calculates', async () => {
    localStorage.clear()
    const api = awg({ LOAD: 'HIGHZ' })
    expect(screen.getByRole('button', { name: 'dBm' })).toBeDisabled()
    expect(screen.getByText('HIGH-Z')).toBeInTheDocument()
    expect(screen.getByText(/T 1 ms/)).toBeInTheDocument()
    expect(screen.getByText(/707.1 mVrms/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Vrms' }))
    expect(screen.getByRole('spinbutton', { name: 'Amplitude' })).toHaveAttribute('aria-valuenow', '0.7071')
    enter('Amplitude', '10000')
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('amplitude', 2.82843))
    fireEvent.click(screen.getByRole('button', { name: 'H/L' }))
    expect(screen.getByRole('spinbutton', { name: 'High level' })).toHaveAttribute('aria-valuenow', '1.5')
    expect(screen.queryByRole('spinbutton', { name: 'Offset' })).toBeNull()
    api.set.mockClear()
    // high 3.000 V with low -0.5 V: offset 1.25, amplitude 3.5 (grows: offset first)
    enter('High level', '03000')
    await waitFor(() => expect(api.set.mock.calls).toEqual([['offset', 1.25], ['amplitude', 3.5]]))
    localStorage.clear()
  })

  it('offers dBm into 50 ohm, symmetry only for a ramp, and phase', () => {
    localStorage.clear()
    awg({ LOAD: '50', SHAPE: 'RAMP', SYMM: 20, PHASE: 90 })
    expect(screen.getByRole('button', { name: 'dBm' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Vrms' })).toBeEnabled()
    expect(screen.getByRole('spinbutton', { name: 'Symmetry' })).toHaveAttribute('aria-valuenow', '20')
    expect(screen.getByRole('spinbutton', { name: 'Phase' })).toHaveAttribute('aria-valuenow', '90')
  })

  it('switches the mode and shows the settings of the active one', async () => {
    localStorage.clear()
    const api = awg({ MODE: 'CW' })
    const modes = screen.getByRole('group', { name: 'Mode CH1' })
    expect(screen.queryByRole('group', { name: 'Sweep settings' })).toBeNull()
    fireEvent.click(within(modes).getByRole('button', { name: 'SWEEP' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('mode', 'SWEEP'))
  })

  it('modulation: type, modulating wave and frequency, depth for AM, deviation for FM', async () => {
    localStorage.clear()
    const api = awg({ MODE: 'MOD', MOD_TYPE: 'AM', MOD_SHAPE: 'SIN', MOD_FREQ: si('100'), AM_DEPTH: 80, FM_DEV: si('100') })
    const mod = screen.getByRole('group', { name: 'Modulation settings' })
    expect(within(mod).getByRole('spinbutton', { name: 'Depth' })).toHaveAttribute('aria-valuenow', '80')
    expect(within(mod).getByRole('spinbutton', { name: 'Modulating frequency' })).toHaveAttribute('aria-valuenow', '100')
    expect(screen.getByRole('img', { name: /AM 100 Hz/ })).toBeInTheDocument()
    fireEvent.click(within(mod).getByRole('button', { name: 'FM' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('mod_type', 'FM'))
    fireEvent.click(within(mod).getByRole('button', { name: 'SQU' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('mod_shape', 'SQU'))
  })

  it('sweep: start, stop, time with a unit, spacing and trigger', async () => {
    localStorage.clear()
    const api = awg({ MODE: 'SWEEP', SWEEP_START: si('100'), SWEEP_STOP: si('1000'), SWEEP_TIME: si('0.5'), SWEEP_SPACING: 'LIN', SWEEP_TRIG: 'INT' })
    const sw = screen.getByRole('group', { name: 'Sweep settings' })
    expect(within(sw).getByRole('spinbutton', { name: 'Stop' })).toHaveAttribute('aria-valuenow', '1000')
    expect(within(sw).getByRole('spinbutton', { name: 'Sweep time' })).toHaveAttribute('aria-valuetext', '500 ms')
    fireEvent.click(within(sw).getByRole('button', { name: 'Log' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('sweep_spacing', 'LOG'))
    enter('Sweep time', '002000')
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('sweep_time', 0.002))
  })

  it('burst: cycles only for N cycles, period only with the internal trigger', () => {
    localStorage.clear()
    awg({ MODE: 'BURST', BURST_MODE: 'NCYC', BURST_CYCLES: 5, BURST_PERIOD: si('0.01'), BURST_TRIG: 'INT' })
    const b = screen.getByRole('group', { name: 'Burst settings' })
    expect(within(b).getByRole('spinbutton', { name: 'Cycles' })).toHaveAttribute('aria-valuenow', '5')
    expect(within(b).getByRole('spinbutton', { name: 'Burst period' })).toHaveAttribute('aria-valuetext', '10 ms')
    cleanup()
    awg({ MODE: 'BURST', BURST_MODE: 'INF', BURST_CYCLES: null, BURST_TRIG: 'MAN' })
    const b2 = screen.getByRole('group', { name: 'Burst settings' })
    expect(within(b2).queryByRole('spinbutton', { name: 'Cycles' })).toBeNull()
    expect(within(b2).queryByRole('spinbutton', { name: 'Burst period' })).toBeNull()
  })

  it('pulse: duty, width and edges in their own units; load chips', async () => {
    localStorage.clear()
    const api = awg({ SHAPE: 'PUL', DUTY: 20, WIDTH: si('0.0000666667'), RISE: si('0.000001953'), FALL: si('0.000001953'), LOAD: 'HIGHZ' })
    const pulse = screen.getByRole('group', { name: 'Pulse' })
    expect(within(pulse).getByRole('spinbutton', { name: 'Width' })).toHaveAttribute('aria-valuetext', '66.667 µs')
    expect(within(pulse).getByRole('spinbutton', { name: 'Rise' })).toHaveAttribute('aria-valuetext', '1.953 µs')
    enter('Width', '100000')
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('pulse_width', 0.0001))
    fireEvent.click(within(screen.getByRole('group', { name: 'Load CH1' })).getByRole('button', { name: '50 Ω' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('load', '50'))
  })

  it('no Vrms for noise, and a hint when the amplitude is not reported in Vpp', () => {
    localStorage.clear()
    awg({ SHAPE: 'PRN', AMPL: null })
    expect(screen.getByRole('button', { name: 'Vrms' })).toBeDisabled()
    expect(screen.queryByRole('spinbutton', { name: 'Symmetry' })).toBeNull()
    expect(screen.getByText(/not reported in Vpp/)).toBeInTheDocument()
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
