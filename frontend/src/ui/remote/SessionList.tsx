/** The signed-in remote browsers (address, browser, since) with a Disconnect each. */
import React, { useState } from 'react'
import { errorText } from '../automation/client'
import type { RemoteSession } from './client'
import { useRemote } from './RemoteProvider'
import { shortAgent, sinceText } from './format'

export function SessionList({ sessions }: { sessions: RemoteSession[] }) {
  const remote = useRemote()
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  if (!remote) return null

  const disconnect = async (id: RemoteSession['id']) => {
    setBusy(String(id))
    setError(null)
    try { await remote.client.disconnect(id); await remote.refresh() } catch (e) { setError(errorText(e)) } finally { setBusy(null) }
  }

  if (!sessions.length) return <span className="bench-hint">Nobody is connected.</span>
  return (
    <>
      <ul className="remote-sessions" aria-label="Remote sessions">
        {sessions.map((s) => (
          <li key={String(s.id)} className="remote-sessions__item">
            <span className="remote-sessions__who">
              <b>{s.address}</b>
              {s.role === 'view' && <span className="remote-sessions__role">view only</span>}
              <span title={s.agent}>{shortAgent(s.agent)}</span>
              <span>since {sinceText(s.since)}</span>
            </span>
            <button type="button" className="bench-btn" disabled={busy === String(s.id)} aria-busy={busy === String(s.id)}
              onClick={() => void disconnect(s.id)} aria-label={`Disconnect ${s.address}`}>Disconnect</button>
          </li>
        ))}
      </ul>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </>
  )
}
