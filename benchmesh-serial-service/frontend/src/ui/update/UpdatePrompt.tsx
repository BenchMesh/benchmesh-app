import React, { useEffect, useState } from 'react'
import { Available, Progress, checkForUpdate, installUpdate, openReleasesPage } from './update'

/** Offers a newer version once per app start (desktop app); nothing is shown otherwise. */
export function UpdatePrompt() {
  const [update, setUpdate] = useState<Available | null>(null)
  const [installing, setInstalling] = useState(false)
  const [progress, setProgress] = useState<Progress | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    checkForUpdate().then((u) => { if (!cancelled) setUpdate(u) })
    return () => { cancelled = true }
  }, [])

  if (!update) return null

  const install = () => {
    setInstalling(true)
    setError(null)
    installUpdate(setProgress).catch((e) => {
      setInstalling(false)
      setProgress(null)
      setError(e instanceof Error ? e.message : String(e))
    })
  }
  const pct = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null

  return (
    <div className="bench-modal" role="dialog" aria-modal="true" aria-labelledby="update-title">
      <div className="bench-modal__scrim" />
      <div className="bench-modal__box">
        <h2 id="update-title">Update available</h2>
        <p>BenchMesh <b>{update.version}</b> is available — you have {update.current}.</p>
        {update.notes && <pre className="bench-modal__notes">{update.notes}</pre>}
        {installing && (
          <div className="bench-modal__progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
            <i style={{ width: pct == null ? '100%' : `${pct}%` }} data-indeterminate={pct == null} />
            <span>{pct == null ? 'Downloading…' : `Downloading… ${pct}%`} The app restarts when it is installed.</span>
          </div>
        )}
        {error && <p className="kit-error" role="alert">{error}</p>}
        <div className="bench-modal__actions">
          <button type="button" className="bench-btn" onClick={() => setUpdate(null)} disabled={installing}>Later</button>
          {update.canInstall ? (
            <button type="button" className="bench-btn bench-btn--primary" onClick={install} disabled={installing}>Update and restart</button>
          ) : (
            <button type="button" className="bench-btn bench-btn--primary" onClick={() => { openReleasesPage(); setUpdate(null) }}>Download</button>
          )}
        </div>
      </div>
    </div>
  )
}
