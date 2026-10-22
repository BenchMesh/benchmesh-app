import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { accessFix, FIXES, ProblemText } from '../AccessFix'
import { PAGE_URLS } from '../../about/links'

// the server's wording (crates/bm-transport/src/access.rs)
const USB = 'no permission to use /dev/usbtmc0: Linux gives USB instruments to root only until a udev rule gives them to the logged-in user (one command, once) - see https://benchmesh.online/docs/linux#usb-permissions'
const MAC = '192.168.1.50:5025: no route to host. If the instrument is on and on this network, macOS is blocking BenchMesh\'s local network access: System Settings → Privacy & Security → Local Network, switch BenchMesh on - see https://benchmesh.online/docs/macos#local-network'
const SERIAL = 'cannot open: no permission to use /dev/ttyUSB0: Linux gives serial ports to the dialout group (uucp on Arch) - add your user to it and log in again - see https://benchmesh.online/docs/linux#serial-permissions'

afterEach(cleanup)

describe('accessFix', () => {
  it('finds the fix a refused USB instrument needs and drops the URL from the text', () => {
    const f = accessFix(USB)!
    expect(f.fix.page).toBe('linux-usb')
    expect(f.message).toBe('no permission to use /dev/usbtmc0: Linux gives USB instruments to root only until a udev rule gives them to the logged-in user (one command, once)')
    expect(f.fix.command).toContain('/etc/udev/rules.d/70-benchmesh-usbtmc.rules')
    expect(f.fix.command).toContain('TAG+="uaccess"')
  })

  it('finds the serial fix, also behind the Detect prefix', () => {
    const f = accessFix(SERIAL)!
    expect(f.fix.page).toBe('linux-serial')
    expect(f.message.startsWith('cannot open: no permission to use /dev/ttyUSB0')).toBe(true)
    expect(f.fix.command).toBe('sudo usermod -aG "$(getent group dialout >/dev/null && echo dialout || echo uucp)" "$USER"')
  })

  it('drops the driver prefix of a panel error', () => {
    expect(accessFix(`i/o: ${USB}`)!.message.startsWith('no permission to use /dev/usbtmc0')).toBe(true)
  })

  it('leaves any other message alone', () => {
    expect(accessFix('timeout (0 bytes received)')).toBeNull()
  })

  it('points every fix at its section of the Linux page', () => {
    expect(FIXES.map((f) => PAGE_URLS[f.page])).toEqual([
      'https://benchmesh.online/docs/linux#usb-permissions',
      'https://benchmesh.online/docs/linux#serial-permissions',
      'https://benchmesh.online/docs/macos#local-network',
    ])
  })
})

describe('ProblemText', () => {
  it('shows the command with Copy and the help link', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<ProblemText text={USB} />)
    expect(screen.getByTestId('access-fix').textContent).not.toContain('https://')
    fireEvent.click(screen.getByRole('button', { name: 'Copy command' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Copied' })).toBeTruthy())
    expect(writeText).toHaveBeenCalledWith(FIXES[0].command)
    expect(screen.getByRole('link', { name: 'How to fix' }).getAttribute('href')).toBe('https://benchmesh.online/docs/linux#usb-permissions')
  })

  it('macOS: a settings button in the desktop app, no command', () => {
    render(<ProblemText text={MAC} />)
    expect(screen.queryByRole('button', { name: 'Copy command' })).toBeNull()
    expect(screen.queryByRole('link', { name: 'Open Local Network settings' })).toBeNull() // a browser: not this computer's settings
    expect(screen.getByRole('link', { name: 'How to fix' }).getAttribute('href')).toBe('https://benchmesh.online/docs/macos#local-network')
    cleanup()
    const w = window as unknown as { __TAURI__?: unknown }
    w.__TAURI__ = { window: {}, webviewWindow: {} }
    try {
      render(<ProblemText text={MAC} />)
      expect(screen.getByRole('link', { name: 'Open Local Network settings' }).getAttribute('href'))
        .toBe('x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork')
    } finally { delete w.__TAURI__ }
  })

  it('shows any other message as plain text', () => {
    const { container } = render(<ProblemText text="not attached" />)
    expect(container.textContent).toBe('not attached')
    expect(screen.queryByRole('button')).toBeNull()
  })
})
