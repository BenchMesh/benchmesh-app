/** Instruments - System shows the groups a distribution adds (`src/extensions/contract.ts`). */
import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import type { SystemGroupProps } from '../../../extensions/contract'
import { SystemView } from '../SystemView'
import * as api from '../client'

vi.mock('../../../extensions', () => ({
  extensions: {
    root: [],
    systemGroups: [({ viewOnly, onStopping }: SystemGroupProps) => (
      <div role="group" aria-label="Example">
        <button type="button" disabled={viewOnly} onClick={() => onStopping({ title: 'Doing the example…', hint: 'BenchMesh stops and starts again.' })}>Go</button>
      </div>
    )],
  },
}))

afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('Instruments - System with an extension', () => {
  it('shows its group and waits while it has BenchMesh stopped', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue({
      host: 'bench', os: 'Linux', arch: 'x86_64', cpus: 2, uptime_s: 60, load: null, cpu_percent: 1, memory: { total: 1, used: 0 },
      process: { pid: 1, memory: 1, cpu_percent: 0, started_ms: 1000 }, disk: null, temperature_c: null, headless: false, version: '0.1.16',
    })
    render(<SystemView />)
    fireEvent.click(await screen.findByRole('button', { name: 'Go' }))
    expect(await screen.findByText('Doing the example…')).toBeTruthy()
    expect(screen.getByText('BenchMesh stops and starts again. This page reloads by itself.')).toBeTruthy()
  })
})
