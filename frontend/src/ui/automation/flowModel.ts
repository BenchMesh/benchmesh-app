/**
 * The editor's flow document model: pure, immutable helpers. Paths are the server's issue
 * paths (`steps[2].then[0]`, `params[1]`, `setup[0]`); a *list path* names a step list
 * (`steps`, `steps[2].then`). Nothing here talks to the server.
 */
import { FLOW_SCHEMA } from './types'
import type { FlowDoc, Issue, Param, Scalar, Step, StepType, ValueSpec, Vocab, VocabClass, VocabDevice } from './types'

export type PathPart = string | number

// ---- paths ---------------------------------------------------------------------------

/** `steps[2].then[0]` → `['steps', 2, 'then', 0]`. */
export function parsePath(path: string): PathPart[] {
  const out: PathPart[] = []
  for (const m of path.matchAll(/([^.[\]]+)|\[(\d+)\]/g)) out.push(m[2] !== undefined ? Number(m[2]) : m[1])
  return out
}

export function formatPath(parts: PathPart[]): string {
  return parts.map((p, i) => (typeof p === 'number' ? `[${p}]` : i === 0 ? p : `.${p}`)).join('')
}

/** `steps[2].then[0]` → `{list: 'steps[2].then', index: 0}`. */
export function splitStepPath(path: string): { list: string; index: number } {
  const parts = parsePath(path)
  const index = parts[parts.length - 1]
  if (typeof index !== 'number') throw new Error(`not a step path: ${path}`)
  return { list: formatPath(parts.slice(0, -1)), index }
}

export const childPath = (list: string, index: number) => `${list}[${index}]`

export function getIn(doc: unknown, path: string | PathPart[]): any {
  const parts = typeof path === 'string' ? parsePath(path) : path
  let cur: any = doc
  for (const p of parts) {
    if (cur == null) return undefined
    cur = cur[p as any]
  }
  return cur
}

/** A copy of `doc` with `value` at `path` (missing objects / lists are created). */
export function setIn<T>(doc: T, path: string | PathPart[], value: unknown): T {
  const parts = typeof path === 'string' ? parsePath(path) : path
  if (!parts.length) return value as T
  const [head, ...rest] = parts
  const base: any = Array.isArray(doc) ? [...doc] : { ...(doc as any ?? {}) }
  const next = base[head as any] ?? (typeof rest[0] === 'number' ? [] : {})
  base[head as any] = rest.length ? setIn(next, rest, value) : value
  return base
}

export function updateIn<T>(doc: T, path: string, f: (v: any) => any): T {
  return setIn(doc, path, f(getIn(doc, path)))
}

// ---- step lists ----------------------------------------------------------------------

export function listAt(doc: FlowDoc, list: string): Step[] {
  const v = getIn(doc, list)
  return Array.isArray(v) ? v : []
}

export function insertStep(doc: FlowDoc, list: string, index: number, step: Step): FlowDoc {
  const l = [...listAt(doc, list)]
  l.splice(Math.max(0, Math.min(index, l.length)), 0, step)
  return setIn(doc, list, l)
}

export function removeStep(doc: FlowDoc, path: string): FlowDoc {
  const { list, index } = splitStepPath(path)
  return setIn(doc, list, listAt(doc, list).filter((_, i) => i !== index))
}

/** Move a step up (`-1`) or down (`+1`) within its list; out of range is a no-op. */
export function moveStep(doc: FlowDoc, path: string, delta: number): FlowDoc {
  const { list, index } = splitStepPath(path)
  const l = [...listAt(doc, list)]
  const to = index + delta
  if (to < 0 || to >= l.length) return doc
  ;[l[index], l[to]] = [l[to], l[index]]
  return setIn(doc, list, l)
}

export function duplicateStep(doc: FlowDoc, path: string): FlowDoc {
  const { list, index } = splitStepPath(path)
  return insertStep(doc, list, index + 1, structuredClone(listAt(doc, list)[index]))
}

