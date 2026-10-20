import React from 'react'
import { describe, it, expect, vi, beforeAll } from 'vitest'
import { render, screen, fireEvent, waitFor, within, act } from '@testing-library/react'
import fixtures from './lcr-sal-fixtures.json'
import { lcrProfile, salProfile, setterMap } from '../profiles'
import { LcrPanel, plainText, derived, pickFunction } from '../panels/LcrPanel'
import { SalPanel, stepList } from '../panels/SalPanel'
import { ActionButton } from '../../kit'
import type { ChannelApi } from '../api'

const fx = (key: keyof typeof fixtures) => {
  const f = fixtures[key] as { features: any; methods: any[] }
  return [f.features, setterMap(f.methods)] as const
}
const si = (val: string) => ({ val, sci: val, si: { number: val } })

function mockApi(values: Record<string, unknown> = {}): ChannelApi & { set: ReturnType<typeof vi.fn> } {
  return { get: vi.fn(async (op: string) => values[op] ?? null), set: vi.fn(async () => {}) }
}

describe('LcrPanel', () => {
  const th = lcrProfile(...fx('tonghui-th2817b:LCR'))
  const et = lcrProfile(...fx('easttester-et4410:LCR'))
  const status = { PRIMARY: si('1.0234e-9'), SECONDARY: si('0.0123'), FUNC: 'CPD', FREQ: si('10000'), LEVEL: si('1'), SPEED: 'MED', RANGE: 'AUTO', OVERLOAD: false }

  it('shows the pair with its units, the setup annunciators and derived values', () => {
    render(<LcrPanel status={status} profile={th} api={mockApi()} online />)
    expect(screen.getByRole('img', { name: /Primary Cp.*1\.0234/ })).toBeInTheDocument()
    expect(screen.getByText('nF')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: /Secondary D.*0\.0123/ })).toBeInTheDocument()
    expect(screen.getByLabelText('PAR on')).toBeInTheDocument()
    expect(screen.getByLabelText('AUTO on')).toBeInTheDocument()
    expect(screen.getByLabelText('10 kHz on')).toBeInTheDocument()
    expect(screen.getByLabelText('OL off')).toBeInTheDocument()
    expect(screen.getByText('81.301')).toBeInTheDocument() // Q = 1/D
  })

  it('function keys: primary, secondary and circuit each select one function', async () => {
    const api = mockApi()
    render(<LcrPanel status={status} profile={th} api={api} online />)
    fireEvent.click(within(screen.getByRole('group', { name: 'Primary parameter' })).getByRole('button', { name: 'L' }))
    await waitFor(() => expect(api.set).toHaveBeenLastCalledWith('function', 'LSD'))
    fireEvent.click(within(screen.getByRole('group', { name: 'Secondary parameter' })).getByRole('button', { name: 'Q' }))
    await waitFor(() => expect(api.set).toHaveBeenLastCalledWith('function', 'CPQ'))
    fireEvent.click(screen.getByRole('button', { name: 'SER' }))
    await waitFor(() => expect(api.set).toHaveBeenLastCalledWith('function', 'CSD'))
  })

  it('frequency presets, range and speed', async () => {
    const api = mockApi({ level: si('1') })
    render(<LcrPanel status={status} profile={th} api={api} online />)
    expect(screen.getByRole('spinbutton', { name: 'Frequency' })).toHaveAttribute('aria-valuenow', '10000')
    fireEvent.click(screen.getByRole('button', { name: '1 kHz' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('frequency', 1000))
    fireEvent.click(screen.getByRole('button', { name: '1 kΩ' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('range', '1000'))
    fireEvent.click(screen.getByRole('button', { name: 'Slow' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('speed', 'SLOW'))
  })

  it('open correction asks to prepare the fixture first', async () => {
    const api = mockApi()
    render(<LcrPanel status={status} profile={th} api={api} online />)
    fireEvent.click(screen.getByRole('button', { name: 'Open' }))
    const dialog = screen.getByRole('dialog', { name: 'Open correction' })
    expect(within(dialog).getByText(/Open the test clips/)).toBeInTheDocument()
    expect(api.set).not.toHaveBeenCalled()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Run correction' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('correction', 'OPEN'))
  })

  it('East Tester: level chips from the model list, the DC bias needs two presses', async () => {
    const api = mockApi({ bias: si('0.5') })
    render(<LcrPanel status={{ ...status, LEVEL: si('0.6'), BIAS_ON: 'OFF' }} profile={et} api={api} online />)
    expect(screen.getByRole('button', { name: '600 mV' })).toHaveAttribute('aria-pressed', 'true')
    fireEvent.click(screen.getByRole('button', { name: '1 V' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('level', '1'))
    // the bias setter applies the bias at once: Enter twice
    const bias = screen.getByRole('spinbutton', { name: 'DC bias' })
    await waitFor(() => expect(bias).toHaveAttribute('aria-valuenow', '0.5'))
    fireEvent.focus(bias)
    fireEvent.keyDown(bias, { key: 'ArrowUp' })
    fireEvent.keyDown(bias, { key: 'Enter' })
    expect(await screen.findByText(/Press Enter again/)).toBeInTheDocument()
    expect(api.set).not.toHaveBeenCalledWith('bias', expect.anything())
    fireEvent.keyDown(bias, { key: 'Enter' })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('bias', 1.5))
    expect(screen.getByRole('button', { name: 'Correct' })).toBeInTheDocument()
  })

  it('DCR has no secondary readout; overload shows OL', () => {
    render(<LcrPanel status={{ PRIMARY: null, SECONDARY: null, FUNC: 'DCR', FREQ: null, OVERLOAD: true }} profile={et} api={mockApi()} online />)
    expect(screen.queryByRole('img', { name: /Secondary/ })).toBeNull()
    expect(screen.getByLabelText('OL on')).toBeInTheDocument()
  })

  it('disables control while offline', () => {
    render(<LcrPanel status={{}} profile={th} api={mockApi()} online={false} />)
    expect(screen.getByRole('button', { name: 'Slow' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Open' })).toBeDisabled()
  })

  it('helpers', () => {
    expect(plainText(0.0123, 5)).toBe(' 0.0123')
    expect(plainText(null, 5, true)).toBe('  OL  ')
    const fn = th.functions.find((f) => f.key === 'CSD')!
    expect(derived(fn, 1e-6, 0.01, 1000).map((d) => d.label)).toEqual(['Q', 'ESR'])
    expect(pickFunction(th.functions, fn, { group: 'L' })?.key).toBe('LSD')
    expect(pickFunction(th.functions, fn, { group: 'Z' })?.key).toBe('ZTD')
  })
})

describe('SalPanel', () => {
  beforeAll(() => {
    HTMLCanvasElement.prototype.getBoundingClientRect = () => ({ left: 0, top: 0, width: 442, height: 300, right: 442, bottom: 300, x: 0, y: 0, toJSON: () => ({}) })
  })
  const tiny = salProfile(...fx('tinysa-ultra:SAL'))
  const dsa = salProfile(...fx('rigol-dsa815:SAL'))
  const status = {
    CENTER: si('433920000'), SPAN: si('10000000'), START: si('428920000'), STOP: si('438920000'), RBW: si('30000'), RBW_AUTO: 'ON',
    REF_LEVEL: -10, SCALE: 10, ATTEN: 10, ATTEN_AUTO: 'ON', SWEEP_TIME: si('0.12'), SWEEP_MODE: 'CONT', POINTS: 5,
    TRACE: [-90.1, -85.3, -42.31, -80.2, -91], TRACE_UNIT: 'dBm', MARKER: 'ON', MARKER_FREQ: si('433920000'), MARKER_LEVEL: -42.31, TG_OUTPUT: 'OFF', TG_LEVEL: -30,
  }

  it('shows the marker readout, the annotation bar and the screen', () => {
    render(<SalPanel status={status} profile={tiny} api={mockApi()} online />)
    expect(screen.getByText('433.9200 MHz')).toBeInTheDocument()
    expect(screen.getByText('−42.31 dBm')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: /Spectrum 428\.920 MHz to 438\.920 MHz, reference −10\.0 dBm, 10 dB\/div/ })).toBeInTheDocument()
    expect(screen.getByLabelText('Annotations')).toHaveTextContent(/RBW30 kHz/)
    expect(screen.getByLabelText('CONT on')).toBeInTheDocument()
  })

  it('FREQ: edits the center frequency; a click on the screen places the marker', async () => {
    const api = mockApi()
    render(<SalPanel status={status} profile={tiny} api={api} online />)
    const ed = screen.getByRole('spinbutton', { name: 'Center' })
    expect(ed).toHaveAttribute('aria-valuenow', '433920000')
    fireEvent.focus(ed)
    fireEvent.keyDown(ed, { key: 'ArrowUp' })
    fireEvent.keyDown(ed, { key: 'Enter' })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('center', 433920001))
    fireEvent.click(screen.getByRole('img', { name: /Spectrum/ }), { clientX: 34 + 200, clientY: 50 })
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('marker_freq', 433920000))
  })

  it('BW and AMPT: RBW along the model list, ref level one division per step', async () => {
    const api = mockApi()
    render(<SalPanel status={status} profile={tiny} api={api} online />)
    fireEvent.click(screen.getByRole('tab', { name: 'BW' }))
    fireEvent.click(screen.getByRole('button', { name: 'RBW up' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('rbw', 100000))
    fireEvent.click(screen.getByRole('tab', { name: 'AMPT' }))
    fireEvent.click(screen.getByRole('button', { name: 'Ref level up' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('ref_level', 0))
    fireEvent.click(screen.getByRole('button', { name: '5 dB/div' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('scale', 5))
  })

  it('MKR: peak search waits for the instrument; TG needs two presses', async () => {
    const api = mockApi()
    const { rerender } = render(<SalPanel status={{ ...status, MARKER_FREQ: si('430000000'), MARKER_LEVEL: -88 }} profile={tiny} api={api} online />)
    fireEvent.click(screen.getByRole('tab', { name: 'MKR' }))
    const peak = screen.getByRole('button', { name: 'Peak' })
    fireEvent.click(peak)
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('peak_search', 'PEAK'))
    await waitFor(() => expect(peak).toHaveAttribute('data-confirm', 'waiting'))
    rerender(<SalPanel status={status} profile={tiny} api={api} online />)
    await waitFor(() => expect(peak).toHaveAttribute('data-confirm', 'idle'))
    fireEvent.click(screen.getByRole('tab', { name: 'TG' }))
    const tg = screen.getByRole('button', { name: /Generator output/ })
    fireEvent.click(tg)
    expect(api.set).not.toHaveBeenCalledWith('tg_output', 'ON')
    fireEvent.click(tg)
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('tg_output', 'ON'))
  })

  it('Rigol DSA815: full span, detector, sweep mode; no TG tab', async () => {
    const api = mockApi()
    render(<SalPanel status={{ ...status, DETECTOR: 'POS', TRACE_MODE: 'WRITE' }} profile={dsa} api={api} online />)
    expect(screen.queryByRole('tab', { name: 'TG' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Full span' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('full_span', 1))
    fireEvent.click(screen.getByRole('tab', { name: 'TRACE' }))
    fireEvent.click(screen.getByRole('button', { name: 'RMS' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('detector', 'RMS'))
    fireEvent.click(screen.getByRole('tab', { name: 'SWEEP' }))
    fireEvent.click(screen.getByRole('button', { name: 'Single' }))
    await waitFor(() => expect(api.set).toHaveBeenCalledWith('sweep', 'SINGLE'))
  })

  it('stepList', () => {
    expect(stepList([1, 3, 10], 3, 1)).toBe(10)
    expect(stepList([1, 3, 10], 5, -1)).toBe(3)
    expect(stepList([1, 3, 10], 10, 1)).toBe(10)
    expect(stepList([], 1, 1)).toBeNull()
  })
})

describe('ActionButton', () => {
  it('waits until the reported value says the action happened', async () => {
    vi.useFakeTimers()
    const press = vi.fn(async () => {})
    const { rerender } = render(<ActionButton label="Go" reported={1} done={(now, before) => now > before} onPress={press} />)
    const b = screen.getByRole('button', { name: 'Go' })
    await act(async () => { fireEvent.click(b) })
    expect(b).toHaveAttribute('data-confirm', 'waiting')
    await act(async () => { vi.advanceTimersByTime(6100) })
    expect(b).toHaveAttribute('data-confirm', 'unconfirmed')
    rerender(<ActionButton label="Go" reported={2} done={(now, before) => now > before} onPress={press} />)
    expect(b).toHaveAttribute('data-confirm', 'idle')
    vi.useRealTimers()
  })
})
