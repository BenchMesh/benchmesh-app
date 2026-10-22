/**
 * A port the operating system refused (the server's explained errors, bm-transport access.rs):
 * the message, the fix - a command with Copy (Linux), or a button to the settings pane (macOS, in
 * the desktop app) - and the help page section. The server ends such a message with
 * " - see <help URL>"; anything else is shown as it is.
 */
import React, { useState } from 'react'
import { PageLink, PAGE_URLS, type Page } from '../about/links'
import { isDesktop } from '../desktop'

export interface Fix {
  page: Page
  /** A terminal command that fixes it. */
  command?: string
  /** A settings pane the desktop app opens (the app runs on the computer that refused). */
  settings?: { page: Page; label: string }
  after: string
}

/** The same fixes as the website's /docs/linux and /docs/macos pages. */
export const FIXES: Fix[] = [
  {
    page: 'linux-usb',
    command: `echo 'SUBSYSTEM=="usbmisc", KERNEL=="usbtmc[0-9]*", MODE="0660", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/70-benchmesh-usbtmc.rules && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=usbmisc --action=add`,
    after: 'Run it in a terminal; BenchMesh reconnects by itself. Still refused? Unplug the instrument and plug it in again.',
  },
  {
    page: 'linux-serial',
    // dialout on Debian, Ubuntu, Fedora, openSUSE; uucp on Arch
    command: 'sudo usermod -aG "$(getent group dialout >/dev/null && echo dialout || echo uucp)" "$USER"',
    after: 'Run it in a terminal, then log out and back in.',
  },
  {
    page: 'macos-local-network',
    settings: { page: 'macos-network-settings', label: 'Open Local Network settings' },
    after: 'Switch BenchMesh on in the list; BenchMesh reconnects by itself.',
  },
]

/** The message without its help URL, and the fix it names; null when it names none. */
export function accessFix(text: string): { message: string; fix: Fix } | null {
  for (const fix of FIXES) {
    const url = PAGE_URLS[fix.page]
    // the driver's "i/o: " prefix says nothing to the user
    if (text.includes(url)) return { message: text.replace(` - see ${url}`, '').replace(url, '').replace(/^i\/o: /, '').trim(), fix }
  }
  return null
}

/** `text`, with the fix and the help link when it is a refused port. */
export function ProblemText({ text, className }: { text: string; className?: string }) {
  const found = accessFix(text)
  const [copied, setCopied] = useState<boolean | null>(null)
  if (!found) return <span className={className}>{text}</span>
  const { command, settings } = found.fix
  const copy = async () => {
    if (!command) return
    try { await navigator.clipboard.writeText(command); setCopied(true) } catch { setCopied(false) }
    setTimeout(() => setCopied(null), 2000)
  }
  return (
    <span className={`access-fix${className ? ` ${className}` : ''}`} data-testid="access-fix">
      <span>{found.message}</span>
      {command && <code className="access-fix__command">{command}</code>}
      <span className="access-fix__row">
        {command && (
          <button type="button" className="bench-btn" onClick={() => void copy()}>
            {copied ? 'Copied' : copied === false ? 'Copy failed - select the text' : 'Copy command'}
          </button>
        )}
        {settings && isDesktop() && <PageLink page={settings.page} className="bench-btn">{settings.label}</PageLink>}
        <PageLink page={found.fix.page}>How to fix</PageLink>
      </span>
      <span className="access-fix__after">{found.fix.after}</span>
    </span>
  )
}
