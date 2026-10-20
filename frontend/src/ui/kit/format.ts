/**
 * Number handling for instrument displays: reading registry values and formatting them
 * as fixed-width instrument text (the readout never changes width while it updates).
 */

/** Base-unit value of a registry reading: an SI dict `{val, si, sci}`, a number or a numeric string. */
export function siNumber(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null
  if (typeof v === 'string') {
    const n = Number(v.trim())
    return v.trim() !== '' && Number.isFinite(n) ? n : null
  }
  if (v && typeof v === 'object' && 'val' in v) return siNumber((v as { val: unknown }).val)
  return null
}

/** `x` with a fixed number of integer and decimal digits and a sign column: ` 04.94`. */
export function fixedText(x: number | null, intDigits: number, decimals: number): string {
  if (x == null || !Number.isFinite(x)) {
    return ' ' + '-'.repeat(intDigits) + (decimals ? '.' + '-'.repeat(decimals) : '')
  }
  const [i, f] = Math.abs(x).toFixed(decimals).split('.')
  return (x < 0 ? '-' : ' ') + i.padStart(intDigits, '0') + (decimals ? '.' + f : '')
}

const PREFIXES: [number, string][] = [[1e9, 'G'], [1e6, 'M'], [1e3, 'k'], [1, ''], [1e-3, 'm'], [1e-6, 'µ'], [1e-9, 'n'], [1e-12, 'p']]

/** Reading scaled to an SI prefix with `counts` significant digits: -9.79e-5 → `-97.893` µ. */
export function prefixed(x: number | null, counts: number, overload = false): { text: string; prefix: string } {
  if (x == null || !Number.isFinite(x)) {
    return { text: overload ? '  OL  ' : ' ' + '-'.repeat(counts), prefix: '' }
  }
  const ax = Math.abs(x)
  const [f, prefix] = ax === 0 ? [1, ''] : PREFIXES.find(([f]) => ax >= f * 0.99995) ?? PREFIXES[PREFIXES.length - 1]
  const v = Math.abs(x / f)
  const intDigits = Math.max(1, Math.floor(Math.log10(v || 1)) + 1)
  const decimals = Math.min(6, Math.max(0, counts - intDigits))
  return { text: (x < 0 ? '-' : ' ') + v.toFixed(decimals), prefix }
}

/** Decimal places of a definition wire format (`%.2f` → 2, `%d` → 0); `%g` and unknown use `fallback`. */
export function decimalsFromFormat(format: string | undefined, fallback: number): number {
  const m = format?.match(/^%\.(\d+)f$/)
  if (m) return Number(m[1])
  if (format === '%d') return 0
  return fallback
}

/** Integer digits needed to show `max`. */
export function intDigitsFor(max: number): number {
  return max >= 1 ? Math.floor(Math.log10(max)) + 1 : 1
}
