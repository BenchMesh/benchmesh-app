/** Spectrum analyzer arithmetic: trace levels → screen, frequency of a point, peak, labels. */
import { prefixed } from './format'

/** Frequency of point `i` of an `n`-point sweep from `start` to `stop`. */
export function freqAt(i: number, n: number, start: number, stop: number): number {
  return n > 1 ? start + (i * (stop - start)) / (n - 1) : start
}

/** Fraction (0..1) of the screen width where `f` lies; null outside START..STOP or for zero span. */
export function freqFraction(f: number | null, start: number | null, stop: number | null): number | null {
  if (f == null || start == null || stop == null || stop <= start) return null
  const x = (f - start) / (stop - start)
  return x >= 0 && x <= 1 ? x : null
}

/** Divisions below the top line of a level: (ref − level) / scale. */
export function levelDivs(level: number, ref: number, scale: number): number {
  return (ref - level) / (scale || 10)
}

/** Pixel points of a trace across the screen width; the top line is `ref` (dBm), `scale` dB/div. */
export function spectrumPoints(levels: number[], ref: number, scale: number, w: number, h: number, vdivs: number): [number, number][] {
  const n = levels.length
  const perDiv = h / vdivs
  return levels.map((l, i) => [n > 1 ? (i / (n - 1)) * w : 0, levelDivs(l, ref, scale) * perDiv])
}

/** Index of the highest level (first one on ties), or -1 for an empty trace. */
export function peakIndex(levels: number[] | null | undefined): number {
  if (!levels?.length) return -1
  let best = 0
  for (let i = 1; i < levels.length; i++) if (levels[i] > levels[best]) best = i
  return best
}

/** Point-wise maximum of a held trace and a new one (restarts when the length changes). */
export function maxHold(held: number[] | null, next: number[]): number[] {
  if (!held || held.length !== next.length) return [...next]
  return next.map((v, i) => Math.max(v, held[i]))
}

/** Frequency as an SA annotation: 433920000 → "433.920 MHz", 0 → "0 Hz". */
export function freqText(hz: number | null | undefined, digits = 6): string {
  if (hz == null || !Number.isFinite(hz)) return '—'
  if (hz === 0) return '0 Hz'
  const p = prefixed(hz, digits)
  return `${p.text.trim()} ${p.prefix}Hz`
}

/** Level as "−42.31 dBm" (one or two decimals, real minus sign). */
export function dbText(db: number | null | undefined, unit = 'dBm', decimals = 2): string {
  if (db == null || !Number.isFinite(db)) return '—'
  return `${db < 0 ? '−' : ''}${Math.abs(db).toFixed(decimals)} ${unit}`
}
