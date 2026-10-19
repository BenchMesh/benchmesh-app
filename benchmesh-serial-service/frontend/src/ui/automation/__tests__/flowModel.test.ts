import { describe, expect, it } from 'vitest'
import {
  asFlow, changeStepType, firstField, childPath, defaultValue, duplicateStep, emptyFlow, formatPath, getIn, insertStep, issuesAt, isStepPath,
  literalFromText, moveStep, nestedLists, newStep, paramValues, parsePath, patchStep, removeStep, retarget, sameDoc, setIn, specHint,
  splitStepPath, variables,
} from '../flowModel'
import { flowDoc, vocab } from './fakes'

describe('paths', () => {
  it('parses and formats issue paths', () => {
    expect(parsePath('steps[2].then[0]')).toEqual(['steps', 2, 'then', 0])
    expect(parsePath('params[1]')).toEqual(['params', 1])
    expect(formatPath(['steps', 2, 'else', 10])).toBe('steps[2].else[10]')
    expect(splitStepPath('setup[3].steps[1]')).toEqual({ list: 'setup[3].steps', index: 1 })
    expect(childPath('steps[0].then', 2)).toBe('steps[0].then[2]')
    expect(() => splitStepPath('steps')).toThrow()
  })

  it('reads and writes immutably by path', () => {
    const d = flowDoc()
    const e = setIn(d, 'steps[1].into', 'w')
    expect(getIn(e, 'steps[1].into')).toBe('w')
    expect(getIn(d, 'steps[1].into')).toBe('v')
    expect(e.steps![0]).toBe(d.steps![0]) // untouched branches are shared
    expect(getIn(d, 'steps[9].into')).toBeUndefined()
    expect(getIn(setIn(emptyFlow(), 'steps[0].then', []), 'steps[0].then')).toEqual([])
  })
})

describe('step lists', () => {
  it('inserts, removes, moves and duplicates steps, also nested', () => {
    let d = insertStep(flowDoc(), 'steps', 1, newStep('if'))
    expect(d.steps!.map((s) => s.do)).toEqual(['set', 'if', 'read'])
    d = insertStep(d, 'steps[1].then', 0, newStep('log'))
    d = insertStep(d, 'steps[1].then', 1, newStep('wait'))
    expect(getIn(d, 'steps[1].then').map((s: any) => s.do)).toEqual(['log', 'wait'])
    d = moveStep(d, 'steps[1].then[1]', -1)
    expect(getIn(d, 'steps[1].then').map((s: any) => s.do)).toEqual(['wait', 'log'])
    expect(moveStep(d, 'steps[0]', -1)).toBe(d) // out of range: no-op
    d = duplicateStep(d, 'steps[1].then[0]')
    expect(getIn(d, 'steps[1].then').map((s: any) => s.do)).toEqual(['wait', 'wait', 'log'])
    expect(getIn(d, 'steps[1].then[0]')).not.toBe(getIn(d, 'steps[1].then[1]'))
    d = removeStep(d, 'steps[0]')
    expect(d.steps!.map((s) => s.do)).toEqual(['if', 'read'])
  })

  it('patches fields, undefined removes them', () => {
    const d = patchStep(flowDoc(), 'steps[0]', { expr: undefined, value: 3 })
    expect(d.steps![0]).toEqual({ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'voltage', value: 3 })
  })
})

