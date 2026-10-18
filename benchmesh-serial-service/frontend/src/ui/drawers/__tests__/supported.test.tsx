import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { filterCatalog, catalogClasses, type CatalogFamily } from '../supported'
import { SupportedView } from '../SupportedView'

const families: CatalogFamily[] = [
  { driver: 'tenma_72', vendor: 'TENMA', family: '72-series PSU', classes: ['PSU'], transports: ['serial'],
    models: [{ name: '72-2540', support: 'tested' }, { name: '72-2545', support: 'awaiting' }] },
  { driver: 'rigol_dl3000', vendor: 'RIGOL', family: 'DL3000 load', classes: ['ELL'], transports: ['usbtmc', 'tcpip'],
    models: [{ name: 'DL3021', support: 'awaiting' }] },
  { driver: 'owon_spm', vendor: 'OWON', family: 'SPM', classes: ['DMM', 'PSU'], transports: ['serial'],
    models: [{ name: 'SPM3103', support: 'tested' }] },
]

describe('filterCatalog', () => {
  it('filters by class, status and words across vendor, family and model', () => {
    expect(filterCatalog(families, { text: '', cls: 'PSU', support: null }).map((f) => f.driver)).toEqual(['owon_spm', 'tenma_72'])
    const awaiting = filterCatalog(families, { text: '', cls: null, support: 'awaiting' })
    expect(awaiting.map((f) => [f.driver, f.models.map((m) => m.name)])).toEqual([['rigol_dl3000', ['DL3021']], ['tenma_72', ['72-2545']]])
    expect(filterCatalog(families, { text: 'tenma 2540', cls: null, support: null })[0].models.map((m) => m.name)).toEqual(['72-2540'])
    expect(filterCatalog(families, { text: 'keysight', cls: null, support: null })).toEqual([])
  })
  it('lists the classes present', () => {
    expect(catalogClasses(families)).toEqual(['DMM', 'ELL', 'PSU'])
  })
})

describe('SupportedView', () => {
  afterEach(() => vi.restoreAllMocks())
  it('shows families with each model’s status and filters by status', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => Promise.resolve(new Response(
      JSON.stringify({ families, models: { tested: 2, awaiting: 2 } }), { status: 200 })))
    render(<SupportedView apiBase="/api" />)
    expect(await screen.findAllByTestId('supported-family')).toHaveLength(3)
    expect(screen.getByText('DL3021').textContent).toContain('Awaiting confirmation')
    fireEvent.click(screen.getByRole('button', { name: /Bench-tested/ }))
    expect(screen.getAllByTestId('supported-family')).toHaveLength(2)
    expect(screen.queryByText('DL3021')).toBeNull()
  })
})
