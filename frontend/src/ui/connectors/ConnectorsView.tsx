/**
 * Instruments → Connectors (this computer only): virtual SCPI instruments for VISA programs
 * (access, base port, each instrument's address with code to use it) and the Python package.
 */
import React, { useEffect, useId, useState } from 'react'
import { errorText } from '../automation/client'
import { Snippet } from '../automation/SettingsTab'
import { PageLink } from '../about/links'
import {
  basePortProblem, portState, PYTHON_SNIPPET, scpiClient, scpiSnippets,
  type ScpiAccess, type ScpiClient, type ScpiStatus,
} from './client'
import '../automation/automation.css'
import './connectors.css'

const ACCESS: { value: ScpiAccess; label: string; text: string }[] = [
  { value: 'off', label: 'Off', text: 'No ports are open.' },
  { value: 'read', label: 'Read', text: 'Programs can query instruments and read their latest values. Nothing is changed.' },
  { value: 'control', label: 'Control', text: 'Programs can also set instruments, including switching outputs on.' },
]

export function ConnectorsView({ client = scpiClient }: { client?: ScpiClient }) {
  const [status, setStatus] = useState<ScpiStatus | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    client.status().then(setStatus).catch((e) => setError(errorText(e)))
  }, [client])
  const update = async (u: { access?: ScpiAccess; base_port?: number }) => {
    setError(null)
    try { setStatus(await client.update(u)) } catch (e) { setError(errorText(e)); throw e }
  }
  return (
    <section className="bench-page" aria-label="Connectors" data-testid="connectors">
      <h3 className="settings-view__section-title">Virtual SCPI instruments</h3>
      <p className="bench-hint">
        Every instrument answers SCPI on its own port on this computer, whatever its own connection (serial, USB, LAN). MATLAB, LabVIEW,
        PyVISA, TestStand and other VISA programs open it as a LAN instrument. Commands go through BenchMesh, so the bench keeps showing
        what happens.
      </p>
      {status && (
        <div className="bench-group">
          <fieldset className="auto-radio">
            <legend className="kit-label">Access</legend>
            {ACCESS.map((a) => (
              <label key={a.value} className="auto-radio__item">
                <input type="radio" name="scpi-access" value={a.value} checked={status.access === a.value}
                  onChange={() => void update({ access: a.value }).catch(() => {})} />
                <span><b>{a.label}</b>{a.value === 'off' ? ' (default)' : ''} - {a.text}</span>
              </label>
            ))}
          </fieldset>
          <BasePort key={status.base_port} status={status} onSave={(p) => update({ base_port: p })} />
        </div>
      )}
      {error && <span className="kit-error" role="alert">{error}</span>}
      {status && status.access !== 'off' && <Ports status={status} />}
      <p className="bench-hint">
        Commands: <code>*IDN?</code>, an operation as a query or with a value (<code>VOLTAGE?</code>, <code>VOLTAGE 5</code>,{' '}
        <code>PSU:CH2:OUTPUT ON</code>), the latest reading with <code>FETCH:VOUT?</code>, the full list with{' '}
        <code>SYST:HELP:HEAD?</code> and errors with <code>SYST:ERR?</code>. Only programs on this computer can connect.{' '}
        <PageLink page="docs-scpi">SCPI reference</PageLink>
      </p>

      <h3 className="settings-view__section-title">Python</h3>
      <p className="bench-hint">
        The <code>benchmesh</code> package reads and sets instruments, follows live values, and loads recordings into pandas, in scripts,
        Jupyter or pytest. <PageLink page="docs-python">Python guide</PageLink>
      </p>
      <Snippet title="Install and read a meter" text={PYTHON_SNIPPET} />
    </section>
  )
}

function BasePort({ status, onSave }: { status: ScpiStatus; onSave: (port: number) => Promise<void> }) {
  const [text, setText] = useState(String(status.base_port))
  const id = useId()
  const problem = basePortProblem(text)
  const changed = Number(text) !== status.base_port
  return (
    <form className="bench-group__row" aria-label="Port numbers"
      onSubmit={(e) => { e.preventDefault(); if (!problem) void onSave(Number(text)).catch(() => {}) }}>
      <div className="kit-field">
        <label className="kit-label" htmlFor={id}>First port</label>
        <input id={id} className="kit-input connectors__port" inputMode="numeric" value={text} aria-invalid={!!problem}
          onChange={(e) => setText(e.target.value)} />
      </div>
      <button type="submit" className="bench-btn" disabled={!!problem || !changed}>Save</button>
      <span className={problem ? 'kit-error' : 'bench-hint'}>{problem ?? 'The other instruments take the next free ports. 5025 is the usual SCPI port.'}</span>
    </form>
  )
}

function Ports({ status }: { status: ScpiStatus }) {
  const [selected, setSelected] = useState<string | null>(null)
  const current = status.instruments.find((i) => i.id === selected) ?? status.instruments[0]
  if (!status.instruments.length) return <p className="bench-hint">No instruments are configured yet.</p>
  return (
    <div className="bench-group">
      <span className="kit-label">Addresses (VISA resource names)</span>
      <ul className="connectors__list" aria-label="Virtual SCPI addresses">
        {status.instruments.map((i) => (
          <li key={i.id} className={i.id === current?.id ? 'connectors__item connectors__item--on' : 'connectors__item'}>
            <button type="button" className="connectors__pick" onClick={() => setSelected(i.id)} aria-pressed={i.id === current?.id}>
              <b>{i.name}</b>
              <code>{i.address ?? 'no port'}</code>
            </button>
            <span className={i.error ? 'kit-error' : 'bench-hint'} data-testid={`scpi-state-${i.id}`}>{portState(status, i)}</span>
          </li>
        ))}
      </ul>
      {current && scpiSnippets(current, status.host).map((s) => <Snippet key={s.title} title={`${s.title} - ${current.name}`} text={s.text} />)}
    </div>
  )
}
