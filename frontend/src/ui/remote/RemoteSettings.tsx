/**
 * Settings → Remote access (local principal only): the password, the enable switch, what the
 * network listener does, its addresses and certificate fingerprint, and who is connected.
 * An optional view-only password signs in browsers that see the bench and change nothing.
 * The existing passwords are never shown; the server keeps only their hashes.
 */
import React, { useId, useState } from 'react'
import { ApiError, errorText } from '../automation/client'
import { MicroSwitch } from '../kit'
import { MIN_PASSWORD, RemoteStatus } from './client'
import { useRemote } from './RemoteProvider'
import { SessionList } from './SessionList'
import './remote.css'

/** Why a new password cannot be saved yet, or null. */
export function passwordProblem(password: string, repeat: string): string | null {
  if (password.length < MIN_PASSWORD) return `At least ${MIN_PASSWORD} characters.`
  if (password !== repeat) return 'The passwords do not match.'
  return null
}

export function RemoteSettings() {
  const remote = useRemote()
  const s = remote?.status
  if (!remote || remote.principal !== 'local' || !s) return null
  return (
    <section className="bench-page" aria-label="Remote access settings" data-testid="remote-settings">
      <h3 className="settings-view__section-title">Remote access</h3>
      <p className="bench-hint">
        Control this bench from a browser on your local network. HTTPS with a password; only addresses of your local network are accepted.
      </p>
      <PasswordForm status={s} kind="control" />
      {s.password_set && <PasswordForm status={s} kind="view" />}
      <EnableSwitch status={s} />
      {s.enabled && <ListenerInfo status={s} />}
      <div className="bench-group">
        <span className="kit-label">Connected</span>
        <SessionList sessions={s.sessions} />
        {s.sessions.length > 0 && <DisconnectAll />}
      </div>
      <p className="bench-hint">
        Windows: the first time remote access opens its port, Windows asks whether to allow BenchMesh on private networks. Allow it, or other
        computers cannot connect.
      </p>
    </section>
  )
}

const FORMS = {
  control: {
    title: 'Password', form: 'Remote access password', isSet: (s: RemoteStatus) => s.password_set, unset: 'No password set yet.',
    signsOut: 'every remote browser', set: 'Set password', change: 'Change password',
    label: (isSet: boolean) => (isSet ? 'New password' : 'Password'), repeat: 'Repeat password',
  },
  view: {
    title: 'View-only password', form: 'View-only password', isSet: (s: RemoteStatus) => s.view_password_set,
    unset: 'Optional: browsers that sign in with it see the bench and change nothing. Not set.',
    signsOut: 'every view-only browser', set: 'Set view-only password', change: 'Change view-only password',
    label: (isSet: boolean) => (isSet ? 'New view-only password' : 'View-only password'), repeat: 'Repeat view-only password',
  },
}

function PasswordForm({ status, kind }: { status: RemoteStatus; kind: keyof typeof FORMS }) {
  const remote = useRemote()!
  const f = FORMS[kind]
  const isSet = f.isSet(status)
  const [password, setPassword] = useState('')
  const [repeat, setRepeat] = useState('')
  const [touched, setTouched] = useState(false)
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const [busy, setBusy] = useState(false)
  const ids = { password: useId(), repeat: useId(), problem: useId() }
  const problem = passwordProblem(password, repeat)

  const save = async (e: React.FormEvent) => {
    e.preventDefault()
    setTouched(true)
    if (problem) return
    setBusy(true)
    setMsg(null)
    try {
      await (kind === 'view' ? remote.client.setViewPassword(password) : remote.client.setPassword(password))
      setPassword('')
      setRepeat('')
      setTouched(false)
      const signedOut = kind === 'view' ? status.sessions.some((x) => x.role === 'view') : status.sessions.length > 0
      setMsg({ ok: true, text: signedOut ? `Password saved; ${f.signsOut} was signed out.` : 'Password saved.' })
      await remote.refresh()
    } catch (err) { setMsg({ ok: false, text: errorText(err) }) } finally { setBusy(false) }
  }

  const remove = async () => {
    setBusy(true)
    setMsg(null)
    try {
      await remote.client.removeViewPassword()
      setMsg({ ok: true, text: 'View-only password removed.' })
      await remote.refresh()
    } catch (err) { setMsg({ ok: false, text: errorText(err) }) } finally { setBusy(false) }
  }

  return (
    <form className="bench-group" onSubmit={save} aria-label={f.form}>
      <span className="kit-label">{f.title}</span>
      <span className="bench-hint" data-testid={kind === 'view' ? 'view-password-state' : 'password-state'}>{isSet ? 'Password set' : f.unset}</span>
      <div className="remote-settings__fields">
        <div className="kit-field">
          <label className="kit-label" htmlFor={ids.password}>{f.label(isSet)}</label>
          <input id={ids.password} className="kit-input" type="password" autoComplete="new-password" value={password}
            aria-invalid={touched && password.length < MIN_PASSWORD} aria-describedby={ids.problem}
            onChange={(e) => setPassword(e.target.value)} />
        </div>
        <div className="kit-field">
          <label className="kit-label" htmlFor={ids.repeat}>{f.repeat}</label>
          <input id={ids.repeat} className="kit-input" type="password" autoComplete="new-password" value={repeat}
            aria-invalid={touched && !!repeat && password !== repeat} aria-describedby={ids.problem}
            onChange={(e) => setRepeat(e.target.value)} />
        </div>
      </div>
      <span id={ids.problem} className={touched && problem ? 'kit-error' : 'bench-hint'} role={touched && problem ? 'alert' : undefined}>
        {touched && problem ? problem : `At least ${MIN_PASSWORD} characters${kind === 'view' ? ', not the remote access password' : ''}.${isSet ? ` Changing it signs ${f.signsOut} out.` : ''}`}
      </span>
      <div className="bench-group__row">
        <button type="submit" className="bench-btn bench-btn--primary" disabled={busy || !password} aria-busy={busy}>
          {isSet ? f.change : f.set}
        </button>
        {kind === 'view' && isSet && (
          <button type="button" className="bench-btn bench-btn--danger" onClick={() => void remove()} disabled={busy}>Remove</button>
        )}
        {msg && <span className={msg.ok ? 'bench-hint remote-ok' : 'kit-error'} role={msg.ok ? 'status' : 'alert'}>{msg.text}</span>}
      </div>
    </form>
  )
}