/** Replace a step's fields (`undefined` removes a field). */
export function patchStep(doc: FlowDoc, path: string, patch: Record<string, unknown>): FlowDoc {
  return updateIn(doc, path, (s: Step) => clean({ ...s, ...patch }))
}

function clean<T extends Record<string, unknown>>(o: T): T {
  const out: any = {}
  for (const [k, v] of Object.entries(o)) if (v !== undefined) out[k] = v
  return out
}

// ---- documents -----------------------------------------------------------------------

export function emptyFlow(name = 'New flow'): FlowDoc {
  return { schema: FLOW_SCHEMA, name, description: '', params: [], setup: [], steps: [], cleanup: [] }
}

/** A document loaded from a file or the assistant: a flow object, else an error. */
export function asFlow(v: unknown): FlowDoc {
  if (!v || typeof v !== 'object' || Array.isArray(v)) throw new Error('A flow must be a JSON object')
  const f = v as FlowDoc
  if (f.schema !== FLOW_SCHEMA) throw new Error(`Not a ${FLOW_SCHEMA} document`)
  return f
}

export const sameDoc = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

/** The parameter form's values, as run overrides. */
export function paramValues(doc: FlowDoc): Record<string, Scalar> {
  return Object.fromEntries((doc.params ?? []).map((p) => [p.name, p.value]))
}

export function newParam(doc: FlowDoc): Param {
  const names = new Set((doc.params ?? []).map((p) => p.name))
  let n = 1
  while (names.has(`p${n}`)) n++
  return { name: `p${n}`, value: 0, unit: '', description: '' }
}

/** Variable names a flow defines (params, `into`, `var`) plus the built-ins. */
export function variables(doc: FlowDoc): string[] {
  const out = new Set<string>()
  for (const p of doc.params ?? []) if (p?.name) out.add(p.name)
  const walk = (l: unknown) => {
    if (!Array.isArray(l)) return
    for (const s of l) {
      if (!s || typeof s !== 'object') continue
      if (typeof s.into === 'string' && s.into) out.add(s.into)
      if ((s.do === 'let' || s.do === 'sweep') && typeof s.var === 'string' && s.var) out.add(s.var)
      walk(s.then); walk(s.else); walk(s.steps)
    }
  }
  walk(doc.setup); walk(doc.steps); walk(doc.cleanup)
  return [...out, 'iteration', 'elapsed']
}

// ---- issues --------------------------------------------------------------------------

export const issuesAt = (issues: Issue[], path: string) => issues.filter((i) => i.path === path)
export const hasErrors = (issues: Issue[]) => issues.some((i) => i.level === 'error')

export function issueCounts(issues: Issue[]) {
  return { errors: issues.filter((i) => i.level === 'error').length, warnings: issues.filter((i) => i.level !== 'error').length }
}

/** Issues that no step card shows (document, params or list level), for the summary. */
export function isStepPath(path: string) {
  return /^(setup|steps|cleanup)\[\d+\]/.test(path)
}

// ---- steps ---------------------------------------------------------------------------

export const STEP_TYPES: { type: StepType; label: string; hint: string }[] = [
  { type: 'set', label: 'Set', hint: 'Set an instrument (waits for confirmation)' },
  { type: 'read', label: 'Read', hint: 'Read a status field or query into a variable' },
  { type: 'wait', label: 'Wait', hint: 'Pause for some seconds' },
  { type: 'let', label: 'Let', hint: 'Assign a variable' },
  { type: 'if', label: 'If', hint: 'Branch on a condition' },
  { type: 'repeat', label: 'Repeat', hint: 'Loop a number of times, while or until a condition' },
  { type: 'sweep', label: 'Sweep', hint: 'Run steps for each value of a range' },
  { type: 'check', label: 'Check', hint: 'Assert a condition' },
  { type: 'log', label: 'Log', hint: 'Write a run log line' },
  { type: 'prompt', label: 'Prompt', hint: 'Wait until the user continues' },
  { type: 'record', label: 'Record', hint: 'Start or stop a recording' },
  { type: 'stop', label: 'Stop', hint: 'End the main part (cleanup still runs)' },
]

