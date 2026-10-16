import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import React from 'react'
import { GenericAWG } from './GenericAWG'
import { RequestLogProvider } from '../../RequestLogContext'

const registry = {
  'gen-1': {
    AWG: {
      status_ch2: {
        OUTPUT: 'OFF',
        SHAPE: 'SIN',
        FREQ: { si: { number: '1', symbol: 'k' }, sci: '1.00000E+03', val: '1000' },
        AMPL: { si: { number: '7' }, sci: '7.00000E+00', val: '7' },
        OFFS: { si: { number: '0' }, sci: '0.00000E+00', val: '0' },
        PHASE: 0,
        SYMM: 50,
      },
    },
  },
}

function renderAwg() {
  return render(
    <RequestLogProvider>
      <GenericAWG channelPath="/instruments/AWG/gen-1/2" registry={registry} />
    </RequestLogProvider>
  )
}

describe('GenericAWG', () => {
  it('shows the live channel status from the registry', () => {
    renderAwg()
    expect(screen.getByText('Channel 2')).toBeInTheDocument()
    expect(screen.getByText('SIN')).toBeInTheDocument()
    expect(screen.getByText('1 kHz')).toBeInTheDocument()
    expect(screen.getByText('7 Vpp')).toBeInTheDocument()
    expect(screen.getByText('0 V')).toBeInTheDocument()
    expect(screen.getByText('0°')).toBeInTheDocument()
    expect(screen.queryByText('Symmetry')).toBeNull() // only for RAMP
  })

  it('toggles the output with a POST to the channel', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 204, json: async () => ({}), headers: { get: () => null } })
    global.fetch = fetchMock as any
    renderAwg()
    fireEvent.click(screen.getByRole('button', { name: 'Output OFF' }))
    await waitFor(() => expect(fetchMock).toHaveBeenCalled())
    const [url, init] = fetchMock.mock.calls[0]
    expect(String(url)).toMatch(/\/instruments\/AWG\/gen-1\/2\/output\/ON$/)
    expect(init.method).toBe('POST')
  })
})