function EnableSwitch({ status }: { status: RemoteStatus }) {
  const remote = useRemote()!
  const [error, setError] = useState<string | null>(null)
  const toggle = async (enabled: boolean) => {
    setError(null)
    try {
      remote.setStatus(await remote.client.update({ enabled }))
    } catch (e) {
      setError(e instanceof ApiError && e.status === 409 ? 'Set a password first.' : errorText(e))
      throw e
    }
  }
  return (
    <div className="bench-group">
      <div className="bench-group__row">
        <MicroSwitch label="Remote access" checked={status.enabled} onChange={toggle} disabled={!status.password_set}
          title={status.password_set ? 'Open or close the network listener' : 'Set a password first'} />
        <span className="bench-hint" data-testid="remote-state">{stateText(status)}</span>
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}

export function stateText(s: RemoteStatus): string {
  if (!s.enabled) return s.password_set ? 'Off: nothing listens on the network.' : 'Off. Set a password to switch it on.'
  if (s.error) return `Not listening: ${s.error}`
  if (s.listening) return `Listening on port ${s.port}`
  return 'Starting…'
}

function ListenerInfo({ status }: { status: RemoteStatus }) {
  return (
    <div className="bench-group">
      {status.urls.length > 0 && (
        <>
          <span className="kit-label">Open in a browser on another computer</span>
          <ul className="remote-urls" aria-label="Remote access addresses">
            {status.urls.map((u) => (
              <li key={u}>
                <a href={u} target="_blank" rel="noreferrer"><code>{u}</code></a>
                <CopyButton text={u} label={`Copy ${u}`} />
              </li>
            ))}
          </ul>
        </>
      )}
      {status.fingerprint && (
        <>
          <span className="kit-label">Certificate (SHA-256)</span>
          <span className="remote-urls__fp">
            <code data-testid="remote-fingerprint">{status.fingerprint}</code>
            <CopyButton text={status.fingerprint} label="Copy the certificate fingerprint" />
          </span>
          <span className="bench-hint">
            Your browser warns once about the certificate made by this app; check that its SHA-256 fingerprint matches.
          </span>
        </>
      )}
    </div>
  )
}

function DisconnectAll() {
  const remote = useRemote()!
  const [error, setError] = useState<string | null>(null)
  const run = async () => {
    setError(null)
    try { await remote.client.disconnectAll(); await remote.refresh() } catch (e) { setError(errorText(e)) }
  }
  return (
    <div className="bench-group__row">
      <button type="button" className="bench-btn" onClick={() => void run()}>Disconnect all</button>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState<boolean | null>(null)
  const copy = async () => {
    try { await navigator.clipboard.writeText(text); setCopied(true) } catch { setCopied(false) }
    setTimeout(() => setCopied(null), 2000)
  }
  return (
    <button type="button" className="bench-btn remote-copy" onClick={() => void copy()} aria-label={label}>
      {copied ? 'Copied' : copied === false ? 'Copy failed' : 'Copy'}
    </button>
  )
}
