/**
 * Automation settings: the assistant (provider, the user's own API key - shown only as the
 * server's hint after saving - model, base URL) and MCP access for AI clients, with
 * ready-to-paste connection snippets.
 */
import React, { useEffect, useId, useState } from 'react'
import { AutomationClient, errorText } from './client'
import { providerName } from './AssistantPanel'
import type { AssistantSettings, McpAccess, McpInfo, ModelInfo, Provider } from './types'

const PROVIDERS: Provider[] = ['anthropic', 'openai', 'openrouter', 'custom']

export function SettingsTab({ client }: { client: AutomationClient }) {
  return (
    <div className="auto-settings">
      <AssistantSettingsForm client={client} />
      <McpSettings client={client} />
    </div>
  )
}

function AssistantSettingsForm({ client }: { client: AutomationClient }) {
  const [s, setS] = useState<AssistantSettings | null>(null)
  const [key, setKey] = useState('')
  const [model, setModel] = useState('')
  const [baseUrl, setBaseUrl] = useState('')
  const [models, setModels] = useState<ModelInfo[]>([])
  const [loadingModels, setLoadingModels] = useState(false)
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const ids = { provider: useId(), key: useId(), model: useId(), models: useId(), base: useId() }

  const take = (x: AssistantSettings) => { setS(x); setModel(x.model ?? ''); setBaseUrl(x.base_url ?? '') }
  useEffect(() => { client.assistant().then(take).catch((e) => setMsg({ ok: false, text: errorText(e) })) }, [client])

  const put = async (u: Parameters<AutomationClient['updateAssistant']>[0], done: string) => {
    setMsg(null)
    try { take(await client.updateAssistant(u)); setMsg({ ok: true, text: done }); return true } catch (e) { setMsg({ ok: false, text: errorText(e) }); return false }
  }
  const loadModels = async () => {
    setLoadingModels(true)
    setMsg(null)
    try { setModels(await client.models()) } catch (e) { setMsg({ ok: false, text: errorText(e) }) } finally { setLoadingModels(false) }
  }

  if (!s) return <section className="auto-section"><h3 className="auto-settings__title">Assistant</h3>{msg ? <span className="kit-error" role="alert">{msg.text}</span> : <span className="bench-hint">Loading…</span>}</section>
  const hint = s.keys?.[s.provider] ?? null
  return (
    <section className="auto-section" aria-label="Assistant settings">
      <h3 className="auto-settings__title">Assistant</h3>
      <p className="bench-hint">
        The assistant turns a description into a flow using your own account with an LLM provider. Your API key is stored on this
        computer in the BenchMesh data folder (<code>automation.json</code>) and sent only to the chosen provider; after saving it is
        never shown again (only a hint). Usage is billed by the provider to you.
      </p>
      <div className="auto-settings__grid">
        <div className="kit-field">
          <label className="kit-label" htmlFor={ids.provider}>Provider</label>
          <select id={ids.provider} className="kit-input" value={s.provider} onChange={(e) => { setModels([]); void put({ provider: e.target.value as Provider }, `Provider: ${providerName(e.target.value)}`) }}>
            {PROVIDERS.map((p) => <option key={p} value={p}>{providerName(p)}{s.keys?.[p] ? ' · key stored' : ''}</option>)}
          </select>
        </div>

        <form className="kit-field" onSubmit={async (e) => { e.preventDefault(); if (key.trim() && await put({ provider: s.provider, api_key: key.trim() }, 'API key saved')) setKey('') }}>
          <label className="kit-label" htmlFor={ids.key}>API key for {providerName(s.provider)}</label>
          <div className="auto-inline">
            <input id={ids.key} className="kit-input" type="password" autoComplete="off" spellCheck={false} value={key}
              placeholder={hint ? `stored: ${hint}` : s.provider === 'custom' ? 'optional for local servers' : 'paste your key'} onChange={(e) => setKey(e.target.value)} />
            <button type="submit" className="bench-btn bench-btn--primary" disabled={!key.trim()}>Save key</button>
          </div>
          <span className="bench-hint" data-testid="key-hint">
            {hint ? <>Stored key: <code>{hint}</code> <button type="button" className="auto-link" onClick={() => void put({ provider: s.provider, api_key: '' }, 'API key removed')}>Remove</button></> : 'No key stored.'}
          </span>
        </form>

        <form className="kit-field" onSubmit={(e) => { e.preventDefault(); void put({ provider: s.provider, model: model.trim() }, 'Model saved') }}>
          <label className="kit-label" htmlFor={ids.model}>Model</label>
          <div className="auto-inline">
            <input id={ids.model} className="kit-input" type="text" list={ids.models} value={model} spellCheck={false} onChange={(e) => setModel(e.target.value)} />
            <datalist id={ids.models}>{models.map((m) => <option key={m.id} value={m.id}>{m.name && m.name !== m.id ? m.name : undefined}</option>)}</datalist>
            <button type="button" className="bench-btn" onClick={loadModels} disabled={loadingModels} aria-busy={loadingModels}>Load models</button>
            <button type="submit" className="bench-btn">Save</button>
          </div>
          {models.length > 0 && (
            <select className="kit-input" aria-label="Models of the provider" value={models.some((m) => m.id === model) ? model : ''}
              onChange={(e) => e.target.value && setModel(e.target.value)}>
              <option value="">{models.length} models - choose…</option>
              {models.map((m) => <option key={m.id} value={m.id}>{m.name && m.name !== m.id ? `${m.name} (${m.id})` : m.id}</option>)}
            </select>
          )}
        </form>

        <form className="kit-field" onSubmit={(e) => { e.preventDefault(); void put({ base_url: baseUrl.trim() }, 'Base URL saved') }}>
          <label className="kit-label" htmlFor={ids.base}>Base URL{s.provider === 'custom' ? ' (required)' : ' (optional override)'}</label>
          <div className="auto-inline">
            <input id={ids.base} className="kit-input" type="url" value={baseUrl} spellCheck={false} placeholder="http://localhost:11434/v1"
              aria-invalid={s.provider === 'custom' && !s.base_url} onChange={(e) => setBaseUrl(e.target.value)} />
            <button type="submit" className="bench-btn">Save</button>
          </div>
          <span className="bench-hint">Any OpenAI-compatible endpoint, e.g. <code>http://localhost:11434/v1</code> for Ollama. Empty: the provider's own.</span>
        </form>
      </div>
      {msg && <span className={msg.ok ? 'bench-hint auto-ok' : 'kit-error'} role={msg.ok ? 'status' : 'alert'}>{msg.text}</span>}
    </section>
  )
}