export const INSTRUMENT_STEPS: StepType[] = ['set', 'read']

/** The nested step lists of a step: `then`/`else` of `if`, `steps` of `repeat`/`sweep`. */
export function nestedLists(step: Step): { key: 'then' | 'else' | 'steps'; label: string }[] {
  if (step.do === 'if') return [{ key: 'then', label: 'Then' }, { key: 'else', label: 'Else' }]
  if (step.do === 'repeat' || step.do === 'sweep') return [{ key: 'steps', label: 'Do' }]
  return []
}

// ---- vocabulary ----------------------------------------------------------------------

export const deviceOf = (v: Vocab | null, id: string | undefined): VocabDevice | undefined => v?.devices.find((d) => d.id === id)
export const classOf = (v: Vocab | null, device: string | undefined, cls: string | undefined): VocabClass | undefined =>
  cls ? deviceOf(v, device)?.classes[cls] : undefined

export const setters = (c: VocabClass | undefined) => Object.entries(c?.operations ?? {}).filter(([, o]) => o.set).map(([n]) => n)
export const queries = (c: VocabClass | undefined) => Object.entries(c?.operations ?? {}).filter(([, o]) => o.get).map(([n]) => n)
export const fields = (c: VocabClass | undefined) => Object.keys(c?.fields ?? {})
/** The field a new read / series starts with: the first measured value (`si` / number), else the first. */
export const firstField = (c: VocabClass | undefined) =>
  fields(c).find((f) => ['si', 'number', 'integer'].includes(c!.fields[f].type)) ?? fields(c)[0] ?? ''

/** `0 - 60 V`, `OFF | ON`, … for a setter's value spec. */
export function specHint(spec: ValueSpec | undefined): string {
  if (!spec) return ''
  if (spec.type === 'enum') return (spec.values ?? []).join(' | ')
  if (spec.type === 'bool') return 'true | false'
  const unit = spec.unit ? ` ${spec.unit}` : ''
  if (spec.min != null && spec.max != null) return `${spec.min} - ${spec.max}${unit}`
  if (spec.min != null) return `≥ ${spec.min}${unit}`
  if (spec.max != null) return `≤ ${spec.max}${unit}`
  return unit.trim()
}

/** A literal that fits the spec: the first enum value that is not ON, `false`, else the min (or 0). */
export function defaultValue(spec: ValueSpec | undefined): Scalar {
  if (!spec) return 0
  if (spec.type === 'enum') return spec.values?.find((x) => x.toUpperCase() !== 'ON') ?? spec.values?.[0] ?? ''
  if (spec.type === 'bool') return false
  if (spec.type === 'string') return ''
  return spec.min != null && spec.min > 0 ? spec.min : 0
}

/** The address a new instrument step starts with: the first device (online first) with a class that has what the step needs. */
function firstAddress(v: Vocab | null, want: 'set' | 'read' | 'field'): { device: string; class: string; channel: number } {
  const devices = [...(v?.devices ?? [])].sort((a, b) => Number(b.online) - Number(a.online))
  for (const d of devices) {
    for (const [cls, c] of Object.entries(d.classes)) {
      const ok = want === 'set' ? setters(c).length > 0 : want === 'read' ? queries(c).length > 0 : fields(c).length > 0
      if (ok) return { device: d.id, class: cls, channel: 1 }
    }
  }
  return { device: '', class: '', channel: 1 }
}

/** First non-energising setter of a class (else any). */
export function firstSetter(c: VocabClass | undefined): string {
  const s = setters(c)
  return s.find((n) => !c?.operations[n]?.energizes) ?? s[0] ?? ''
}

