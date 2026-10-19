/**
 * Assistant: the user describes an automation and the LLM of their choice (their own key,
 * called by the server) assembles the flow, or changes the current one. The result is
 * loaded into the editor as unsaved changes — never run automatically.
 */
import React, { useEffect, useId, useState } from 'react'
import { AutomationClient, errorText } from './client'
import { IssueList } from './StepCard'
import type { AssistantSettings, AssistResult, FlowDoc } from './types'

const PROVIDER_NAMES: Record<string, string> = { anthropic: 'Anthropic', openai: 'OpenAI', openrouter: 'OpenRouter', custom: 'Custom endpoint' }
export const providerName = (p: string) => PROVIDER_NAMES[p] ?? p

interface Props {
  client: AutomationClient
  /** The flow to change; null: assemble a new one. */
  flow: FlowDoc | null
  onFlow: (f: FlowDoc) => void
  onOpenSettings: () => void
}

export function AssistantPanel({ client, flow, onFlow, onOpenSettings }: Props) {
  const [settings, setSettings] = useState<AssistantSettings | null>(null)
  const [instruction, setInstruction] = useState('')
  const [busy, setBusy] = useState(false)
  const [elapsed, setElapsed] = useState(0)
  const [result, setResult] = useState<AssistResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const id = useId()

  useEffect(() => { client.assistant().then(setSettings).catch(() => {}) }, [client])
  useEffect(() => {
    if (!busy) return
    const t0 = Date.now()
    const t = setInterval(() => setElapsed(Math.round((Date.now() - t0) / 1000)), 1000)
    return () => clearInterval(t)
  }, [busy])

  const submit = async () => {
    setBusy(true)
    setElapsed(0)
    setError(null)
    setResult(null)
    try {
      const r = await client.assist(instruction.trim(), flow ?? undefined)
      setResult(r)
      onFlow(r.flow)
    } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }

  const provider = settings ? providerName(settings.provider) : null
  const hasKey = settings ? !!settings.keys?.[settings.provider] || settings.provider === 'custom' : true
  const settingsHint = error && /api key|model|base url|base_url|provider/i.test(error)
  return (
    <section className="auto-assist" aria-label="Assistant">
      <h4 className="auto-list__title">Assistant</h4>
      <div className="kit-field">
        <label className="kit-label" htmlFor={id}>{flow ? 'What should change?' : 'Describe the automation…'}</label>
        <textarea id={id} className="kit-input" rows={3} value={instruction} maxLength={8000} disabled={busy}
          placeholder={flow ? 'e.g. also record VOUT and IOUT every second' : 'e.g. ramp the PSU from 0 to 5 V in 0.5 V steps, read the DMM at each step, switch off at the end'}
          onChange={(e) => setInstruction(e.target.value)} />
      </div>
      <div className="auto-assist__actions">
        <button type="button" className="bench-btn bench-btn--primary" disabled={busy || !instruction.trim()} aria-busy={busy} onClick={submit}>
          {flow ? 'Change flow' : 'Assemble'}
        </button>
        {busy && <span className="auto-spinner" role="status">Working… {elapsed} s (can take up to 2 min)</span>}
      </div>
      {settings && (
        <p className="bench-hint">
          Uses <b>{provider}</b>{settings.model ? <> · <b>{settings.model}</b></> : null}{!hasKey && <> · <span className="kit-error">no API key yet</span></>}
          {' '}(<button type="button" className="auto-link" onClick={onOpenSettings}>settings</button>).
          {' '}Requests go to {provider} on your own account and at your expense, and include the bench description (instrument names, operations) and the current flow.
        </p>
      )}
      {error && (
        <div className="kit-error auto-assist__error" role="alert">
          {error}
          {settingsHint && <> — <button type="button" className="auto-link" onClick={onOpenSettings}>open assistant settings</button></>}
        </div>
      )}
      {result && (
        <div className="auto-assist__result" aria-label="Assistant result">
          <span className="kit-label">Loaded into the editor (not saved)</span>
          <p className="auto-assist__explanation">{result.explanation}</p>
          {result.attempts != null && result.attempts > 1 && <span className="bench-hint">{result.attempts} attempts</span>}
          <IssueList issues={result.issues ?? []} />
        </div>
      )}
    </section>
  )
}
