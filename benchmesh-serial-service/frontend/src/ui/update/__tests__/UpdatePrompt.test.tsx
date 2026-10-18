import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { UpdatePrompt } from '../UpdatePrompt'

type Handler = (cmd: string) => Promise<unknown>

function fakeTauri(handle: Handler) {
  const listeners: Record<string, (e: { payload: unknown }) => void> = {}
  const invoke = vi.fn(handle)
  ;(window as any).__TAURI__ = {
    core: { invoke },
    event: { listen: vi.fn(async (ev: string, cb: (e: { payload: unknown }) => void) => { listeners[ev] = cb; return () => { delete listeners[ev] } }) },
  }
  return { invoke, listeners }
}

const available = { version: '0.2.0', current: '0.1.1', notes: 'Fixes', canInstall: true }

afterEach(() => { delete (window as any).__TAURI__ })

describe('UpdatePrompt', () => {
  it('shows nothing in a browser (no desktop app)', () => {
    const { container } = render(<UpdatePrompt />)
    expect(container).toBeEmptyDOMElement()
  })

  it('shows nothing when there is no newer version', async () => {
    const { invoke } = fakeTauri(async () => null)
    const { container } = render(<UpdatePrompt />)
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('update_check'))
    expect(container).toBeEmptyDOMElement()
  })

  it('offers the newer version and can be dismissed', async () => {
    fakeTauri(async () => available)
    render(<UpdatePrompt />)
    expect(await screen.findByRole('dialog')).toHaveTextContent('BenchMesh 0.2.0 is available — you have 0.1.1')
    fireEvent.click(screen.getByText('Later'))
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('installs with progress and reports a failure', async () => {
    let fail: (e: Error) => void = () => {}
    const { invoke, listeners } = fakeTauri(async (cmd) => {
      if (cmd === 'update_check') return available
      return new Promise((_, reject) => { fail = reject })
    })
    render(<UpdatePrompt />)
    fireEvent.click(await screen.findByText('Update and restart'))
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('update_install'))
    listeners['update://progress']({ payload: { downloaded: 50, total: 200 } })
    expect(await screen.findByText(/Downloading… 25%/)).toBeInTheDocument()
    fail(new Error('download failed: offline'))
    expect(await screen.findByRole('alert')).toHaveTextContent('download failed: offline')
    expect(screen.getByText('Update and restart')).not.toBeDisabled()
  })

  it('offers the download page where the app cannot replace itself (.deb)', async () => {
    const { invoke } = fakeTauri(async (cmd) => (cmd === 'update_check' ? { ...available, canInstall: false } : undefined))
    render(<UpdatePrompt />)
    fireEvent.click(await screen.findByText('Download'))
    expect(invoke).toHaveBeenCalledWith('update_open_page')
    expect(screen.queryByRole('dialog')).toBeNull()
  })
})