export function newStep(type: StepType, vocab: Vocab | null = null): Step {
  switch (type) {
    case 'set': {
      const at = firstAddress(vocab, 'set')
      const c = classOf(vocab, at.device, at.class)
      const op = firstSetter(c)
      return { do: 'set', ...at, op, value: defaultValue(c?.operations[op]?.value) }
    }
    case 'read': {
      const at = firstAddress(vocab, 'field')
      return { do: 'read', ...at, field: firstField(classOf(vocab, at.device, at.class)), into: 'reading' }
    }
    case 'wait': return { do: 'wait', seconds: 1 }
    case 'let': return { do: 'let', var: 'x', expr: '0' }
    case 'if': return { do: 'if', condition: 'true', then: [], else: [] }
    case 'repeat': return { do: 'repeat', times: 3, steps: [] }
    case 'sweep': return { do: 'sweep', var: 'x', from: 0, to: 1, step: 0.5, steps: [] }
    case 'check': return { do: 'check', condition: 'true', message: '', fail: 'stop' }
    case 'log': return { do: 'log', message: '' }
    case 'prompt': return { do: 'prompt', message: 'Continue when ready' }
    case 'record': {
      const at = firstAddress(vocab, 'field')
      const f = firstField(classOf(vocab, at.device, at.class))
      return { do: 'record', action: 'start', name: 'Flow recording', interval_s: 1, series: [{ ...at, field: f }] }
    }
    case 'stop': return { do: 'stop', message: '' }
  }
}

/** Change a step's type: a fresh template, keeping the instrument address and nested steps where both types have them. */
export function changeStepType(step: Step, type: StepType, vocab: Vocab | null = null): Step {
  if (step.do === type) return step
  let next = newStep(type, vocab)
  if (INSTRUMENT_STEPS.includes(step.do) && INSTRUMENT_STEPS.includes(type) && step.device) {
    next = { ...next, device: step.device, class: step.class, channel: step.channel ?? 1 }
    const c = classOf(vocab, step.device, step.class)
    if (type === 'set') { const op = firstSetter(c); next.op = op; next.value = defaultValue(c?.operations[op]?.value) }
    if (type === 'read') { delete next.op; next.field = firstField(c) }
  }
  const inner: Step[] = [...(step.then ?? []), ...(step.steps ?? [])]
  if (inner.length) {
    if (type === 'if') next.then = inner
    if (type === 'repeat' || type === 'sweep') next.steps = inner
  }
  return next
}

/** Point an instrument step (or a series) at another device / class / channel, keeping what still fits. */
export function retarget(step: Step, vocab: Vocab | null, at: { device?: string; class?: string; channel?: number }): Step {
  const device = at.device ?? step.device
  const d = deviceOf(vocab, device)
  let cls = at.class ?? step.class
  if (d && !d.classes[cls]) cls = Object.keys(d.classes)[0] ?? ''
  const c = d?.classes[cls]
  const channel = Math.min(Math.max(1, at.channel ?? step.channel ?? 1), c?.channels ?? 1)
  const next: Step = { ...step, device, class: cls, channel }
  if (step.do === 'set' && c && !c.operations[step.op]?.set) {
    next.op = firstSetter(c)
    if (!('expr' in step)) next.value = defaultValue(c.operations[next.op]?.value)
  }
  if ('field' in step && c && !(step.field in c.fields)) next.field = firstField(c)
  if (step.do === 'read' && 'op' in step && c && !c.operations[step.op]?.get) next.op = queries(c)[0] ?? ''
  return next
}

/** Parse what a user typed for a literal of this spec. */
export function literalFromText(text: string, spec: ValueSpec | undefined): Scalar {
  if (spec?.type === 'bool') return text === 'true'
  if (spec?.type === 'enum' || spec?.type === 'string') return text
  const t = text.trim()
  if (t !== '' && Number.isFinite(Number(t))) return Number(t)
  if (!spec && (t === 'true' || t === 'false')) return t === 'true'
  return text
}
