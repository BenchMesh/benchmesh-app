import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { AboutView } from '../AboutView'
import { PageLink, PAGE_URLS, SUPPORT_EMAIL } from '../links'
import { fetchVersion } from '../version'

const answer = (body: unknown, status = 200) => vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify(body), { status }))

afterEach(() => {
  vi.restoreAllMocks()
  delete (window as { __TAURI__?: unknown }).__TAURI__
})

describe('version', () => {
  it('reads the release version from /version', async () => {
    answer({ releaseVersion: '0.1.7', version: '0.1.7', displayVersion: 'BenchMesh 0.1.7' })
    expect(await fetchVersion()).toEqual({ version: '0.1.7', display: 'BenchMesh 0.1.7' })
  })
  it('is null when the server does not answer', async () => {
    answer({}, 500)
    expect(await fetchVersion()).toBeNull()
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('offline'))
    expect(await fetchVersion()).toBeNull()
  })
})

describe('About and feedback', () => {
  it('shows the version and links to the GitHub issues', async () => {
    answer({ version: '0.1.7' })
    render(<AboutView />)
    expect(await screen.findByTestId('app-version')).toHaveTextContent('BenchMesh 0.1.7')
    expect(screen.getByRole('link', { name: 'Report an issue' })).toHaveAttribute('href', 'https://github.com/MarkoVcode/BenchMesh/issues/new')
    expect(screen.getByRole('link', { name: 'See open issues' })).toHaveAttribute('href', PAGE_URLS.issues)
    expect(screen.getByText(PAGE_URLS.issues)).toBeInTheDocument()
  })

  it('offers the support email: a mail link and Copy email', async () => {
    answer({ version: '0.1.7' })
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<AboutView />)
    const mail = screen.getByRole('link', { name: 'support@benchmesh.online' })
    expect(mail).toHaveAttribute('href', 'mailto:support@benchmesh.online')
    expect(mail).not.toHaveAttribute('target')
    fireEvent.click(screen.getByRole('button', { name: 'Copy email' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(SUPPORT_EMAIL))
    expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument()
  })

  it('copies the version', async () => {
    answer({ version: '0.1.7' })
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<AboutView />)
    fireEvent.click(await screen.findByRole('button', { name: 'Copy version' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('BenchMesh 0.1.7'))
  })

  it('in the desktop app a link opens the system browser by page name', async () => {
    const invoke = vi.fn().mockResolvedValue(undefined)
    ;(window as { __TAURI__?: unknown }).__TAURI__ = { core: { invoke } }
    render(<PageLink page="new-issue">Report</PageLink>)
    const link = screen.getByRole('link', { name: 'Report' })
    const ev = new MouseEvent('click', { bubbles: true, cancelable: true })
    link.dispatchEvent(ev)
    expect(ev.defaultPrevented).toBe(true)
    expect(invoke).toHaveBeenCalledWith('open_page', { page: 'new-issue' })
  })

  it('in a browser the link is followed in a new tab', () => {
    render(<PageLink page="issues">Issues</PageLink>)
    const link = screen.getByRole('link', { name: 'Issues' })
    expect(link).toHaveAttribute('target', '_blank')
    const ev = new MouseEvent('click', { bubbles: true, cancelable: true })
    link.dispatchEvent(ev)
    expect(ev.defaultPrevented).toBe(false)
  })
})
