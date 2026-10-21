/**
 * Step cards of the flow editor: one card per step (type picker, fields per type, move /
 * duplicate / delete, issues at its path), nested lists for `if` / `repeat` / `sweep`, and
 * the add-step menu after every step. Instrument fields are pickers fed by the vocabulary.
 */
import React, { createContext, useContext, useEffect, useId, useRef } from 'react'
import {
  changeStepType, childPath, classOf, defaultValue, deviceOf, duplicateStep, fields, insertStep, issuesAt, listAt,
  firstField, literalFromText, moveStep, nestedLists, newStep, patchStep, queries, removeStep, retarget, setIn, setters, specHint, STEP_TYPES,
} from './flowModel'
import type { FlowDoc, Issue, Step, StepType, ValueSpec, Vocab } from './types'

export interface EditorContext {
  doc: FlowDoc
  vocab: Vocab | null
  issues: Issue[]
  update: (f: (d: FlowDoc) => FlowDoc) => void
  variables: string[]
  /** Path clicked in the run log. */
  highlight: string | null
  /** Path of the step the run is executing. */
  running: string | null
}

export const EditorCtx = createContext<EditorContext | null>(null)
const useEditor = () => useContext(EditorCtx)!

// ---- small inputs ---------------------------------------------------------------------

export function IssueList({ issues }: { issues: Issue[] }) {
  if (!issues.length) return null
  return (
    <ul className="auto-issues" aria-label="Problems">
      {issues.map((i, n) => <li key={n} data-level={i.level}>{i.level === 'error' ? '✕' : '!'} {i.message}</li>)}
    </ul>
  )
}

function Field({ label, children, hint, wide }: { label: string; children: (id: string) => React.ReactNode; hint?: React.ReactNode; wide?: boolean }) {
  const id = useId()
  return (
    <div className={`kit-field auto-field${wide ? ' auto-field--wide' : ''}`}>
      <label className="kit-label" htmlFor={id}>{label}</label>
      {children(id)}
      {hint && <span className="bench-hint">{hint}</span>}
    </div>
  )
}

function Text({ label, value, onChange, wide, placeholder }: { label: string; value: string | undefined; onChange: (v: string) => void; wide?: boolean; placeholder?: string }) {
  return (
    <Field label={label} wide={wide}>
      {(id) => <input id={id} className="kit-input" type="text" value={value ?? ''} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />}
    </Field>
  )
}

/** Number field; empty text removes the value (`undefined`). */
function Num({ label, value, onChange, min, step = 'any' }: { label: string; value: number | undefined; onChange: (v: number | undefined) => void; min?: number; step?: string }) {
  return (
    <Field label={label}>
      {(id) => (
        <input id={id} className="kit-input" type="number" inputMode="decimal" step={step} min={min} value={value ?? ''}
          onChange={(e) => onChange(e.target.value === '' ? undefined : Number(e.target.value))} />
      )}
    </Field>
  )
}

/** A CEL expression with the variables in scope as a hint. */
function Expr({ label, value, onChange, wide = true }: { label: string; value: string | undefined; onChange: (v: string) => void; wide?: boolean }) {
  const { variables } = useEditor()
  return (
    <Field label={label} wide={wide} hint={<>CEL · variables: {variables.join(', ')} · num() abs() min() max() round()</>}>
      {(id) => <input id={id} className="kit-input auto-expr" type="text" spellCheck={false} value={value ?? ''} onChange={(e) => onChange(e.target.value)} />}
    </Field>
  )
}

