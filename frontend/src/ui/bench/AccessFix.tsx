/**
 * A port Linux refused (the server's "no permission to use …" message, bm-transport access.rs):
 * the message, the one command that fixes it with Copy, and the help page section. The server
 * ends such a message with " - see <help URL>"; anything else is shown as it is.
 */
import React, { useState } from 'react'
import { PageLink, PAGE_URLS, type Page } from '../about/links'

export interface Fix { page: Page; command: string; after: string }

/** The same commands as the website's /docs/linux page. */
export const FIXES: Fix[] = [
  {
    page: 'linux-usb',
    command: `echo 'SUBSYSTEM=="usbmisc", KERNEL=="usbtmc[0-9]*", MODE="0660", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/70-benchmesh-usbtmc.rules && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=usbmisc --action=add`,
    after: 'Run it in a terminal; BenchMesh reconnects by itself. Still refused? Unplug the instrument and plug it in again.',
  },
  {
    page: 'linux-serial',
    command: 'sudo usermod -aG dialout "$USER"',
    after: 'Run it in a terminal, then log out and back in (on Arch the group is uucp).',
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

/** `text`, with the command and the help link when it is a refused port. */
export function ProblemText({ text, className }: { text: string; className?: string }) {
  const found = accessFix(text)
  const [copied, setCopied] = useState<boolean | null>(null)
  if (!found) return <span className={className}>{text}</span>
  const copy = async () => {
    try { await navigator.clipboard.writeText(found.fix.command); setCopied(true) } catch { setCopied(false) }
    setTimeout(() => setCopied(null), 2000)
  }
  return (
    <span className={`access-fix${className ? ` ${className}` : ''}`} data-testid="access-fix">
      <span>{found.message}</span>
      <code className="access-fix__command">{found.fix.command}</code>
      <span className="access-fix__row">
        <button type="button" className="bench-btn" onClick={() => void copy()}>
          {copied ? 'Copied' : copied === false ? 'Copy failed - select the text' : 'Copy command'}
        </button>
        <PageLink page={found.fix.page}>How to fix</PageLink>
      </span>
      <span className="access-fix__after">{found.fix.after}</span>
    </span>
  )
}
