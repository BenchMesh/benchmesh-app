import { describe, it, expect } from 'vitest'
import fixtures from './bench-fixtures.json'
import { psuProfile, ellProfile, dmmProfile, awgProfile, oscProfile, setterMap, modeGlyph } from '../profiles'

const fx = (key: keyof typeof fixtures) => {
  const f = fixtures[key] as { features: any; methods: any[] }
  return [f.features, setterMap(f.methods)] as const
}

describe('PSU profile (same panel, per-model limits)', () => {
  it('Tenma 72-2540: model limits from features, decimals from the wire format', () => {
    const p = psuProfile(...fx('tenma-72-2540:PSU'))
    expect(p.voltage).toMatchObject({ min: 0, max: 30, intDigits: 2, decimals: 2, unit: 'V' })
    expect(p.current).toMatchObject({ min: 0, max: 5, intDigits: 1, decimals: 3, unit: 'A' })
    expect(p.output).toBe(true)
    expect(p.protectionSwitches).toEqual(['ovp', 'ocp'])
    expect(p.voltageLimit).toBeNull()
    expect(p.remote).toBe(false)
  })
  it('OWON SPM3103: mode setpoint limits and steps, protection levels, remote', () => {
    const p = psuProfile(...fx('owon-spm3103:PSU'))
    expect(p.voltage).toMatchObject({ max: 31, decimals: 2 })
    expect(p.current).toMatchObject({ max: 10.1, intDigits: 2, decimals: 3 })
    expect(p.voltageLimit).toMatchObject({ min: 0.1, max: 32, unit: 'V' })
    expect(p.currentLimit).toMatchObject({ min: 0.1, max: 10.2, unit: 'A' })
    expect(p.protectionSwitches).toEqual([])
    expect(p.remote).toBe(true)
  })
})

describe('ELL profile', () => {
  it('lists settable modes in CC/CV/CR/CP order with their setpoint ranges', () => {
    const p = ellProfile(...fx('owon-oel1515:ELL'))
    expect(p.modes.map((m) => m.key)).toEqual(['CURR', 'VOLT', 'RES', 'POW'])
    expect(p.modes.map((m) => m.short)).toEqual(['CC', 'CV', 'CR', 'CP'])
    const res = p.modes.find((m) => m.key === 'RES')!
    expect(res).toMatchObject({ op: 'res', name: 'Constant Resistance' })
    expect(res.spec).toMatchObject({ min: 0.05, max: 1000, intDigits: 4, decimals: 2, unit: 'Ω' })
    expect(p.modes[0].spec).toMatchObject({ max: 15, intDigits: 2, decimals: 3, unit: 'A' })
    expect(p).toMatchObject({ input: true, remote: true, sense: true, remoteRequired: true })
  })
})

describe('DMM profile', () => {
  it('builds the dial and per-function ranges (without MIN/MAX)', () => {
    const p = dmmProfile(...fx('owon-spm3103:DMM'))
    expect(p.modes.map((m) => m.key)).toEqual(['VOLT_DC', 'VOLT_AC', 'CURR_DC', 'CURR_AC', 'RES', 'CONT', 'DIOD', 'CAP'])
    expect(p.modes.map((m) => m.glyph)).toEqual(['V⎓', 'V~', 'A⎓', 'A~', 'Ω', '·)))', '→|', '⊣⊢'])
    expect(p.ranges.VOLT_DC.map((r) => r.value)).toEqual(['AUTO', '200E-3', '2', '20', '200', '1000'])
    expect(p.canRange).toBe(true)
    expect(p.rates).toEqual([])
  })
  it('XDM1241: rates and temperature options', () => {
    const p = dmmProfile(...fx('owon-xdm1241:DMM'))
    expect(p.modes.find((m) => m.key === 'FRESistance')?.glyph).toBe('Ω4W')
    expect(p.rates.map((r) => r.value)).toEqual(['F', 'M', 'L'])
    expect(p.tempSensors.map((s) => s.value)).toEqual(['KITS90', 'PT100'])
    expect(p.tempScales.map((s) => s.value)).toEqual(['C', 'F', 'K'])
  })
  it('falls back to a short key for unknown functions', () => {
    expect(modeGlyph('WEIRD', {})).toBe('WEIR')
  })
})

describe('AWG profile', () => {
  it('takes waveforms and limits from the definition', () => {
    const p = awgProfile(...fx('owon-dge2070:AWG'))
    expect(p.shapes).toEqual(['SIN', 'SQU', 'RAMP', 'PUL', 'PRN'])
    expect(p.frequency).toMatchObject({ min: 1, max: 70e6, intDigits: 8, decimals: 0, unit: 'Hz' })
    expect(p.amplitude).toMatchObject({ min: 0.001, max: 20, intDigits: 2, decimals: 3, unit: 'Vpp' })
    expect(p.offset).toMatchObject({ min: -10, max: 10, decimals: 3 })
    expect(p.channelColors).toEqual({ 1: '#FFD700', 2: '#4169E1' })
  })
})

describe('OSC profile', () => {
  it('RIGOL DHO804: ranges from features, options from the setters, in front-panel order', () => {
    const [features, setters] = fx('rigol-dho804:OSC')
    const p = oscProfile(features, setters, 4)
    expect(p).toMatchObject({ hdivs: 10, vdivs: 8, scale: { min: 0.0005, max: 10 }, timebase: { min: 5e-9, max: 1000 } })
    expect(p.couplings).toEqual(['DC', 'AC', 'GND'])
    expect(p.probes).toEqual(['0.1', '1', '10', '100', '1000'])
    expect(p.triggerSources).toEqual(['CHAN1', 'CHAN2', 'CHAN3', 'CHAN4', 'AC'])
    expect(p.slopes).toEqual(['POS', 'NEG', 'RFAL'])
    expect(p.sweeps).toEqual(['AUTO', 'NORM', 'SING'])
    expect(p.acqTypes).toEqual(['NORM', 'PEAK', 'AVER'])
    expect(p.runActions).toEqual(['RUN', 'STOP', 'SINGLE', 'AUTO', 'CLEAR'])
    expect(p.measurements).toEqual(['VPP', 'VAVG', 'VRMS', 'FREQ'])
    expect(p).toMatchObject({ bwlimit: true, invert: true, offset: true, timebaseOffset: true, triggerLevel: true })
  })
  it('a 2-channel model only offers its own channels as trigger sources', () => {
    const [features, setters] = fx('rigol-dho804:OSC')
    expect(oscProfile(features, setters, 2).triggerSources).toEqual(['CHAN1', 'CHAN2', 'AC'])
  })
  it('works without optional features', () => {
    const p = oscProfile({}, {}, 1)
    expect(p).toMatchObject({ hdivs: 10, vdivs: 8, couplings: [], triggerSources: [], measurements: [] })
  })
})