function Select({ label, value, options, onChange, wide }: {
  label: string; value: string; options: { value: string; label: string }[]; onChange: (v: string) => void; wide?: boolean
}) {
  const known = options.some((o) => o.value === value)
  return (
    <Field label={label} wide={wide}>
      {(id) => (
        <select id={id} className="kit-input" value={value} onChange={(e) => onChange(e.target.value)} aria-invalid={!known}>
          {!known && <option value={value}>{value ? `${value} (unknown)` : '- choose -'}</option>}
          {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
      )}
    </Field>
  )
}

/** Two-way switch between choices (literal value / expression, field / query, …). */
function Mode<T extends string>({ label, value, options, onChange }: { label: string; value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="kit-field auto-field auto-field--wide">
      <span className="kit-label">{label}</span>
      <div className="kit-seg auto-mode" role="group" aria-label={label}>
        {options.map((o) => (
          <button key={o.value} type="button" aria-pressed={o.value === value} onClick={() => o.value !== value && onChange(o.value)}>{o.label}</button>
        ))}
      </div>
    </div>
  )
}

/** A literal for a setter's value spec: select of enum values / booleans, number input, else text. */
function LiteralInput({ label, value, spec, onChange }: { label: string; value: unknown; spec: ValueSpec | undefined; onChange: (v: unknown) => void }) {
  if (spec?.type === 'enum') {
    return <Select label={label} value={String(value ?? '')} options={(spec.values ?? []).map((v) => ({ value: v, label: v }))} onChange={onChange} />
  }
  if (spec?.type === 'bool') {
    return <Select label={label} value={String(value)} options={[{ value: 'true', label: 'true' }, { value: 'false', label: 'false' }]} onChange={(v) => onChange(v === 'true')} />
  }
  const numeric = spec?.type === 'number' || spec?.type === 'integer'
  return (
    <Field label={label} hint={specHint(spec) || undefined}>
      {(id) => (
        <input id={id} className="kit-input" type={numeric ? 'number' : 'text'} step="any" min={spec?.min} max={spec?.max}
          value={value === undefined || value === null ? '' : String(value)} onChange={(e) => onChange(literalFromText(e.target.value, spec))} />
      )}
    </Field>
  )
}

// ---- instrument address ---------------------------------------------------------------

/** device → class → channel pickers for an instrument step or a recording series. */
function AddressPicker({ value, onChange }: { value: Step; onChange: (at: { device?: string; class?: string; channel?: number }) => void }) {
  const { vocab } = useEditor()
  const d = deviceOf(vocab, value.device)
  const c = classOf(vocab, value.device, value.class)
  const channels = c?.channels ?? 1
  return (
    <>
      <Select label="Instrument" value={value.device ?? ''} onChange={(device) => onChange({ device })}
        options={(vocab?.devices ?? []).map((x) => ({ value: x.id, label: `${x.name}${x.name !== x.id ? ` (${x.id})` : ''}${x.parked ? ' · parked' : x.online ? '' : ' · offline'}` }))} />
      <Select label="Class" value={value.class ?? ''} onChange={(cls) => onChange({ class: cls })}
        options={Object.keys(d?.classes ?? {}).map((k) => ({ value: k, label: k }))} />
      {channels > 1 && (
        <Select label="Channel" value={String(value.channel ?? 1)} onChange={(ch) => onChange({ channel: Number(ch) })}
          options={Array.from({ length: channels }, (_, i) => ({ value: String(i + 1), label: `CH${i + 1}` }))} />
      )}
    </>
  )
}

// ---- fields per step type ---------------------------------------------------------------

function SetFields({ step, set }: { step: Step; set: (patch: Record<string, unknown>) => void }) {
  const { vocab } = useEditor()
  const c = classOf(vocab, step.device, step.class)
  const op = c?.operations[step.op]
  const isExpr = 'expr' in step
  return (
    <>
      <AddressPicker value={step} onChange={(at) => set(retarget(step, vocab, at))} />
      <Select label="Setting" value={step.op ?? ''}
        options={setters(c).map((n) => ({ value: n, label: `${n}${c!.operations[n].energizes ? ' ⚡' : ''}` }))}
        onChange={(n) => set(isExpr ? { op: n } : { op: n, value: defaultValue(c?.operations[n]?.value) })} />
      <div className="auto-field auto-field--wide auto-sethint">
        {op?.value && <span className="bench-hint">{op.description ? `${op.description} · ` : ''}{specHint(op.value)}</span>}
        {op?.energizes && <span className="auto-badge auto-badge--energize" title="Can switch an output or input on: the run must be armed">⚡ energizes</span>}
      </div>
      <Mode label="Value from" value={isExpr ? 'expr' : 'value'} options={[{ value: 'value', label: 'Value' }, { value: 'expr', label: 'Expression' }]}
        onChange={(m) => set(m === 'expr' ? { value: undefined, expr: step.value !== undefined ? String(step.value) : '' } : { expr: undefined, value: defaultValue(op?.value) })} />
      {isExpr
        ? <Expr label={`Value expression${op?.value?.unit ? ` (${op.value.unit})` : ''}`} value={step.expr} onChange={(expr) => set({ expr })} />
        : <LiteralInput label={`Value${op?.value?.unit ? ` (${op.value.unit})` : ''}`} value={step.value} spec={op?.value} onChange={(value) => set({ value })} />}
    </>
  )
}

function ReadFields({ step, set }: { step: Step; set: (patch: Record<string, unknown>) => void }) {
  const { vocab } = useEditor()
  const c = classOf(vocab, step.device, step.class)
  const byQuery = 'op' in step
  return (
    <>
      <AddressPicker value={step} onChange={(at) => set(retarget(step, vocab, at))} />
      <Mode label="Read" value={byQuery ? 'op' : 'field'} options={[{ value: 'field', label: 'Status field' }, { value: 'op', label: 'Query' }]}
        onChange={(m) => set(m === 'op' ? { field: undefined, op: queries(c)[0] ?? '' } : { op: undefined, field: firstField(c) })} />
      {byQuery
        ? <Select label="Query" value={step.op ?? ''} options={queries(c).map((n) => ({ value: n, label: n }))} onChange={(op) => set({ op })} />
        : <Select label="Field" value={step.field ?? ''} onChange={(field) => set({ field })}
            options={fields(c).map((n) => ({ value: n, label: `${n} · ${c!.fields[n].type}` }))} />}
      <Text label="Into variable" value={step.into} onChange={(into) => set({ into })} />
    </>
  )
}

function RecordFields({ step, set }: { step: Step; set: (patch: Record<string, unknown>) => void }) {
  const { vocab } = useEditor()
  const series: Step[] = Array.isArray(step.series) ? step.series : []
  const setSeries = (i: number, s: Step | null) => set({ series: s ? series.map((x, j) => (j === i ? s : x)) : series.filter((_, j) => j !== i) })
  return (
    <>
      <Mode label="Action" value={step.action === 'stop' ? 'stop' : 'start'} options={[{ value: 'start', label: 'Start' }, { value: 'stop', label: 'Stop' }]}
        onChange={(a) => set(a === 'stop' ? { action: 'stop', name: undefined, interval_s: undefined, series: undefined } : { ...newStep('record', vocab), do: 'record' })} />
      {step.action !== 'stop' && (
        <>
          <Text label="Recording name" value={step.name} onChange={(name) => set({ name })} />
          <Num label="Interval (s)" value={step.interval_s} min={1} step="1" onChange={(interval_s) => set({ interval_s })} />
          <div className="auto-field auto-field--wide auto-series">
            <span className="kit-label">Series</span>
            {series.map((s, i) => {
              const c = classOf(vocab, s.device, s.class)
              return (
                <div key={i} className="auto-series__row">
                  <AddressPicker value={s} onChange={(at) => setSeries(i, retarget({ ...s, do: 'read' }, vocab, at))} />
                  <Select label="Field" value={s.field ?? ''} options={fields(c).map((n) => ({ value: n, label: n }))} onChange={(field) => setSeries(i, { ...s, field })} />
                  <Text label="Label" value={s.label} onChange={(label) => setSeries(i, { ...s, label: label || undefined })} />
                  <button type="button" className="bench-icon" aria-label={`Remove series ${i + 1}`} onClick={() => setSeries(i, null)}>✕</button>
                </div>
              )
            })}
            <button type="button" className="bench-btn" onClick={() => set({ series: [...series, newStep('record', vocab).series[0]] })}>+ Series</button>
          </div>
        </>
      )}
    </>
  )
}

/** Strip the editor-only `do` from a series entry (retarget works on step-shaped objects). */
function stripDo(step: Step): Step {
  if (step.do !== 'record' || !Array.isArray(step.series)) return step
  return { ...step, series: step.series.map(({ do: _d, ...rest }: any) => rest) }
}

function StepFields({ step, set }: { step: Step; set: (patch: Record<string, unknown>) => void }) {
  switch (step.do) {
    case 'set': return <SetFields step={step} set={set} />
    case 'read': return <ReadFields step={step} set={set} />
    case 'wait': {
      const isExpr = 'expr' in step
      return (
        <>
          <Mode label="Duration" value={isExpr ? 'expr' : 'seconds'} options={[{ value: 'seconds', label: 'Seconds' }, { value: 'expr', label: 'Expression' }]}
            onChange={(m) => set(m === 'expr' ? { seconds: undefined, expr: String(step.seconds ?? 1) } : { expr: undefined, seconds: 1 })} />
          {isExpr ? <Expr label="Seconds expression" value={step.expr} onChange={(expr) => set({ expr })} />
            : <Num label="Seconds" value={step.seconds} min={0} onChange={(seconds) => set({ seconds })} />}
        </>
      )
    }
    case 'let': return <><Text label="Variable" value={step.var} onChange={(v) => set({ var: v })} /><Expr label="Expression" value={step.expr} onChange={(expr) => set({ expr })} /></>
    case 'if': return <Expr label="Condition" value={step.condition} onChange={(condition) => set({ condition })} />
    case 'repeat': {
      const bound = 'while' in step ? 'while' : 'until' in step ? 'until' : 'times'
      const reset = { times: undefined, while: undefined, until: undefined }
      return (
        <>
          <Mode label="Repeat" value={bound} options={[{ value: 'times', label: 'Times' }, { value: 'while', label: 'While' }, { value: 'until', label: 'Until' }]}
            onChange={(b) => set({ ...reset, [b]: b === 'times' ? 3 : 'false' })} />
          {bound === 'times' ? <Num label="Times" value={step.times} min={0} step="1" onChange={(times) => set({ times })} />
            : <Expr label={bound === 'while' ? 'While (tested before each pass)' : 'Until (tested after each pass)'} value={step[bound]} onChange={(v) => set({ [bound]: v })} />}
          <Num label="Max iterations" value={step.max_iterations} min={1} step="1" onChange={(max_iterations) => set({ max_iterations })} />
        </>
      )
    }
    case 'sweep': {
      const list = Array.isArray(step.values)
      return (
        <>
          <Text label="Variable" value={step.var} onChange={(v) => set({ var: v })} />
          <Mode label="Values" value={list ? 'list' : 'range'} options={[{ value: 'range', label: 'Range' }, { value: 'list', label: 'List' }]}
            onChange={(m) => set(m === 'list' ? { from: undefined, to: undefined, step: undefined, values: [0, 1] } : { values: undefined, from: 0, to: 1, step: 0.5 })} />
          {list ? (
            <Field label="Values (comma separated)" wide>
              {(id) => <ValuesInput id={id} values={step.values} onChange={(values) => set({ values })} />}
            </Field>
          ) : (
            <>
              <Num label="From" value={step.from} onChange={(from) => set({ from })} />
              <Num label="To" value={step.to} onChange={(to) => set({ to })} />
              <Num label="Step" value={step.step} onChange={(s) => set({ step: s })} />
            </>
          )}
        </>
      )
    }
    case 'check':
      return (
        <>
          <Expr label="Condition" value={step.condition} onChange={(condition) => set({ condition })} />
          <Text label="Message when it fails" value={step.message} onChange={(message) => set({ message: message || undefined })} wide />
          <Select label="On failure" value={step.fail ?? 'stop'} options={[{ value: 'stop', label: 'Stop the run' }, { value: 'warn', label: 'Warn and go on' }]} onChange={(fail) => set({ fail })} />
        </>
      )
    case 'log': return <Text label="Message ({variable} is replaced)" value={step.message} onChange={(message) => set({ message })} wide />
    case 'prompt': return <Text label="Message to the user" value={step.message} onChange={(message) => set({ message })} wide />
    case 'record': return <RecordFields step={step} set={(p) => set(p)} />
    case 'stop': return <Text label="Message" value={step.message} onChange={(message) => set({ message: message || undefined })} wide />
    default: return <span className="bench-hint">Unknown step type - edit it in the JSON view.</span>
  }
}

/** Comma-separated literal values; numbers where they parse. Keeps the text while typing. */
function ValuesInput({ id, values, onChange }: { id: string; values: unknown[]; onChange: (v: unknown[]) => void }) {
  const [text, setText] = React.useState(values.join(', '))
  const [seen, setSeen] = React.useState(values)
  if (seen !== values) {
    setSeen(values)
    if (parseValues(text).join(',') !== values.join(',')) setText(values.join(', '))
  }
  return <input id={id} className="kit-input" type="text" value={text} onChange={(e) => { setText(e.target.value); onChange(parseValues(e.target.value)) }} />
}

const parseValues = (t: string) => t.split(',').map((s) => s.trim()).filter(Boolean).map((s) => (Number.isFinite(Number(s)) ? Number(s) : s))

// ---- cards and lists ---------------------------------------------------------------------

function AddStep({ list, index, label }: { list: string; index: number; label: string }) {
  const { update, vocab } = useEditor()
  return (
    <select className="kit-input auto-add" aria-label={label} value=""
      onChange={(e) => { const t = e.target.value as StepType; if (t) update((d) => insertStep(d, list, index, newStep(t, vocab))) }}>
      <option value="">+ Add step…</option>
      {STEP_TYPES.map((t) => <option key={t.type} value={t.type}>{t.label} - {t.hint}</option>)}
    </select>
  )
}

export function StepList({ list, title, note }: { list: string; title?: string; note?: string }) {
  const { doc } = useEditor()
  const steps = listAt(doc, list)
  return (
    <div className="auto-list" data-list={list}>
      {title && <h4 className="auto-list__title">{title}{note && <span className="auto-list__note">{note}</span>}</h4>}
      {steps.length === 0 && <span className="bench-hint auto-list__empty">No steps.</span>}
      <AddStep list={list} index={0} label={`Add step at the start of ${list}`} />
      {steps.map((s, i) => (
        <React.Fragment key={i}>
          <StepCard path={childPath(list, i)} step={s} index={i} count={steps.length} />
          <AddStep list={list} index={i + 1} label={`Add step after ${childPath(list, i)}`} />
        </React.Fragment>
      ))}
    </div>
  )
}

function StepCard({ path, step, index, count }: { path: string; step: Step; index: number; count: number }) {
  const { update, vocab, issues, highlight, running } = useEditor()
  const ref = useRef<HTMLDivElement>(null)
  const mine = issuesAt(issues, path)
  const level = mine.some((i) => i.level === 'error') ? 'error' : mine.length ? 'warning' : undefined
  const set = (patch: Record<string, unknown>) => update((d) => patchStep(d, path, patch))
  const highlighted = highlight === path
  useEffect(() => {
    if (highlighted && typeof ref.current?.scrollIntoView === 'function') ref.current.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }, [highlighted])
  const typeId = useId()
  return (
    <div ref={ref} className="auto-step" data-path={path} data-level={level} data-highlight={highlighted || undefined}
      data-running={running === path || undefined} role="group" aria-label={`Step ${path}: ${step.do}`}>
      <div className="auto-step__head">
        <label className="auto-visually-hidden" htmlFor={typeId}>Step type</label>
        <select id={typeId} className="kit-input auto-step__type" value={step.do}
          onChange={(e) => update((d) => setIn(d, path, stripDo(changeStepType(step, e.target.value as StepType, vocab))))}>
          {STEP_TYPES.map((t) => <option key={t.type} value={t.type}>{t.label}</option>)}
        </select>
        <span className="auto-step__path">{path}</span>
        {running === path && <span className="auto-badge auto-badge--run">running</span>}
        <span className="auto-step__tools">
          <button type="button" className="bench-icon" aria-label={`Move ${path} up`} disabled={index === 0} onClick={() => update((d) => moveStep(d, path, -1))}>↑</button>
          <button type="button" className="bench-icon" aria-label={`Move ${path} down`} disabled={index === count - 1} onClick={() => update((d) => moveStep(d, path, 1))}>↓</button>
          <button type="button" className="bench-icon" aria-label={`Duplicate ${path}`} onClick={() => update((d) => duplicateStep(d, path))}>⧉</button>
          <button type="button" className="bench-icon" aria-label={`Delete ${path}`} onClick={() => update((d) => removeStep(d, path))}>✕</button>
        </span>
      </div>
      <div className="auto-step__fields">
        <StepFields step={step} set={(p) => set(step.do === 'record' ? stripDo({ ...step, ...p } as Step) : p)} />
      </div>
      <IssueList issues={mine} />
      {nestedLists(step).map((n) => (
        <div key={n.key} className="auto-step__nested">
          <StepList list={`${path}.${n.key}`} title={n.label} />
        </div>
      ))}
    </div>
  )
}
