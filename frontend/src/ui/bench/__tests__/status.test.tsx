import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { apiStatus, readingsStatus } from '../status'
import { StatusPill } from '../StatusPill'

describe('top bar statuses', () => {
  it('API: answers, first answer pending, unreachable with the reason', () => {
    expect(apiStatus(false, null)).toMatchObject({ label: 'API', state: 'ok' })
    expect(apiStatus(true, null)).toMatchObject({ label: 'API connecting…', state: 'warn' })
    const down = apiStatus(false, 'HTTP 500')
    expect(down).toMatchObject({ label: 'API unreachable', state: 'alert' })
    expect(down.title).toContain('HTTP 500')
  })

  it('Readings: current, stalled, lost, connecting', () => {
    expect(readingsStatus('receiving')).toMatchObject({ label: 'Readings', state: 'ok' })
    expect(readingsStatus('open')).toMatchObject({ label: 'Readings', state: 'ok' })
    expect(readingsStatus('stale')).toMatchObject({ label: 'Readings stalled', state: 'warn' })
    expect(readingsStatus('closed')).toMatchObject({ label: 'Readings lost', state: 'alert' })
    expect(readingsStatus('connecting')).toMatchObject({ label: 'Readings connecting…', state: 'warn' })
  })

  it('every status explains itself and tells the window link apart from the instruments', () => {
    expect(apiStatus(false, null).title).toMatch(/BenchMesh server answers this window/)
    expect(readingsStatus('receiving').title).toMatch(/instrument itself answers shows on its panel/)
  })

  it('labels start with a capital (one casing for the whole top bar)', () => {
    const labels = [apiStatus(false, null), apiStatus(true, null), apiStatus(false, 'x'), ...(['receiving', 'stale', 'closed', 'connecting'] as const).map(readingsStatus)].map((s) => s.label)
    for (const l of labels) expect(l[0]).toBe(l[0].toUpperCase())
  })
})

describe('StatusPill', () => {
  it('a status with a tooltip; a button when it opens something', () => {
    const open = vi.fn()
    const { rerender } = render(<StatusPill label="Readings" title="current" state="ok" />)
    const pill = screen.getByText('Readings')
    expect(pill.getAttribute('title')).toBe('current')
    expect(pill.getAttribute('data-ok')).toBe('true')
    expect(screen.queryByRole('button')).toBeNull()
    rerender(<StatusPill label="Not linked" title="why link" state="warn" onOpen={open} />)
    const b = screen.getByRole('button', { name: 'Not linked' })
    expect(b.getAttribute('data-state')).toBe('warn')
    expect(b.getAttribute('data-ok')).toBe('false')
    fireEvent.click(b)
    expect(open).toHaveBeenCalled()
  })
})