const ACCESS: { value: McpAccess; label: string; text: string }[] = [
  { value: 'off', label: 'Off', text: 'AI clients cannot use BenchMesh.' },
  { value: 'read', label: 'Read', text: 'Read instruments, recordings, flows and the run log; validate flows; stop a run (always allowed). Nothing is changed.' },
  { value: 'control', label: 'Control', text: 'Also set instruments, save and run flows, start and stop recordings. Switching an output on still needs explicit confirmation by the client.' },
]

function McpSettings({ client }: { client: AutomationClient }) {
  const [info, setInfo] = useState<McpInfo | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { client.mcp().then(setInfo).catch((e) => setError(errorText(e))) }, [client])
  const set = async (access: McpAccess) => {
    setError(null)
    try { const r = await client.setMcp(access); setInfo((i) => ({ ...(i ?? {}), access: r.access })) } catch (e) { setError(errorText(e)) }
  }
  const endpoint = info?.endpoint ?? 'http://127.0.0.1:57666/mcp'
  const snippets = [
    { title: 'Claude Code', text: `claude mcp add --transport http benchmesh ${endpoint}` },
    ...(info?.command ? [{ title: 'Claude Desktop and other stdio clients', text: JSON.stringify({ mcpServers: { benchmesh: { command: info.command, args: info.args ?? [], ...(info.env ? { env: info.env } : {}) } } }, null, 2) }] : []),
    { title: 'Cursor and other HTTP clients', text: JSON.stringify({ mcpServers: { benchmesh: { url: endpoint } } }, null, 2) },
  ]
  return (
    <section className="auto-section" aria-label="MCP settings">
      <h3 className="auto-settings__title">MCP (AI clients)</h3>
      <p className="bench-hint">AI clients such as Claude Code, Claude Desktop or Cursor connect to the running app over MCP. The app must be running while they use it.</p>
      {info && (
        <fieldset className="auto-radio">
          <legend className="kit-label">Access</legend>
          {ACCESS.map((a) => (
            <label key={a.value} className="auto-radio__item">
              <input type="radio" name="mcp-access" value={a.value} checked={info.access === a.value} onChange={() => void set(a.value)} />
              <span><b>{a.label}</b>{a.value === 'read' ? ' (default)' : ''} - {a.text}</span>
            </label>
          ))}
        </fieldset>
      )}
      {error && <span className="kit-error" role="alert">{error}</span>}
      {info && snippets.map((s) => <Snippet key={s.title} title={s.title} text={s.text} />)}
      {info && !info.command && <span className="bench-hint">The stdio bridge command is not known for this server.</span>}
    </section>
  )
}

function Snippet({ title, text }: { title: string; text: string }) {
  const [copied, setCopied] = useState<boolean | null>(null)
  const copy = async () => {
    try { await navigator.clipboard.writeText(text); setCopied(true) } catch { setCopied(false) }
    setTimeout(() => setCopied(null), 2000)
  }
  return (
    <div className="auto-snippet">
      <div className="auto-snippet__head">
        <span className="kit-label">{title}</span>
        <button type="button" className="bench-btn" onClick={copy} aria-label={`Copy: ${title}`}>{copied ? 'Copied' : copied === false ? 'Copy failed' : 'Copy'}</button>
      </div>
      <pre className="auto-snippet__text">{text}</pre>
    </div>
  )
}
