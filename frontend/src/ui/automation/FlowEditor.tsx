/**
 * Editor of one `benchmesh.flow/v1` document: name, description, the parameter form (the
 * settings a user adjusts without touching the steps), Setup / Steps / Cleanup step lists,
 * and a JSON view of the same document (kept in sync; parse errors shown).
 */
import React, { useId, useMemo, useState } from 'react'
import { issuesAt, newParam, sameDoc, variables } from './flowModel'
import { EditorCtx, EditorContext, IssueList, StepList } from './StepCard'
import type { FlowDoc, Issue, Param, Vocab } from './types'

export type EditorView = 'form' | 'json'

interface Props {
  doc: FlowDoc
  onChange: (d: FlowDoc) => void
  vocab: Vocab | null
  issues: Issue[]
  highlight: string | null
  running: string | null
  view: EditorView
  onView: (v: EditorView) => void
}

export function FlowEditor({ doc, onChange, vocab, issues, highlight, running, view, onView }: Props) {
  const ctx = useMemo<EditorContext>(() => ({
    doc, vocab, issues, highlight, running, variables: variables(doc),
    update: (f) => onChange(f(doc)),
  }), [doc, vocab, issues, highlight, running, onChange])
  return (
    <EditorCtx.Provider value={ctx}>
      <div className="auto-editor">
        <div className="kit-seg auto-view" role="group" aria-label="Editor view">
          <button type="button" aria-pressed={view === 'form'} onClick={() => onView('form')}>Flow</button>
          <button type="button" aria-pressed={view === 'json'} onClick={() => onView('json')}>JSON</button>
        </div>
        {view === 'json' ? <JsonView doc={doc} onChange={onChange} /> : <FormView doc={doc} onChange={onChange} issues={issues} />}
      </div>
    </EditorCtx.Provider>
  )
}

function FormView({ doc, onChange, issues }: { doc: FlowDoc; onChange: (d: FlowDoc) => void; issues: Issue[] }) {
  const nameId = useId()
  const descId = useId()
  const timeoutId = useId()
  return (
    <>
      <section className="auto-section auto-meta" aria-label="Flow">
        <div className="kit-field">
          <label className="kit-label" htmlFor={nameId}>Flow name</label>
          <input id={nameId} className="kit-input" type="text" value={doc.name ?? ''} maxLength={100}
            aria-invalid={issuesAt(issues, 'name').length > 0} onChange={(e) => onChange({ ...doc, name: e.target.value })} />
          <IssueList issues={issuesAt(issues, 'name')} />
        </div>
        <div className="kit-field">
          <label className="kit-label" htmlFor={descId}>Description</label>
          <textarea id={descId} className="kit-input" rows={2} value={doc.description ?? ''} onChange={(e) => onChange({ ...doc, description: e.target.value })} />
        </div>
        <div className="kit-field auto-meta__timeout">
          <label className="kit-label" htmlFor={timeoutId}>Time limit (s, optional)</label>
          <input id={timeoutId} className="kit-input" type="number" min={0} step="any" value={doc.timeout_s ?? ''}
            onChange={(e) => {
              const { timeout_s: _t, ...rest } = doc
              onChange(e.target.value === '' ? rest as FlowDoc : { ...doc, timeout_s: Number(e.target.value) })
            }} />
          <IssueList issues={issuesAt(issues, 'timeout_s')} />
        </div>
      </section>

      <ParamsForm doc={doc} onChange={onChange} issues={issues} />

      <section className="auto-section" aria-label="Setup">
        <StepList list="setup" title="Setup" note="runs first" />
        <IssueList issues={issuesAt(issues, 'setup')} />
      </section>
      <section className="auto-section" aria-label="Steps">
        <StepList list="steps" title="Steps" />
        <IssueList issues={issuesAt(issues, 'steps')} />
      </section>
      <section className="auto-section auto-section--cleanup" aria-label="Cleanup">
        <StepList list="cleanup" title="Cleanup" note="always runs - after success, failure, stop or abort" />
        <IssueList issues={issuesAt(issues, 'cleanup')} />
      </section>
    </>
  )
}

type ParamKind = 'number' | 'text' | 'bool'
const kindOf = (v: unknown): ParamKind => (typeof v === 'boolean' ? 'bool' : typeof v === 'string' ? 'text' : 'number')

