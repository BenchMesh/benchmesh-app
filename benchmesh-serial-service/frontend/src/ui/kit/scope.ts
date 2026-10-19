/** Oscilloscope arithmetic: 1-2-5 steps, sample codes → screen, compact SI labels. */
import { prefixed } from './format'

const MANTISSAS = [1, 2, 5]

/**
 * Next value of the 1-2-5 sequence (…, 0.1, 0.2, 0.5, 1, 2, 5, …) in direction `dir`
 * (+1 / -1), clamped to [min, max]. Off-sequence values snap to the next step that way.
 */
export function step125(value: number, dir: number, min = 0, max = Infinity): number {
  const seq: number[] = []
  const e = Math.floor(Math.log10(Math.abs(value) || 1))
  for (let k = e - 2; k <= e + 2; k++) for (const m of MANTISSAS) seq.push(Number((m * 10 ** k).toPrecision(6)))
  const eps = Math.abs(value) * 1e-6
  const next = dir > 0 ? seq.find((s) => s > value + eps) : [...seq].reverse().find((s) => s < value - eps)
  return Math.min(max, Math.max(min, next ?? value))
}

/** Divisions from the screen centre of sample `code`: volts = (code − y0)·dy, shifted by `offset`. */
export function screenDivs(code: number, y0: number, dy: number, offset: number, scale: number): number {
  return ((code - y0) * dy + offset) / scale
}

export interface TraceScale { y0: number; dy: number; offset: number; scale: number }

/** Pixel points of a trace spanning the screen width (`vdivs` divisions tall, centre 0). */
export function tracePoints(codes: number[], t: TraceScale, w: number, h: number, vdivs: number): [number, number][] {
  const n = codes.length
  const perDiv = h / vdivs
  return codes.map((c, i) => [n > 1 ? (i / (n - 1)) * w : 0, h / 2 - screenDivs(c, t.y0, t.dy, t.offset, t.scale) * perDiv])
}

/** Compact engineering text: 0.05 V → "50 mV", 312.5e6 Sa/s → "312.5 MSa/s". */
export function siText(x: number | null | undefined, unit: string, digits = 4): string {
  if (x == null || !Number.isFinite(x)) return '-'
  if (x === 0) return `0 ${unit}`
  const p = prefixed(x, digits)
  const num = p.text.trim().replace(/(\.\d*?)0+$/, '$1').replace(/\.$/, '')
  return `${num} ${p.prefix}${unit}`
}
