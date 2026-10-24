/**
 * Automation drawer: saved flows (new, duplicate, delete, import / export JSON), the flow
 * editor with live validation, the run panel, the assistant, and the settings tab
 * (assistant provider / key / model, MCP access). Contract: `ai_context/AUTOMATION.md`.
 */
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { saveFile, savedText } from '../files'
import { ConfirmButton } from '../recording/charts'
import '../recording/recording.css'
import { AssistantPanel } from './AssistantPanel'
import { ApiError, errorText } from './client'
import { EditorView, FlowEditor } from './FlowEditor'
import { asFlow, emptyFlow, hasErrors, issueCounts, isStepPath, sameDoc } from './flowModel'
import { useRun } from './RunProvider'
import { RunPanel } from './RunPanel'
import { SettingsTab } from './SettingsTab'
import { IssueList } from './StepCard'
import type { FlowDoc, FlowSummary, Issue, Validation, Vocab } from './types'
import './automation.css'

interface Current { id: string | null; doc: FlowDoc; saved: FlowDoc | null }

/** The editor's document survives closing the drawer (Escape, scrim) until the page reloads. */
let draft: Current | null = null
export const resetDraft = () => { draft = null }

const fresh = (): Current => ({ id: null, doc: emptyFlow(), saved: emptyFlow() })
const hasSteps = (d: FlowDoc) => [d.setup, d.steps, d.cleanup].some((l) => Array.isArray(l) && l.length > 0)

export function AutomationDrawer({ onOpenRecordings }: { onOpenRecordings?: () => void }) {
  const run = useRun()
  const [tab, setTab] = useState<'flows' | 'settings'>('flows')
  if (!run) return null
  return (
    <div className="auto">
      <div className="kit-seg auto-tabs" role="tablist" aria-label="Automation">
        <button type="button" role="tab" aria-selected={tab === 'flows'} aria-pressed={tab === 'flows'} onClick={() => setTab('flows')}>Flows</button>
        <button type="button" role="tab" aria-selected={tab === 'settings'} aria-pressed={tab === 'settings'} onClick={() => setTab('settings')}>Settings</button>
      </div>
      {tab === 'flows' ? <FlowsTab onOpenSettings={() => setTab('settings')} onOpenRecordings={onOpenRecordings} /> : <SettingsTab client={run.client} />}
    </div>
  )
}

