import { describe, it, expect } from 'vitest'
import fixtures from './lcr-sal-fixtures.json'
import { lcrProfile, lcrFunction, salProfile, setterMap, steps1310 } from '../profiles'

// Hand-made from the LCR/SAL definitions' features and /methods value specs (no simulated capture yet).
const fx = (key: keyof typeof fixtures) => {
  const f = fixtures[key] as { features: any; methods: any[] }
  return [f.features, setterMap(f.methods)] as const
}

describe('LCR profile', () => {
  it('Tonghui TH2817B+: functions in front-panel order, continuous frequency and level, options from the setters', () => {
    const p = lcrProfile(...fx('tonghui-th2817b:LCR'))
    expect(p.functions.map((f) => f.key)).toEqual(['LSD', 'LPQ', 'LSQ', 'LSRS', 'CPD', 'CSD', 'CPQ', 'CSQ', 'CSRS', 'CPRP', 'RX', 'ZTD', 'ZTR'])
    expect(p.functions.find((f) => f.key === 'CSRS')).toMatchObject({ primary: 'Cs', secondary: 'Rs', circuit: 'SER', units: ['F', 'Ω'], group: 'C' })
    expect(p.frequency).toMatchObject({ kind: 'range', spec: { min: 50, max: 100000, intDigits: 6, decimals: 0, unit: 'Hz' }, presets: [100, 120, 1000, 10000, 100000] })
    expect(p.level).toMatchObject({ kind: 'range', spec: { min: 0.1, max: 1, decimals: 3, unit: 'V' } })
    expect(p.speeds).toEqual([{ value: 'FAST', display: 'fast (53/s)' }, { value: 'MED', display: 'medium (12/s)' }, { value: 'SLOW', display: 'slow (3/s)' }])
    expect(p.ranges.map((r) => r.display)).toEqual(['AUTO', '10 Ω', '100 Ω', '1 kΩ', '10 kΩ', '100 kΩ'])
    expect(p.triggerSources).toEqual(['INT', 'BUS', 'EXT', 'HOLD'])
    expect(p).toMatchObject({ canFunction: true, trigger: true, corrections: ['OPEN', 'SHORT'], correctionSwitches: ['open_correction', 'short_correction'], bias: null, biasOutput: false, counts: 5 })
  })

  it('East Tester ET4410: discrete frequency and level lists, hardware-decided correction, bias', () => {
    const p = lcrProfile(...fx('easttester-et4410:LCR'))
    expect(p.frequency?.kind).toBe('list')
    expect(p.frequency?.kind === 'list' && p.frequency.values.map((v) => v.num).slice(0, 3)).toEqual([100, 120, 200])
    expect(p.level?.kind === 'list' && p.level.values.map((v) => v.value)).toEqual(['0.1', '0.3', '0.6', '1', '1.5', '2'])
    expect(p.corrections).toEqual(['AUTO'])
    expect(p.ranges.map((r) => r.value)).toEqual(['AUTO', 'HOLD'])
    expect(p.bias).toMatchObject({ min: 0, max: 1.5, decimals: 3, unit: 'V' }) // wire format counts mV
    expect(p).toMatchObject({ biasOutput: false, biasEnergizes: true, circuit: true, triggerSources: ['INT', 'MAN', 'EXT'] })
    expect(p.functions.find((f) => f.key === 'DCR')).toMatchObject({ group: 'DCR', secondary: null, units: ['Ω', ''] })
    expect(p.functions.at(-1)?.key).toBe('DCR')
  })

  it('a numeric setter with a model point list (ET44 behind a shared ET44/ET45 definition) shows the list', () => {
    const setters = { frequency: { value: { type: 'number', min: 10, max: 100000, unit: 'Hz' } } }
    const p = lcrProfile({ frequency: { list: [100, 1000, 10000] } }, setters)
    expect(p.frequency).toEqual({ kind: 'list', values: [{ value: '100', num: 100 }, { value: '1000', num: 1000 }, { value: '10000', num: 10000 }] })
    expect(lcrProfile({ frequency: { min: 10, max: 100000 } }, setters).frequency?.kind).toBe('range')
  })

  it('read-only meter: no controls, functions from features', () => {
    const p = lcrProfile({ functions: { CSD: {}, LSQ: {} }, counts: 20000 }, {})
    expect(p.functions.map((f) => f.key)).toEqual(['LSQ', 'CSD'])
    expect(p).toMatchObject({ canFunction: false, frequency: null, level: null, speeds: [], ranges: [], corrections: [], counts: 5 })
  })

  it('canonical tokens fill what features leave out', () => {
    expect(lcrFunction('ZTR')).toMatchObject({ primary: 'Z', secondary: 'θ', circuit: null, units: ['Ω', 'rad'], name: 'Z-θ' })
    expect(lcrFunction('XYZ')).toMatchObject({ primary: 'XYZ', secondary: null, group: 'X' })
  })
})

