/**
 * Class profiles: what one class panel needs to know about a particular instrument, derived
 * from the API alone — features (`GET /instruments/{CLS}/{id}`) and the setters' value specs
 * (`GET /instruments/{CLS}/{id}/methods`). Every PSU uses the same panel; only these limits,
 * steps and optional controls differ between manufacturers.
 */
import { decimalsFromFormat, intDigitsFor } from '../kit/format'
import type { DigitSpec } from '../kit/digitModel'
import { WAVE_SHAPES } from '../kit/WavePreview'

export interface ValueSpec { type: string; min?: number; max?: number; format?: string; unit?: string; values?: string[] }
export interface Setter { value?: ValueSpec; energizes?: boolean }
export type Setters = Record<string, Setter>

export interface NumberSpec extends DigitSpec { unit: string }

export function setterMap(methods: { name: string; http_method: string; value?: ValueSpec; energizes?: boolean }[]): Setters {
  const out: Setters = {}
  for (const m of methods ?? []) if (m.http_method === 'POST') out[m.name] = { value: m.value, energizes: m.energizes }
  return out
}

const isNum = (x: unknown): x is number => typeof x === 'number' && Number.isFinite(x)

function decimalsOfStep(step: number): number {
  const s = String(step)
  return s.includes('e-') ? Number(s.split('e-')[1]) : (s.split('.')[1] ?? '').length
}

/**
 * Digit editor spec for a numeric setter: the tightest of the definition's and the model's
 * limits; decimals from the model's step, else the wire format, else what fits 6 digits.
 */
export function numberSpec(setter: Setter | undefined, model: { min?: number; max?: number; step?: number; unit?: string } = {}): NumberSpec | null {
  const v = setter?.value
  if (!v || (v.type !== 'number' && v.type !== 'integer')) return null
  const max = [v.max, model.max].filter(isNum).reduce((a, b) => Math.min(a, b), Infinity)
  const top = Number.isFinite(max) ? max : 1000
  const intDigits = intDigitsFor(Math.abs(top))
  const decimals = v.type === 'integer' ? 0
    : isNum(model.step) ? decimalsOfStep(model.step)
    : decimalsFromFormat(v.format, Math.min(3, Math.max(0, 6 - intDigits)))
  const resolution = 10 ** -decimals
  const lo = [v.min, model.min].filter(isNum).reduce((a, b) => Math.max(a, b), -Infinity)
  // the smallest positive value the editor can show, so `min: 1e-6` with 0 decimals becomes 1
  const min = !Number.isFinite(lo) ? 0 : lo > 0 ? Math.max(lo, resolution) : lo
  return { min, max: top, intDigits, decimals, unit: model.unit ?? v.unit ?? '' }
}

const enumSetter = (s: Setters, op: string) => s[op]?.value?.type === 'enum'

// ---- PSU -------------------------------------------------------------------------------

export function psuProfile(features: any, setters: Setters) {
  const cv = features?.modes?.CV
  const cc = features?.modes?.CC
  const abs = features?.absolute_limits ?? {}
  return {
    voltage: numberSpec(setters.voltage, { max: cv?.setpoint?.max ?? abs.voltage?.max, step: cv?.setpoint?.step, unit: 'V' }),
    current: numberSpec(setters.current, { max: cc?.setpoint?.max ?? abs.current?.max, step: cc?.setpoint?.step, unit: 'A' }),
    voltageLimit: numberSpec(setters.voltage_limit, { ...cv?.protection?.OVP, unit: 'V' }),
    currentLimit: numberSpec(setters.current_limit, { ...cc?.protection?.OCP, unit: 'A' }),
    output: !!setters.output,
    protectionSwitches: ['ovp', 'ocp'].filter((op) => enumSetter(setters, op)),
    remote: enumSetter(setters, 'remote'),
    maxPower: isNum(abs.power?.max) ? abs.power.max : null,
  }
}
export type PsuProfile = ReturnType<typeof psuProfile>

// ---- ELL -------------------------------------------------------------------------------

const ELL_ORDER = ['CURR', 'VOLT', 'RES', 'POW']
const ELL_SHORT: Record<string, string> = { CURR: 'CC', VOLT: 'CV', RES: 'CR', POW: 'CP' }

export function ellProfile(features: any, setters: Setters) {
  const modes = Object.entries<any>(features?.modes ?? {})
    .filter(([key, m]) => m?.setpoint && setters[key.toLowerCase()])
    .map(([key, m]) => ({
      key,
      op: key.toLowerCase(),
      short: ELL_SHORT[key] ?? key,
      name: m.ui_name ?? key,
      spec: numberSpec(setters[key.toLowerCase()], m.setpoint)!,
    }))
    .filter((m) => m.spec)
    .sort((a, b) => (ELL_ORDER.indexOf(a.key) + 99) % 99 - (ELL_ORDER.indexOf(b.key) + 99) % 99)
  return {
    modes,
    input: !!setters.input,
    remote: enumSetter(setters, 'remote'),
    sense: enumSetter(setters, 'remote_compensation'),
    remoteRequired: features?.compulsory_lock === true,
  }
}
export type EllProfile = ReturnType<typeof ellProfile>

// ---- DMM -------------------------------------------------------------------------------

