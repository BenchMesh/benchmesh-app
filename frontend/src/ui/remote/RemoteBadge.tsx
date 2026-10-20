/**
 * Top-bar remote access indicator. In the app (local principal), while remote browsers are
 * connected: **Remote: N**, opening the list of sessions with a Disconnect each and a red
 * **Disconnect all and switch off**. In a remote browser: a **Remote** label and **Log out**.
 */
import React, { useEffect, useRef, useState } from 'react'
import { errorText } from '../automation/client'
import { logoutUrl } from './client'
import { useRemote } from './RemoteProvider'
import { SessionList } from './SessionList'
import './remote.css'

/** A plain form post: the browser follows the server's 303 to the sign-in page. */
export function LogoutButton() {
  return (
    <form method="post" action={logoutUrl()} className="remote-logout">
      <button type="submit" className="bench-btn">Log out</button>
    </form>
  )
}

export function RemoteBadge() {
  const remote = useRemote()
  const [opened, setOpen] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const root = useRef<HTMLDivElement>(null)

  const sessions = remote?.status?.enabled ? remote.status.sessions : []
  const shown = sessions.length > 0
  const open = opened && shown

  // the popover closes on Escape and on a press outside it
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false) }
    const onDown = (e: PointerEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false) }
    window.addEventListener('keydown', onKey)
    window.addEventListener('pointerdown', onDown)
    return () => { window.removeEventListener('keydown', onKey); window.removeEventListener('pointerdown', onDown) }
  }, [open])

  if (!remote) return null
  if (remote.principal === 'remote') {
    return (
      <div className="remote-badge" role="status" aria-label="Remote access">
        <span className="remote-badge__label" title="This bench is controlled from a browser over the network">Remote</span>
        <LogoutButton />
      </div>
    )
  }
  if (!shown) return null

  const switchOff = async () => {
    setBusy(true)
    setError(null)
    try { remote.setStatus(await remote.client.update({ enabled: false })) } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }

  return (
    <div className="remote-badge" ref={root}>
      <button type="button" className="remote-badge__open" aria-expanded={open} aria-haspopup="dialog" onClick={() => setOpen((o) => !o)}
        title={sessions.map((s) => s.address).join(', ')}>
        Remote: {sessions.length}
      </button>
      {open && (
        <div className="remote-pop" role="dialog" aria-label="Remote sessions">
          <span className="kit-label">Connected over the network</span>
          <SessionList sessions={sessions} />
          <button type="button" className="bench-btn remote-btn--off" onClick={() => void switchOff()} disabled={busy} aria-busy={busy}>
            Disconnect all and switch off
          </button>
          {error && <span className="kit-error" role="alert">{error}</span>}
        </div>
      )}
    </div>
  )
}
