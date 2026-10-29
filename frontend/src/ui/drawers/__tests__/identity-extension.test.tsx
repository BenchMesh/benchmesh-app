/** A build with an identity field and an Instruments section (the official build: Detect). */
import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import type { IdentityFieldProps, InstrumentsSectionProps } from '../../../extensions/contract'
import { InstrumentConfigView } from '../InstrumentConfigView'
import { mockApi } from './configApi'

vi.mock('../../../extensions', () => ({
  extensions: {
    instrumentsSections: [({ configured }: InstrumentsSectionProps) => <div data-testid="section">{configured.length} configured</div>],
    identityField: ({ match, path, onUseFixedPort }: IdentityFieldProps) => (
      <div data-testid="identity-field">{JSON.stringify(match)} on {path}<button type="button" onClick={onUseFixedPort}>Fixed</button></div>
    ),
  },
}))

afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('Instruments form with an identity field', () => {
  it('shows the sections and the identity instead of the port', async () => {
    mockApi()
    render(<InstrumentConfigView apiBase="/api" onConfigUpdated={() => {}} />)
    expect(await screen.findByText('1 configured')).toBeTruthy()
    fireEvent.click(await screen.findByText(/XDM/))
    expect((await screen.findByTestId('identity-field')).textContent).toContain('on /dev/ttyUSB2')
    expect(screen.queryByText('Serial port path')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Fixed' }))
    expect(await screen.findByText('Serial port path')).toBeTruthy()
  })
})
