/**
 * Instruments - System - Updates: check the latest published release and, for a headless
 * BenchMesh, install it (AppImage, Windows, macOS: BenchMesh stops, installs the signed download
 * and starts again; .deb: the verified package and the apt command to run). The desktop app offers
 * new versions itself when it starts (contract `ai_context/HEADLESS.md`).
 */
import { useCallback, useEffect, useState } from 'react'
import * as api from './client'
import type { UpdateState } from './client'
import { formatBytes } from './format'
import { CopyCommand, Meter, TwoPress } from './parts'

const BUSY_POLL_MS = 1000

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))
const capital = (s: string) => s[0].toUpperCase() + s.slice(1)

/** `onInstalling`: BenchMesh is about to stop and start the new version. */
export function UpdateGroup({ viewOnly, onInstalling }: { viewOnly: boolean; onInstalling: () => void }) {
  const [u, setU] = useState<UpdateState | null>(null)
  const [error, setError] = useState<string | null>(null)

  const show = useCallback((s: UpdateState) => {
    setU(s)
    if (s.phase === 'installing') onInstalling()
  }, [onInstalling])

  useEffect(() => {
    api.getUpdate().then(show, (e) => setError(message(e)))
  }, [show])

  const busy = u?.phase === 'checking' || u?.phase === 'downloading'
  useEffect(() => {
    if (!busy) return
    const t = setInterval(() => { api.getUpdate().then(show, () => { /* next tick */ }) }, BUSY_POLL_MS)
    return () => clearInterval(t)
  }, [busy, show])

  const run = async (call: () => Promise<UpdateState>) => {
    setError(null)
    try { show(await call()) } catch (e) { setError(message(e)) }
  }

  if (!u) return error ? <div className="bench-group" role="group" aria-label="Updates"><span className="kit-error" role="alert">{error}</span></div> : null
  const latest = u.latest?.version
  const percent = u.total ? (u.downloaded / u.total) * 100 : 0
  return (
    <div className="bench-group" role="group" aria-label="Updates">
      <span className="kit-label">Updates</span>
      <p className="bench-hint">Installed: BenchMesh {u.current} ({u.installed_as}).</p>
      <div className="bench-group__row">
        <button type="button" className="bench-btn" disabled={viewOnly || busy} aria-busy={u.phase === 'checking'} onClick={() => void run(api.checkUpdate)}>
          {u.phase === 'checking' ? 'Checking…' : 'Check for updates'}
        </button>
        {u.can_install && u.available && latest && u.phase !== 'downloading' && u.phase !== 'manual' && (
          <TwoPress
            label={u.replaces_itself ? `Install ${latest} and restart` : `Download ${latest}`}
            confirmLabel={u.replaces_itself ? 'Press again: stop, install, start' : 'Press again: download'}
            disabled={viewOnly}
            onConfirm={() => run(api.installUpdate)}
          />
        )}
      </div>
      {u.checked_ms != null && u.phase !== 'failed' && !u.available && <p className="bench-hint">BenchMesh is up to date.</p>}
      {u.available && latest && <p className="bench-hint">BenchMesh {latest} is available{u.latest?.pub_date ? ` (published ${u.latest.pub_date.slice(0, 10)})` : ''}.</p>}
      {u.available && u.latest?.notes && <p className="bench-hint sys-notes">{u.latest.notes}</p>}
      {u.phase === 'downloading' && (
        <Meter label="Download" percent={percent} level="ok" text={u.total ? `${formatBytes(u.downloaded)} of ${formatBytes(u.total)}` : formatBytes(u.downloaded)} />
      )}
      {u.available && u.can_install && u.replaces_itself && u.phase !== 'downloading' && (
        <p className="bench-hint">The download is installed only when its signature is BenchMesh&apos;s. The instruments stop cleanly first; remote browsers sign in again afterwards.</p>
      )}
      {u.phase === 'manual' && u.command && (
        <>
          <p className="bench-hint">Downloaded and verified. The package manager installs a .deb: run this on the computer BenchMesh runs on, then Restart above.</p>
          <CopyCommand command={u.command} />
        </>
      )}
      {u.available && !u.can_install && u.reason && <p className="bench-hint">{capital(u.reason)}.</p>}
      {u.phase === 'failed' && u.error && <span className="kit-error" role="alert">{u.error}</span>}
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
