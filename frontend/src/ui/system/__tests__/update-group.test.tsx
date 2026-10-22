import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { UpdateGroup } from '../UpdateGroup'
import * as api from '../client'
import type { UpdateState } from '../client'

const state = (over: Partial<UpdateState> = {}): UpdateState => ({
  current: '0.1.14', installed_as: 'AppImage', can_install: true, replaces_itself: true, reason: null, checked_ms: null,
  latest: null, available: false, phase: 'idle', downloaded: 0, total: null, error: null, command: null, ...over,
})
const newer = { version: '0.1.15', notes: 'Headless updates', pub_date: '2026-10-23T10:00:00Z' }

afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('Instruments - System - Updates', () => {
  it('checks and says when BenchMesh is up to date', async () => {
    vi.spyOn(api, 'getUpdate').mockResolvedValue(state())
    const check = vi.spyOn(api, 'checkUpdate').mockResolvedValue(state({ checked_ms: 1, latest: { ...newer, version: '0.1.14' } }))
    render(<UpdateGroup viewOnly={false} onInstalling={() => {}} />)
    expect(await screen.findByText('Installed: BenchMesh 0.1.14 (AppImage).')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }))
    expect(await screen.findByText('BenchMesh is up to date.')).toBeTruthy()
    expect(check).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole('button', { name: /Install/ })).toBeNull()
  })

  it('installs a newer version only on a second press and hands over to the restart', async () => {
    vi.spyOn(api, 'getUpdate').mockResolvedValue(state({ checked_ms: 1, latest: newer, available: true }))
    const install = vi.spyOn(api, 'installUpdate').mockResolvedValue(state({ latest: newer, available: true, phase: 'installing' }))
    const onInstalling = vi.fn()
    render(<UpdateGroup viewOnly={false} onInstalling={onInstalling} />)
    expect(await screen.findByText('BenchMesh 0.1.15 is available (published 2026-10-23).')).toBeTruthy()
    expect(screen.getByText('Headless updates')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Install 0.1.15 and restart' }))
    expect(install).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Press again: stop, install, start' }))
    await vi.waitFor(() => expect(onInstalling).toHaveBeenCalled())
    expect(install).toHaveBeenCalledTimes(1)
  })

  it('follows the download and then shows the apt command of a .deb', async () => {
    const deb = { installed_as: '.deb package', replaces_itself: false, latest: newer, available: true }
    vi.spyOn(api, 'getUpdate')
      .mockResolvedValueOnce(state({ ...deb, phase: 'downloading', downloaded: 5 * 1024 ** 2, total: 20 * 1024 ** 2 }))
      .mockResolvedValue(state({ ...deb, phase: 'manual', command: 'sudo apt install /home/pi/.benchmesh/updates/BenchMesh_0.1.15_arm64.deb' }))
    render(<UpdateGroup viewOnly={false} onInstalling={() => {}} />)
    const meter = await screen.findByRole('meter', { name: 'Download' })
    expect(meter.getAttribute('aria-valuenow')).toBe('25')
    expect(within(meter).getByText('5.0 MB of 20 MB')).toBeTruthy()
    expect(await screen.findByText('sudo apt install /home/pi/.benchmesh/updates/BenchMesh_0.1.15_arm64.deb', undefined, { timeout: 3000 })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Copy command' })).toBeTruthy()
    expect(screen.queryByRole('meter', { name: 'Download' })).toBeNull()
  })

  it('says why this installation does not install and shows a failure', async () => {
    vi.spyOn(api, 'getUpdate').mockResolvedValue(state({ installed_as: 'desktop app', can_install: false, reason: 'the desktop app offers new versions when it starts', latest: newer, available: true }))
    render(<UpdateGroup viewOnly={false} onInstalling={() => {}} />)
    expect(await screen.findByText('The desktop app offers new versions when it starts.')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /Install/ })).toBeNull()
    cleanup()
    vi.spyOn(api, 'getUpdate').mockResolvedValue(state({ phase: 'failed', error: 'the download is not signed by BenchMesh: not installed' }))
    render(<UpdateGroup viewOnly={false} onInstalling={() => {}} />)
    expect((await screen.findByRole('alert')).textContent).toMatch(/not signed by BenchMesh/)
  })

  it('a view-only session cannot check or install', async () => {
    vi.spyOn(api, 'getUpdate').mockResolvedValue(state({ latest: newer, available: true }))
    render(<UpdateGroup viewOnly onInstalling={() => {}} />)
    expect((await screen.findByRole('button', { name: 'Check for updates' })).hasAttribute('disabled')).toBe(true)
    expect(screen.getByRole('button', { name: 'Install 0.1.15 and restart' }).hasAttribute('disabled')).toBe(true)
  })
})