function FlowsTab({ onOpenSettings, onOpenRecordings }: { onOpenSettings: () => void; onOpenRecordings?: () => void }) {
  const run = useRun()!
  const { client } = run
  const [flows, setFlows] = useState<FlowSummary[]>([])
  const [listError, setListError] = useState<string | null>(null)
  const [vocab, setVocab] = useState<Vocab | null>(null)
  const [cur, setCurState] = useState<Current>(() => draft ?? fresh())
  const [validation, setValidation] = useState<Validation | null>(null)
  const [saveIssues, setSaveIssues] = useState<Issue[] | null>(null)
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null)
  const [view, setView] = useState<EditorView>('form')
  const [highlight, setHighlight] = useState<string | null>(null)
  const [pending, setPending] = useState<{ label: string; go: () => void } | null>(null)
  const [saving, setSaving] = useState(false)
  const fileRef = useRef<HTMLInputElement>(null)

  const setCur = useCallback((c: Current) => { draft = c; setCurState(c) }, [])
  const dirty = cur.saved === null || !sameDoc(cur.doc, cur.saved)

  const loadList = useCallback(() => client.listFlows().then((l) => { setFlows(l); setListError(null) }).catch((e) => setListError(errorText(e))), [client])
  useEffect(() => { void loadList() }, [loadList])
  useEffect(() => { client.vocabulary().then(setVocab).catch(() => {}) }, [client])

  // live validation, debounced; stale answers are dropped
  const seq = useRef(0)
  useEffect(() => {
    const n = ++seq.current
    const t = setTimeout(() => {
      client.validate(cur.doc).then((v) => { if (n === seq.current) setValidation(v) })
        .catch((e) => { if (n === seq.current) setValidation({ ok: false, energizes: [], issues: [{ path: '', level: 'error', message: errorText(e) }] }) })
    }, 400)
    return () => clearTimeout(t)
  }, [client, cur.doc])

  const issues = saveIssues ?? validation?.issues ?? []
  const counts = issueCounts(issues)
  const errors = hasErrors(issues)
  const checking = !validation && !saveIssues

  const edit = useCallback((doc: FlowDoc) => {
    setSaveIssues(null)
    setMsg(null)
    setCur({ ...(draft ?? cur), doc })
  }, [cur, setCur])

  /** Leave the current document, asking first when it has unsaved changes. */
  const leave = (label: string, go: () => void) => {
    if (dirty && hasSteps(cur.doc)) setPending({ label, go })
    else go()
  }
  const loadDoc = (c: Current) => { setCur(c); setSaveIssues(null); setValidation(null); setMsg(null); setHighlight(null); setPending(null) }

  const open = (id: string) => leave(`open another flow`, () => {
    client.getFlow(id).then((doc) => loadDoc({ id, doc, saved: doc })).catch((e) => setMsg({ ok: false, text: errorText(e) }))
  })

  const save = async () => {
    setSaving(true)
    setMsg(null)
    try {
      const r = cur.id ? await client.updateFlow(cur.id, cur.doc) : await client.createFlow(cur.doc)
      setCur({ id: r.id, doc: cur.doc, saved: cur.doc })
      setSaveIssues(null)
      setMsg({ ok: true, text: r.issues?.length ? `Saved with ${r.issues.length} warning(s)` : 'Saved' })
      void loadList()
    } catch (e) {
      setMsg({ ok: false, text: errorText(e) })
      if (e instanceof ApiError && e.issues.length) setSaveIssues(e.issues)
    } finally { setSaving(false) }
  }

  const importFile = async (f: File) => {
    try {
      const doc = asFlow(JSON.parse(await f.text()))
      leave('import a flow', () => loadDoc({ id: null, doc, saved: null }))
    } catch (e) { setMsg({ ok: false, text: `Import: ${errorText(e)}` }) }
  }
  const exportFile = async () => {
    try {
      const name = `${(cur.doc.name || 'flow').replace(/[^\w.-]+/g, '_')}.json`
      const text = savedText(await saveFile(name, { text: JSON.stringify(cur.doc, null, 2), type: 'application/json' }))
      if (text) setMsg({ ok: true, text })
    } catch (e) { setMsg({ ok: false, text: `Export: ${errorText(e)}` }) }
  }

  const runFlow = run.status.flow
  const sameFlow = runFlow && (cur.id ? runFlow.id === cur.id : runFlow.id == null && runFlow.name === cur.doc.name)
  const running = run.active && sameFlow && run.status.path ? run.status.path : null
  const blocked = checking ? 'Checking the flow…' : errors ? `Fix ${counts.errors} error${counts.errors === 1 ? '' : 's'} first` : null
  const summaryIssues = issues.filter((i) => !isStepPath(i.path) && !/^(name|timeout_s|params(\[\d+\])?|setup|steps|cleanup)$/.test(i.path))

  return (
    <div className="auto-flows">
      <aside className="auto-flowlist" aria-label="Saved flows">
        <div className="auto-flowlist__actions">
          <button type="button" className="bench-btn" onClick={() => leave('start a new flow', () => loadDoc(fresh()))}>New</button>
          <button type="button" className="bench-btn" onClick={() => fileRef.current?.click()}>Import</button>
          <input ref={fileRef} type="file" accept="application/json,.json" hidden aria-label="Import flow file"
            onChange={(e) => { const f = e.target.files?.[0]; e.target.value = ''; if (f) void importFile(f) }} />
        </div>
        {listError && <span className="kit-error" role="alert">{listError}</span>}
        {flows.length === 0 && !listError && <span className="bench-hint">No saved flows yet.</span>}
        <ul className="auto-flowlist__items">
          {flows.map((f) => (
            <li key={f.id}>
              <button type="button" className="auto-flowlist__item" aria-current={f.id === cur.id || undefined} onClick={() => f.id !== cur.id && open(f.id)}
                title={f.description || undefined}>
                <span className="auto-flowlist__name">{f.name}</span>
                {runFlow?.id === f.id && run.active && <span className="auto-badge auto-badge--run">running</span>}
              </button>
            </li>
          ))}
        </ul>
      </aside>

      <div className="auto-main">
        <div className="auto-toolbar">
          <h3 className="auto-toolbar__name">{cur.doc.name || 'Untitled'}</h3>
          {dirty && <span className="auto-badge" data-testid="unsaved">unsaved</span>}
          <span className="auto-summary" data-testid="issue-summary" data-level={errors ? 'error' : counts.warnings ? 'warning' : 'ok'}>
            {checking ? 'checking…' : counts.errors || counts.warnings
              ? `${counts.errors} error${counts.errors === 1 ? '' : 's'}, ${counts.warnings} warning${counts.warnings === 1 ? '' : 's'}`
              : 'no problems'}
          </span>
          <span className="auto-toolbar__actions">
            <button type="button" className="bench-btn bench-btn--primary" onClick={save} disabled={saving || errors || !dirty} aria-busy={saving}
              title={errors ? 'Fix the errors first' : undefined}>Save</button>
            <button type="button" className="bench-btn" onClick={() => loadDoc({ id: null, doc: { ...structuredClone(cur.doc), name: `${cur.doc.name} (copy)`.slice(0, 100) }, saved: null })}>Duplicate</button>
            <button type="button" className="bench-btn" onClick={() => void exportFile()}>Export</button>
            {cur.id && (
              <ConfirmButton label="Delete" confirmLabel="Delete flow" question={`Delete “${cur.doc.name}”?`} danger
                onConfirm={async () => { await client.deleteFlow(cur.id!); loadDoc(fresh()); void loadList() }} />
            )}
          </span>
        </div>
        {pending && (
          <div className="auto-pending" role="alert">
            <span>Discard the unsaved changes to “{cur.doc.name}” and {pending.label}?</span>
            <button type="button" className="bench-btn rec-btn--danger" onClick={() => { const go = pending.go; setPending(null); go() }}>Discard</button>
            <button type="button" className="bench-btn" onClick={() => setPending(null)}>Cancel</button>
          </div>
        )}
        {msg && <span className={msg.ok ? 'bench-hint auto-ok' : 'kit-error'} role={msg.ok ? 'status' : 'alert'}>{msg.text}</span>}
        <IssueList issues={summaryIssues} />

        <div className="auto-work">
          <div className="auto-work__editor">
            <FlowEditor doc={cur.doc} onChange={edit} vocab={vocab} issues={issues} highlight={highlight} running={running} view={view} onView={setView} />
          </div>
          <div className="auto-work__side">
            <RunPanel doc={cur.doc} flowId={cur.id} dirty={dirty} blocked={blocked} onOpenRecordings={onOpenRecordings}
              onHighlight={(p) => { setView('form'); setHighlight(p) }} />
            <AssistantPanel client={client} flow={hasSteps(cur.doc) ? cur.doc : null} onOpenSettings={onOpenSettings}
              onFlow={(doc) => { setCur({ ...(draft ?? cur), doc }); setSaveIssues(null); setValidation(null) }} />
          </div>
        </div>
      </div>
    </div>
  )
}