/** Dial label for a meter function. */
export function modeGlyph(key: string, m: { unit?: string; note?: string }): string {
  const k = key.toUpperCase()
  if (k.startsWith('CONT')) return '·)))'
  if (k.startsWith('DIOD')) return '→|'
  if (k.startsWith('TEMP')) return '°'
  if (k.startsWith('PER')) return 'PER'
  const ac = m.note === 'AC'
  switch (m.unit) {
    case 'V': return ac ? 'V~' : 'V⎓'
    case 'A': return ac ? 'A~' : 'A⎓'
    case 'Ω': return m.note === '4W' ? 'Ω4W' : 'Ω'
    case 'F': return '⊣⊢'
    case 'Hz': return 'Hz'
  }
  return k.slice(0, 4)
}

// Physical dial order: voltage, current, resistance family, then the rest.
const DMM_ORDER = ['VOLT', 'CURR', 'RES', 'FRES', 'CONT', 'DIOD', 'CAP', 'FREQ', 'PER', 'TEMP']
const dmmRank = (key: string) => {
  const k = key.toUpperCase()
  const i = DMM_ORDER.findIndex((p) => k.startsWith(p))
  return (i < 0 ? 99 : i) * 10 + (k.includes('AC') ? 1 : 0)
}

export interface Option { value: string; display: string }

export function dmmProfile(features: any, setters: Setters) {
  const modes = Object.entries<any>(features?.modes ?? {})
    .map(([key, m]) => ({ key, glyph: modeGlyph(key, m ?? {}), name: [m?.ui_name ?? key, m?.note === '4W' ? '4-wire' : ''].filter(Boolean).join(' '), unit: m?.unit ?? '' }))
    .sort((a, b) => dmmRank(a.key) - dmmRank(b.key))
  const R = features?.RANGe ?? {}
  const ranges: Record<string, Option[]> = {}
  for (const [k, list] of Object.entries<any>(R)) {
    if (Array.isArray(list)) ranges[k] = list.filter((r: Option) => r.value !== 'MIN' && r.value !== 'MAX')
  }
  const temp = R.TEMP && !Array.isArray(R.TEMP) ? R.TEMP : {}
  return {
    modes,
    ranges,
    canRange: !!setters.range,
    rates: setters.rate && Array.isArray(features?.rate) ? (features.rate as Option[]) : [],
    tempSensors: setters.temp_sensor && Array.isArray(temp.Sensor) ? (temp.Sensor as Option[]) : [],
    tempScales: setters.temp_scale && Array.isArray(temp.Scale)
      ? (temp.Scale as { symbol: string; unit: string; display: string }[]).map((s) => ({ value: s.symbol, display: s.unit }))
      : [],
    counts: 5,
  }
}
export type DmmProfile = ReturnType<typeof dmmProfile>

// ---- AWG -------------------------------------------------------------------------------

export function awgProfile(features: any, setters: Setters) {
  const shapes = setters.shape?.value?.values ?? []
  return {
    shapes: [...shapes].sort((a, b) => (WAVE_SHAPES.indexOf(a) + 99) % 99 - (WAVE_SHAPES.indexOf(b) + 99) % 99),
    frequency: numberSpec(setters.frequency),
    amplitude: numberSpec(setters.amplitude),
    offset: numberSpec(setters.offset),
    output: !!setters.output,
    channelColors: (features?.channel_colors ?? {}) as Record<string, string>,
  }
}
export type AwgProfile = ReturnType<typeof awgProfile>

// ---- OSC -------------------------------------------------------------------------------

/** Enum values of a setter in front-panel order (`order` first, anything else after). */
function ordered(setters: Setters, op: string, order: string[]): string[] {
  const values = setters[op]?.value?.values ?? []
  const rank = (v: string) => { const i = order.indexOf(v); return i < 0 ? order.length : i }
  return [...values].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b, undefined, { numeric: true }))
}

export function oscProfile(features: any, setters: Setters, channels: number) {
  const range = (r: any, min: number, max: number) => ({ min: isNum(r?.min) ? r.min : min, max: isNum(r?.max) ? r.max : max })
  const chans = Array.from({ length: channels }, (_, i) => `CHAN${i + 1}`)
  const sources = ordered(setters, 'trigger_source', [...chans, 'EXT', 'AC'])
    .filter((s) => !/^CHAN\d+$/.test(s) || chans.includes(s))
  return {
    hdivs: isNum(features?.divisions?.horizontal) ? features.divisions.horizontal : 10,
    vdivs: isNum(features?.divisions?.vertical) ? features.divisions.vertical : 8,
    scale: range(features?.scale, 1e-3, 10),
    timebase: range(features?.timebase, 1e-9, 100),
    couplings: ordered(setters, 'coupling', ['DC', 'AC', 'GND']),
    probes: [...(setters.probe?.value?.values ?? [])].sort((a, b) => Number(a) - Number(b)),
    triggerSources: sources,
    slopes: ordered(setters, 'trigger_slope', ['POS', 'NEG', 'RFAL']),
    sweeps: ordered(setters, 'trigger_sweep', ['AUTO', 'NORM', 'SING']),
    acqTypes: ordered(setters, 'acquire_type', ['NORM', 'PEAK', 'AVER']),
    runActions: ordered(setters, 'run', ['RUN', 'STOP', 'SINGLE', 'AUTO', 'CLEAR']),
    measurements: Array.isArray(features?.measurements) ? (features.measurements as string[]) : [],
    display: enumSetter(setters, 'display'),
    bwlimit: enumSetter(setters, 'bwlimit'),
    invert: enumSetter(setters, 'invert'),
    offset: !!setters.offset,
    timebaseOffset: !!setters.timebase_offset,
    triggerLevel: !!setters.trigger_level,
    channelColors: (features?.channel_colors ?? {}) as Record<string, string>,
  }
}
export type OscProfile = ReturnType<typeof oscProfile>
