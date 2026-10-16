import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import { InstrumentPod } from '../InstrumentPod'
import { MeasurementProvider } from '../MeasurementContext'
import { RequestLogProvider } from '../RequestLogContext'

const baseInstrument = {
  id: 'psu-1',
  IDN: 'ACME PSU-1000',
  classes: [
    { class: 'PSU', channels: ['/instruments/PSU/psu-1/1'] }
  ]
}

describe('InstrumentPod', () => {
  it('renders instrument id and IDN', () => {
    render(
      <RequestLogProvider><MeasurementProvider>
        <InstrumentPod instrument={baseInstrument as any} registry={{}} />
      </MeasurementProvider></RequestLogProvider>
    )
    expect(screen.getByText('psu-1')).toBeInTheDocument()
    expect(screen.getByText('ACME PSU-1000')).toBeInTheDocument()
  })

  it('shows the PSU face under the PSU class', () => {
    render(
      <RequestLogProvider><MeasurementProvider>
        <InstrumentPod instrument={baseInstrument as any} registry={{}} />
      </MeasurementProvider></RequestLogProvider>
    )
    // PSU face now renders Settings and Readings columns
    expect(screen.getByText('Settings')).toBeInTheDocument()
    expect(screen.getByText('Readings')).toBeInTheDocument()
    // No editable P in Settings; P present in Readings only
    expect(screen.getAllByText('U').length).toBeGreaterThanOrEqual(2)
    expect(screen.getAllByText('I').length).toBeGreaterThanOrEqual(2)
  })
})
