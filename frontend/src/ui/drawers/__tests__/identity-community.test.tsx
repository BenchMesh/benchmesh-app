/** The open-source build: an entry's `match` is kept but not followed; the named port is shown. */
import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { InstrumentConfigView } from '../InstrumentConfigView'
import { mockApi } from './configApi'

// what the open-source build adds (index.ts), also when the official build runs this test
vi.mock('../../../extensions', () => ({ extensions: { instrumentsSections: [], identityField: null } }))

afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('Instruments form without an identity field', () => {
  it('shows the port and says the identity is not followed', async () => {
    mockApi()
    render(<InstrumentConfigView apiBase="/api" onConfigUpdated={() => {}} />)
    fireEvent.click(await screen.findByText(/XDM/))
    expect(await screen.findByTestId('identity-unfollowed')).toBeTruthy()
    expect(screen.queryByTestId('identity-field')).toBeNull()
    expect(screen.getByText('Serial port path')).toBeTruthy()
    expect(screen.queryByTestId('detect-btn')).toBeNull()
  })
})
