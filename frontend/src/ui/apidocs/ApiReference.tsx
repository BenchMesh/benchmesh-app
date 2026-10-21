import React, { useEffect, useId, useMemo, useRef, useState } from 'react'
import type { Instrument } from '../types'
import { PageLink } from '../about/links'
import { buildUrl, curlFor, filterGroups, formatBody, parseSpec, wsUrl, type ApiDoc, type ApiOp } from './openapi'
import './apidocs.css'

interface Props {
  apiBase: string
  /** Configured instruments: suggestions for `{id}`, `{klass}` and `{ch}`. */
  instruments?: Instrument[]
}

type Suggestions = Record<string, string[]>

const originOf = (apiBase: string) => apiBase || window.location.origin

/**
 * The API reference drawer: every route of this server from `/openapi.json`, grouped by tag,
 * filterable; an operation opens to its parameters and sends the request (a second press for
 * anything that is not a GET) with status, time and the response, plus a curl line.
 * WebSockets can be peeked at (first message); operations the server marks `x-no-try` say why.
 */
export function ApiReference({ apiBase, instruments = [] }: Props) {
  const [doc, setDoc] = useState<ApiDoc | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState<Set<string>>(() => new Set())

  useEffect(() => {
    let live = true
    fetch(`${apiBase}/openapi.json`)
      .then(async (r) => {
        if (!r.ok) throw new Error(`The server answered ${r.status} for /openapi.json`)
        return parseSpec(await r.json())
      })
      .then((d) => { if (live) setDoc(d) })
      .catch((e) => { if (live) setError(e instanceof Error ? e.message : String(e)) })
    return () => { live = false }
  }, [apiBase, attempt])

  const suggestions = useMemo<Suggestions>(() => {
    const classes = new Set<string>()
    const channels = new Set<string>()
    for (const i of instruments) for (const c of i.classes ?? []) {
      classes.add(c.class)
      c.channels?.forEach((path) => { const ch = path.split('/').pop(); if (ch) channels.add(ch) })
    }
    const ids = instruments.map((i) => i.id)
    return { id: ids, klass: [...classes].sort(), ch: [...channels].sort() }
  }, [instruments])

  const groups = useMemo(() => (doc ? filterGroups(doc.groups, query) : []), [doc, query])
  const count = groups.reduce((n, g) => n + g.ops.length, 0)
  const toggle = (id: string) => setOpen((s) => {
    const n = new Set(s)
    if (n.has(id)) n.delete(id)
    else n.add(id)
    return n
  })

  return (
    <div className="bench-page api-ref">
      <p className="bench-hint">
        Every endpoint of this BenchMesh server, ready to call. Guides for flows, the assistant and MCP are on{' '}
        <PageLink page="docs">benchmesh.online/docs</PageLink>. The document itself: <code>/openapi.json</code>, <code>/openapi.yaml</code>.
      </p>
      {error && (
        <div className="bench-group" role="alert">
          <span className="kit-error">{error}</span>
          <div><button type="button" className="bench-btn" onClick={() => { setError(null); setAttempt((a) => a + 1) }}>Retry</button></div>
        </div>
      )}
      {!doc && !error && <span className="bench-hint">Loading the API description…</span>}
      {doc && (
        <>
          <div className="api-ref__bar">
            <span className="api-ref__title">{doc.title} <span className="api-ref__version">v{doc.version}</span></span>
            <input className="kit-input api-ref__filter" type="search" placeholder="Filter: path, method or words" aria-label="Filter endpoints"
              value={query} onChange={(e) => setQuery(e.target.value)} />
            <span className="bench-hint" aria-live="polite">{count} endpoint{count === 1 ? '' : 's'}</span>
          </div>
          {groups.length === 0 && <span className="bench-hint">No endpoint matches “{query}”.</span>}
          {groups.map((g) => (
            <section key={g.tag} className="api-ref__group" aria-label={g.tag}>
              <h3 className="api-ref__tag">{g.tag}{g.description && <span className="bench-hint">{g.description}</span>}</h3>
              <ul className="api-ref__ops">
                {g.ops.map((op) => (
                  <li key={op.id} className="api-op" data-open={open.has(op.id) || undefined}>
                    <button type="button" className="api-op__row" aria-expanded={open.has(op.id)} onClick={() => toggle(op.id)}>
                      <span className="api-method" data-method={op.method}>{op.websocket ? 'WS' : op.method.toUpperCase()}</span>
                      <span className="api-op__path">{op.path}</span>
                      <span className="api-op__summary">{op.summary}</span>
                    </button>
                    {open.has(op.id) && <Operation op={op} apiBase={apiBase} suggestions={suggestions} />}
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </>
      )}
    </div>
  )
}

interface Sent { status: number; statusText: string; ms: number; contentType: string; body: string }

function Operation({ op, apiBase, suggestions }: { op: ApiOp; apiBase: string; suggestions: Suggestions }) {
  const uid = useId()
  const [values, setValues] = useState<Record<string, string>>({})
  const [query, setQuery] = useState('')
  const [body, setBody] = useState('{}')
  const [armed, setArmed] = useState(false)
  const [sending, setSending] = useState(false)
  const [sent, setSent] = useState<Sent | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const disarm = useRef<ReturnType<typeof setTimeout>>()
  useEffect(() => () => clearTimeout(disarm.current), [])

  const origin = originOf(apiBase)
  const { url, missing } = buildUrl(op.path, values, query)
  let bodyError: string | null = null
  if (op.hasBody) { try { JSON.parse(body) } catch (e) { bodyError = `Not valid JSON: ${e instanceof Error ? e.message : e}` } }
  const ready = !!url && !bodyError && !sending
  const writes = op.method !== 'get'

  const send = async () => {
    if (!url) return
    if (writes && !armed) {
      // anything that changes the server (or an instrument) takes a second press
      setArmed(true)
      clearTimeout(disarm.current)
      disarm.current = setTimeout(() => setArmed(false), 4000)
      return
    }
    setArmed(false)
    clearTimeout(disarm.current)
    setSending(true)
    setFailure(null)
    const t0 = performance.now()
    try {
      const r = await fetch(apiBase + url, {
        method: op.method.toUpperCase(),
        headers: op.hasBody ? { 'Content-Type': 'application/json' } : undefined,
        body: op.hasBody ? body : undefined,
      })
      const contentType = r.headers.get('content-type') ?? ''
      const text = await r.text()
      setSent({ status: r.status, statusText: r.statusText, ms: Math.round(performance.now() - t0), contentType, body: formatBody(text, contentType) })
    } catch (e) {
      setSent(null)
      setFailure(`No answer from the server: ${e instanceof Error ? e.message : e}`)
    } finally {
      setSending(false)
    }
  }

  return (
    <div className="api-op__detail">
      {op.summary && <p className="api-op__text">{op.summary}</p>}
      {op.websocket ? (
        <WebSocketPeek address={wsUrl(origin, op.path)} />
      ) : op.noTry ? (
        <p className="bench-hint api-op__note">Not sent from here: {op.noTry}.</p>
      ) : (
        <>
          <div className="api-op__fields">
              {op.params.map((p) => (
                <div className="kit-field" key={p}>
                  <label className="kit-label" htmlFor={`${uid}-${p}`}>{p}</label>
                  <input id={`${uid}-${p}`} className="kit-input" value={values[p] ?? ''} aria-invalid={missing.includes(p) && p in values}
                    list={suggestions[p]?.length ? `${uid}-${p}-list` : undefined} autoComplete="off" spellCheck={false}
                    onChange={(e) => setValues((v) => ({ ...v, [p]: e.target.value }))} />
                  {suggestions[p]?.length ? <datalist id={`${uid}-${p}-list`}>{suggestions[p].map((s) => <option key={s} value={s} />)}</datalist> : null}
                </div>
              ))}
              <div className="kit-field api-op__query">
                <label className="kit-label" htmlFor={`${uid}-query`}>Query (optional)</label>
                <input id={`${uid}-query`} className="kit-input" placeholder="a=1&b=2" value={query} spellCheck={false}
                  onChange={(e) => setQuery(e.target.value)} />
              </div>
          </div>
          {op.hasBody && (
            <div className="kit-field">
              <label className="kit-label" htmlFor={`${uid}-body`}>Body (JSON)</label>
              <textarea id={`${uid}-body`} className="kit-input api-op__body" rows={5} spellCheck={false} value={body}
                aria-invalid={!!bodyError} onChange={(e) => setBody(e.target.value)} />
              {bodyError && <span className="kit-error">{bodyError}</span>}
            </div>
          )}
          <div className="api-op__actions">
            <button type="button" className={`bench-btn ${armed ? 'api-op__send--armed' : 'bench-btn--primary'}`} disabled={!ready}
              aria-busy={sending} onClick={() => void send()}>
              {sending ? 'Sending…' : armed ? `Press again: ${op.method.toUpperCase()}` : 'Send'}
            </button>
            {missing.length > 0 && <span className="bench-hint">Fill in {missing.join(', ')}.</span>}
            {writes && !armed && missing.length === 0 && <span className="bench-hint">Changes the server: Send asks twice.</span>}
          </div>
          {url && <CopyLine label="curl" text={curlFor(origin, op.method, url, op.hasBody ? body : undefined)} />}
          {failure && <span className="kit-error" role="alert">{failure}</span>}
          {sent && (
            <div className="api-op__response" aria-label="Response">
              <div className="api-op__status">
                <span className="api-status" data-ok={sent.status < 400}>{sent.status} {sent.statusText}</span>
                <span className="bench-hint">{sent.ms} ms</span>
                {sent.contentType && <span className="bench-hint">{sent.contentType}</span>}
              </div>
              {sent.body ? <pre className="api-op__pre">{sent.body}</pre> : <span className="bench-hint">(no body)</span>}
            </div>
          )}
        </>
      )}
    </div>
  )
}

function CopyLine({ label, text }: { label: string; text: string }) {
  const [copied, setCopied] = useState(false)
  const copy = () => {
    void navigator.clipboard?.writeText(text).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    }, () => {})
  }
  return (
    <div className="api-op__copy">
      <span className="kit-label">{label}</span>
      <code className="api-op__code">{text}</code>
      <button type="button" className="bench-btn bench-btn--icon" onClick={copy} aria-label={`Copy ${label}`}>{copied ? 'Copied' : 'Copy'}</button>
    </div>
  )
}

/** Connect to a WebSocket stream, show its first message, close. */
function WebSocketPeek({ address }: { address: string }) {
  const [state, setState] = useState<'idle' | 'waiting' | 'done'>('idle')
  const [text, setText] = useState('')
  const [failure, setFailure] = useState<string | null>(null)
  const sock = useRef<WebSocket | null>(null)
  useEffect(() => () => sock.current?.close(), [])

  const peek = () => {
    sock.current?.close()
    setState('waiting')
    setFailure(null)
    setText('')
    const ws = new WebSocket(address)
    sock.current = ws
    const timer = setTimeout(() => { ws.close(); setState('done'); setFailure('No message within 35 s (some streams send every 30 s).') }, 35_000)
    ws.onmessage = (e) => {
      clearTimeout(timer)
      setText(formatBody(typeof e.data === 'string' ? e.data : '[binary message]', 'application/json'))
      setState('done')
      ws.close()
    }
    ws.onerror = () => { clearTimeout(timer); setState('done'); setFailure('The WebSocket could not be opened.') }
  }

  return (
    <>
      <CopyLine label="address" text={address} />
      <div className="api-op__actions">
        <button type="button" className="bench-btn bench-btn--primary" onClick={peek} disabled={state === 'waiting'} aria-busy={state === 'waiting'}>
          {state === 'waiting' ? 'Waiting for a message…' : 'Show the first message'}
        </button>
        <span className="bench-hint">A stream: connect with any WebSocket client. The UI uses it live.</span>
      </div>
      {failure && <span className="kit-error" role="alert">{failure}</span>}
      {text && <pre className="api-op__pre" aria-label="First message">{text}</pre>}
    </>
  )
}