/** The parameter form: the settings of the flow (expression variables, run overrides). */
function ParamsForm({ doc, onChange, issues }: { doc: FlowDoc; onChange: (d: FlowDoc) => void; issues: Issue[] }) {
  const params = doc.params ?? []
  const set = (i: number, p: Param | null) =>
    onChange({ ...doc, params: p ? params.map((x, j) => (j === i ? p : x)) : params.filter((_, j) => j !== i) })
  const opt = (v: string) => (v === '' ? undefined : Number(v))
  return (
    <section className="auto-section auto-params" aria-label="Parameters">
      <div className="auto-section__head">
        <h4 className="auto-list__title">Parameters<span className="auto-list__note">the settings of this flow - use them by name in expressions</span></h4>
        <button type="button" className="bench-btn" onClick={() => onChange({ ...doc, params: [...params, newParam(doc)] })}>+ Parameter</button>
      </div>
      {params.length === 0 && <span className="bench-hint">No parameters. Add one for every number a user may want to adjust (target voltage, step, limit…).</span>}
      {params.map((p, i) => {
        const n = i + 1
        const kind = kindOf(p.value)
        const mine = issuesAt(issues, `params[${i}]`)
        return (
          <div key={i} className="auto-param" data-path={`params[${i}]`} data-level={mine.length ? mine[0].level : undefined}>
            <label className="kit-field"><span className="kit-label">Name</span>
              <input className="kit-input" aria-label={`Parameter ${n} name`} value={p.name} onChange={(e) => set(i, { ...p, name: e.target.value })} /></label>
            <label className="kit-field"><span className="kit-label">Type</span>
              <select className="kit-input" aria-label={`Parameter ${n} type`} value={kind}
                onChange={(e) => set(i, { ...p, value: e.target.value === 'bool' ? false : e.target.value === 'text' ? '' : 0 })}>
                <option value="number">number</option><option value="text">text</option><option value="bool">true/false</option>
              </select></label>
            <label className="kit-field auto-param__value"><span className="kit-label">Value</span>
              {kind === 'bool' ? (
                <select className="kit-input" aria-label={`Parameter ${n} value`} value={String(p.value)} onChange={(e) => set(i, { ...p, value: e.target.value === 'true' })}>
                  <option value="true">true</option><option value="false">false</option>
                </select>
              ) : (
                <input className="kit-input" aria-label={`Parameter ${n} value`} type={kind === 'number' ? 'number' : 'text'} step="any" value={String(p.value)}
                  min={p.min ?? undefined} max={p.max ?? undefined}
                  onChange={(e) => set(i, { ...p, value: kind === 'number' ? (e.target.value === '' ? 0 : Number(e.target.value)) : e.target.value })} />
              )}</label>
            <label className="kit-field"><span className="kit-label">Unit</span>
              <input className="kit-input" aria-label={`Parameter ${n} unit`} value={p.unit ?? ''} onChange={(e) => set(i, { ...p, unit: e.target.value || undefined })} /></label>
            {kind === 'number' && <>
              <label className="kit-field"><span className="kit-label">Min</span>
                <input className="kit-input" aria-label={`Parameter ${n} min`} type="number" step="any" value={p.min ?? ''} onChange={(e) => set(i, { ...p, min: opt(e.target.value) })} /></label>
              <label className="kit-field"><span className="kit-label">Max</span>
                <input className="kit-input" aria-label={`Parameter ${n} max`} type="number" step="any" value={p.max ?? ''} onChange={(e) => set(i, { ...p, max: opt(e.target.value) })} /></label>
            </>}
            <label className="kit-field auto-param__desc"><span className="kit-label">Description</span>
              <input className="kit-input" aria-label={`Parameter ${n} description`} value={p.description ?? ''} onChange={(e) => set(i, { ...p, description: e.target.value || undefined })} /></label>
            <button type="button" className="bench-icon auto-param__remove" aria-label={`Remove parameter ${p.name || n}`} onClick={() => set(i, null)}>✕</button>
            <div className="auto-param__issues"><IssueList issues={mine} /></div>
          </div>
        )
      })}
      <IssueList issues={issuesAt(issues, 'params')} />
    </section>
  )
}

/** Raw JSON of the document; valid edits update the document, invalid text shows the parse error. */
function JsonView({ doc, onChange }: { doc: FlowDoc; onChange: (d: FlowDoc) => void }) {
  const [text, setText] = useState(() => JSON.stringify(doc, null, 2))
  const [error, setError] = useState<string | null>(null)
  // follow outside changes (assistant, step edits) unless the text already says the same
  const [seen, setSeen] = useState(doc)
  if (seen !== doc) {
    setSeen(doc)
    let same: boolean
    try { same = sameDoc(JSON.parse(text), doc) } catch { same = !!error /* keep what is being typed */ }
    if (!same) { setText(JSON.stringify(doc, null, 2)); setError(null) }
  }
  const id = useId()
  return (
    <div className="kit-field auto-json">
      <label className="kit-label" htmlFor={id}>Flow document (JSON)</label>
      <textarea id={id} className="kit-input auto-json__text" spellCheck={false} value={text} aria-invalid={!!error} rows={24}
        onChange={(e) => {
          const t = e.target.value
          setText(t)
          try {
            const v = JSON.parse(t)
            if (!v || typeof v !== 'object' || Array.isArray(v)) throw new Error('the document must be a JSON object')
            setError(null)
            onChange(v)
          } catch (err) { setError(err instanceof Error ? err.message : String(err)) }
        }} />
      {error && <span className="kit-error" role="alert">JSON: {error}</span>}
    </div>
  )
}

