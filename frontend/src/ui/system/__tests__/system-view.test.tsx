import React from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { SystemView } from '../SystemView'
import * as api from '../client'
import type { Autostart, SystemInfo } from '../client'

const GB = 1024 ** 3
const info = (over: Partial<SystemInfo> = {}): SystemInfo => ({
  host: 'benchpi', os: 'Linux 12 Debian GNU/Linux', arch: 'aarch64', cpus: 4, uptime_s: 93784,
  load: [0.42, 0.3, 0.25], cpu_percent: 12.5, memory: { total: 4 * GB, used: 3.5 * GB },
  process: { pid: 1234, memory: 80 * 1024 ** 2, cpu_percent: 1.2, started_ms: 1000 },
  disk: { path: '/home/pi/.benchmesh', total: 32 * GB, available: 20 * GB },
  temperature_c: 72.4, headless: true, version: '0.1.15', ...over,
})
const autostart = (over: Partial<Autostart> = {}): Autostart => ({
  supported: true, enabled: false, method: 'systemd user service', location: '/home/pi/.config/systemd/user/benchmesh.service',
  starts_at: 'login', boot_command: 'sudo loginctl enable-linger "$USER"', note: 'Starts when you log in.', reason: null,
  command: '/usr/bin/benchmesh --headless', ...over,
})

beforeEach(() => {
  vi.spyOn(api, 'getAutostart').mockResolvedValue(autostart())
})
afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('Instruments - System', () => {
  it('shows the computer, its resources and BenchMesh', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue(info())
    render(<SystemView />)
    const computer = await screen.findByRole('group', { name: 'This computer' })
    expect(within(computer).getByText('benchpi')).toBeTruthy()
    expect(within(computer).getByText('aarch64, 4 cores')).toBeTruthy()
    expect(within(computer).getByText('1 d 2 h')).toBeTruthy()
    expect(within(computer).getByText('72.4 °C').getAttribute('data-state')).toBe('warn')
    expect(within(computer).getByText('0.42 / 0.30 / 0.25')).toBeTruthy()
    const memory = screen.getByRole('meter', { name: 'Memory' })
    expect(memory.getAttribute('aria-valuenow')).toBe('88')
    expect(memory.getAttribute('data-level')).toBe('warn')
    expect(within(memory).getByText('3.5 GB of 4.0 GB')).toBeTruthy()
    expect(within(screen.getByRole('meter', { name: 'Disk' })).getByText('20 GB free of 32 GB')).toBeTruthy()
    expect(screen.getByText(/Disk of the data folder \/home\/pi\/.benchmesh/)).toBeTruthy()
  })

  it('restarts and shuts down a headless BenchMesh only on a second press', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue(info())
    const shutdown = vi.spyOn(api, 'shutdown').mockResolvedValue({ action: 'shutdown' })
    render(<SystemView />)
    const button = await screen.findByRole('button', { name: 'Shut down' })
    fireEvent.click(button)
    expect(shutdown).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Press again: shut down' }))
    expect(await screen.findByText('BenchMesh is stopped')).toBeTruthy()
    expect(shutdown).toHaveBeenCalledTimes(1)
  })

  it('says why a restart was refused', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue(info())
    vi.spyOn(api, 'restart').mockRejectedValue(new Error('a flow is running: stop it first'))
    render(<SystemView />)
    fireEvent.click(await screen.findByRole('button', { name: 'Restart' }))
    fireEvent.click(screen.getByRole('button', { name: 'Press again: restart' }))
    expect((await screen.findByRole('alert')).textContent).toMatch(/a flow is running/)
    expect(screen.getByRole('group', { name: 'This computer' })).toBeTruthy()
  })

  it('switches start with the computer on and offers the power-on command', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue(info())
    const set = vi.spyOn(api, 'setAutostart').mockResolvedValue(autostart({ enabled: true }))
    render(<SystemView />)
    const sw = await screen.findByRole('switch', { name: 'Start with this computer' })
    expect(sw.getAttribute('aria-checked')).toBe('false')
    expect(screen.queryByText('sudo loginctl enable-linger "$USER"')).toBeNull()
    fireEvent.click(sw)
    expect(await screen.findByText('sudo loginctl enable-linger "$USER"')).toBeTruthy()
    expect(set).toHaveBeenCalledWith(true)
    expect(screen.getByRole('switch', { name: 'Start with this computer' }).getAttribute('aria-checked')).toBe('true')
    expect(screen.getByText(/Starts when you log in \(systemd user service\)/)).toBeTruthy()
  })

  it('in the desktop app: resources only, and how to run headless', async () => {
    vi.spyOn(api, 'getSystem').mockResolvedValue(info({ headless: false, load: null, temperature_c: null }))
    render(<SystemView />)
    await screen.findByRole('group', { name: 'BenchMesh' })
    expect(screen.queryByRole('button', { name: 'Restart' })).toBeNull()
    expect(screen.queryByRole('switch')).toBeNull()
    expect(screen.getByText(/runs in the desktop app: closing its window stops it/)).toBeTruthy()
    expect(screen.getByRole('link', { name: 'Headless BenchMesh' }).getAttribute('href')).toBe('https://benchmesh.online/docs/headless')
    expect(screen.queryByText(/°C/)).toBeNull()
  })
})
