import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import { displayedMode } from './dmmMode'
import { GenericDMM } from './GenericDMM'
import { MeasurementProvider } from '../../MeasurementContext'
import { RequestLogProvider } from '../../RequestLogContext'

describe('displayedMode', () => {
  const modes = ['VOLT', 'VOLT_AC', 'RES']
  it('follows the instrument when it reports a known mode', () => {
    expect(displayedMode('VOLT', 'VOLT_AC', modes)).toBe('VOLT_AC')
  })
  it('keeps the selection when the mode is unknown or not reported (e.g. settling)', () => {
    expect(displayedMode('RES', null, modes)).toBe('RES')
    expect(displayedMode('RES', 'FOO', modes)).toBe('RES')
  })
  it('falls back to the first mode', () => {
    expect(displayedMode('', undefined, modes)).toBe('VOLT')
    expect(displayedMode('', undefined, [])).toBe('')
  })
})

describe('GenericDMM mode dropdown', () => {
  it('shows the mode reported by the instrument', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ modes: { VOLT: { ui_name: 'Voltage DC' }, VOLT_AC: { ui_name: 'Voltage AC' } } }),
      headers: { get: () => null },
    } as any)
    const registry = { 'dmm-1': { DMM: { status_ch1: { MEAS: { si: { number: '1' } }, MODE: 'VOLT_AC' } } } }
    render(
      <RequestLogProvider><MeasurementProvider>
        <GenericDMM channelPath="/instruments/DMM/dmm-1/1" registry={registry} />
      </MeasurementProvider></RequestLogProvider>
    )
    expect(await screen.findByText('Voltage AC')).toBeInTheDocument()
  })
})
