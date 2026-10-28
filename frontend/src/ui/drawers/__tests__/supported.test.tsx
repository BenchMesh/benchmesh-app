import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { filterCatalog, catalogClasses, communityText, communitySourceText, type CatalogFamily } from '../supported'
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

describe('community reports', () => {
  it('describe who confirmed a model on which versions', () => {
    expect(communityText({ status: 'confirmed', versions: ['0.1.5', 'v0.1.4'], reports: 2 })).toBe('Confirmed by 2 users on v0.1.5, v0.1.4')
    expect(communityText({ status: 'issues', reports: 1 })).toBe('Issues reported by 1 user')
    expect(communityText(null)).toBeNull()
  })
  it('say where the reports came from, or why they are missing', () => {
    expect(communitySourceText(undefined)).toMatch(/not loaded/)
    expect(communitySourceText({ source: 'https://x/api/v1/status', fetched_at: 't', error: 'HTTP 500', models: 0 })).toMatch(/could not be loaded \(HTTP 500\)/)
    expect(communitySourceText({ source: 'https://x/api/v1/status', fetched_at: 't', error: null, models: 3 })).toMatch(/3 models loaded/)
    expect(communitySourceText({ source: 'https://x/api/v1/status', fetched_at: 't', error: null, models: 1, issues: 2 })).toMatch(/1 model, 2 with a reported problem loaded/)
  })
})

describe('SupportedView', () => {
  it('marks community-confirmed models with the version they were confirmed on', async () => {
    const withCommunity: CatalogFamily[] = [{ ...families[1], models: [
      { name: 'DL3021', support: 'confirmed', community: { status: 'confirmed', versions: ['0.1.5'], reports: 1 } },
      { name: 'DL3031', support: 'awaiting', community: { status: 'issues', versions: ['0.1.5'], reports: 1 } }] }]
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => Promise.resolve(new Response(JSON.stringify({ families: withCommunity,
      models: { confirmed: 1, awaiting: 1 }, community: { source: 's', fetched_at: 't', error: null, models: 2 } }), { status: 200 })))
    render(<SupportedView apiBase="/api" />)
    const dl = await screen.findByText('DL3021')
    expect(dl.textContent).toContain('Community-confirmed')
    expect(dl.textContent).toContain('v0.1.5')
    expect(dl.getAttribute('title')).toContain('Confirmed by 1 user on v0.1.5')
    expect(screen.getByText('DL3031').textContent).toContain('issues reported')
    expect(screen.getByTestId('community-source').textContent).toMatch(/2 models loaded/)
    vi.restoreAllMocks()
  })

  it('flags models with a reported problem instead of the old issues mark', async () => {
    const flagged: CatalogFamily[] = [{ ...families[1], models: [
      { name: 'DL3021', support: 'awaiting', community: { status: 'issues', reports: 1 }, issue: { state: 'investigating', reports: 1 } },
      { name: 'DL3031', support: 'awaiting', issue: { state: 'fixed', fixed_in: '0.1.20', available: false } }] }]
    vi.spyOn(globalThis, 'fetch').mockImplementation(() => Promise.resolve(new Response(JSON.stringify({ families: flagged,
      models: { awaiting: 2 }, community: { source: 's', fetched_at: 't', error: null, models: 1, issues: 2 } }), { status: 200 })))
    render(<SupportedView apiBase="/api" />)
    const dl = await screen.findByText('DL3021')
    expect(dl.textContent).toContain('Issue reported - investigation pending')
    expect(dl.textContent).not.toContain('issues reported')
    expect(dl.getAttribute('title')).toContain('looking into it')
    expect(screen.getByText('DL3031').textContent).toContain('Fixed in 0.1.20 - next release')
    expect(screen.getAllByTestId('supported-issue')).toHaveLength(2)
  })

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