describe('step templates', () => {
  it('fills instrument steps from the vocabulary (online device, non-energising setter)', () => {
    expect(newStep('set', vocab)).toEqual({ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'voltage', value: 0 })
    expect(newStep('read', vocab)).toMatchObject({ do: 'read', device: 'psu', field: 'VOUT', into: 'reading' })
    expect(newStep('record', vocab).series).toEqual([{ device: 'psu', class: 'PSU', channel: 1, field: 'VOUT' }])
    expect(firstField({ channels: 1, operations: {}, fields: { CC: { type: 'bool' }, MODE: { type: 'enum' }, VOUT: { type: 'si' } } })).toBe('VOUT')
    expect(firstField({ channels: 1, operations: {}, fields: { CC: { type: 'bool' } } })).toBe('CC')
    expect(nestedLists(newStep('if')).map((n) => n.key)).toEqual(['then', 'else'])
    expect(nestedLists(newStep('sweep')).map((n) => n.key)).toEqual(['steps'])
    expect(nestedLists(newStep('log'))).toEqual([])
  })

  it('keeps the address and the nested steps when the type changes', () => {
    const read = changeStepType({ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'output', value: 'ON' }, 'read', vocab)
    expect(read).toMatchObject({ do: 'read', device: 'psu', class: 'PSU', field: 'VOUT' })
    expect(read.op).toBeUndefined()
    const rep = changeStepType({ do: 'if', condition: 'x', then: [{ do: 'log', message: 'a' }], else: [] }, 'repeat')
    expect(rep).toMatchObject({ do: 'repeat', times: 3, steps: [{ do: 'log', message: 'a' }] })
  })

  it('retargets to another device: class, channel and op are kept only where they exist', () => {
    const s = retarget({ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'voltage', value: 5 }, vocab, { device: 'awg' })
    expect(s).toMatchObject({ device: 'awg', class: 'AWG', channel: 1, op: 'frequency', value: 0 })
    expect(retarget(s, vocab, { channel: 5 }).channel).toBe(2)
    const r = retarget({ do: 'read', device: 'psu', class: 'PSU', channel: 1, field: 'IOUT', into: 'i' }, vocab, { device: 'awg' })
    expect(r.field).toBe('FREQ')
  })
})

describe('values and specs', () => {
  it('describes value specs and picks safe defaults', () => {
    expect(specHint({ type: 'number', min: 0, max: 60, unit: 'V' })).toBe('0 – 60 V')
    expect(specHint({ type: 'enum', values: ['OFF', 'ON'] })).toBe('OFF | ON')
    expect(defaultValue({ type: 'enum', values: ['ON', 'OFF'] })).toBe('OFF')
    expect(defaultValue({ type: 'bool' })).toBe(false)
    expect(defaultValue({ type: 'number', min: 2 })).toBe(2)
  })

  it('parses typed literals by spec', () => {
    expect(literalFromText('4.5', { type: 'number' })).toBe(4.5)
    expect(literalFromText('', { type: 'number' })).toBe('')
    expect(literalFromText('ON', { type: 'enum', values: ['ON'] })).toBe('ON')
    expect(literalFromText('12', { type: 'string' })).toBe('12')
    expect(literalFromText('true', undefined)).toBe(true)
  })
})

describe('documents', () => {
  it('lists variables, param values and checks documents', () => {
    const d = insertStep(flowDoc(), 'steps', 2, { do: 'sweep', var: 'u', values: [1], steps: [{ do: 'let', var: 'w', expr: '1' }] })
    expect(variables(d)).toEqual(['target', 'v', 'u', 'w', 'iteration', 'elapsed'])
    expect(paramValues(d)).toEqual({ target: 5 })
    expect(() => asFlow({ schema: 'x' })).toThrow(/benchmesh.flow\/v1/)
    expect(() => asFlow([])).toThrow()
    expect(asFlow(flowDoc()).name).toBe('Ramp')
    expect(sameDoc(flowDoc(), flowDoc())).toBe(true)
  })

  it('matches issues by exact path', () => {
    const issues = [{ path: 'steps[1]', level: 'error' as const, message: 'a' }, { path: 'steps[1].then[0]', level: 'warning' as const, message: 'b' }]
    expect(issuesAt(issues, 'steps[1]').map((i) => i.message)).toEqual(['a'])
    expect(isStepPath('steps[1].then[0]')).toBe(true)
    expect(isStepPath('params[0]')).toBe(false)
  })
})