describe('SAL profile', () => {
  it('tinySA Ultra: frequency editors in the model range, RBW list from features, generator', () => {
    const p = salProfile(...fx('tinysa-ultra:SAL'))
    expect(p).toMatchObject({ hdivs: 10, vdivs: 10, freqRange: { min: 100000, max: 5.3e9 } })
    expect(p.center).toMatchObject({ min: 100000, max: 5.3e9, decimals: 0, unit: 'Hz' })
    expect(p.span).toMatchObject({ min: 0, max: 5.3e9 })
    expect(p.rbw).toEqual([200, 1000, 3000, 10000, 30000, 100000, 300000, 600000, 850000])
    expect(p.vbw).toEqual([])
    expect(p.scales).toEqual([1, 2, 5, 10])
    expect(p.attenuation).toEqual({ min: 0, max: 31, step: 1 })
    expect(p.sweeps).toEqual(['CONT', 'SINGLE', 'HOLD'])
    expect(p.peaks).toEqual(['PEAK'])
    expect(p.tg?.level).toMatchObject({ min: -115, max: 13, unit: 'dBm' })
    expect(p).toMatchObject({ fullSpan: false, rbwAuto: true, attenAuto: true, preamp: false, marker: true })
  })

  it('Rigol DSA815: 1-3-10 bandwidths from the setter limits, detectors and trace modes in front-panel order', () => {
    const p = salProfile(...fx('rigol-dsa815:SAL'))
    expect(p.rbw).toEqual([10, 30, 100, 300, 1000, 3000, 10000, 30000, 100000, 300000, 1000000])
    expect(p.vbw[0]).toBe(1)
    expect(p.vbw.at(-1)).toBe(3000000)
    expect(p.detectors).toEqual(['POS', 'NEG', 'NORM', 'SAMP', 'RMS', 'QPE'])
    expect(p.traceModes).toEqual(['WRITE', 'MAXHOLD', 'MINHOLD', 'VIEW'])
    expect(p.sweeps).toEqual(['CONT', 'SINGLE', 'RESTART'])
    expect(p.peaks).toEqual(['PEAK', 'NEXT', 'MKR_CF'])
    expect(p).toMatchObject({ fullSpan: true, preamp: true, tg: null, refLevel: { min: -100, max: 20 } })
  })

  it('no generator control without features.tg (a model without TG)', () => {
    const [features, setters] = fx('tinysa-ultra:SAL')
    expect(salProfile({ ...features, tg: undefined }, setters).tg).toBeNull()
  })

  it('works without optional features', () => {
    expect(salProfile({}, {})).toMatchObject({ hdivs: 10, vdivs: 10, freqRange: null, center: null, rbw: [], tg: null, refLevel: null })
  })

  it('1-3-10 steps', () => {
    expect(steps1310(100, 3000)).toEqual([100, 300, 1000, 3000])
    expect(steps1310(0, 10)).toEqual([])
  })
})
