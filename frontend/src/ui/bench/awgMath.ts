/**
 * Generator quantities the instrument does not have to provide: BenchMesh derives them from
 * what it reports (frequency, Vpp amplitude, offset, waveform, load) and translates an entry in
 * one of them back into the native setters. Pure arithmetic, so the result is as exact as the
 * instrument's own setting; anything that cannot be derived exactly gives null, never a guess.
 */

/** Period (s) of a frequency (Hz), and back: the same function. */
export function reciprocal(x: number | null): number | null {
  return x != null && Number.isFinite(x) && x > 0 ? 1 / x : null
}

/**
 * Vpp / Vrms of the AC part of a waveform (offset excluded, as generators specify Vrms).
 * Sine 2*sqrt(2), square 2, ramp/triangle 2*sqrt(3) for any symmetry. Pulse depends on the duty
 * cycle, noise and arbitrary waveforms have no fixed ratio: null.
 */
export function peakToRms(shape: string): number | null {
  switch (shape.toUpperCase()) {
    case 'SIN': return 2 * Math.SQRT2
    case 'SQU': return 2
    case 'RAMP': return 2 * Math.sqrt(3)
  }
  return null
}

/**
 * Load impedance in ohm from the `LOAD` status: a number of ohms, `HIGHZ` (Infinity), or null
 * when the instrument does not report it.
 */
export function loadOhms(load: unknown): number | null {
  if (typeof load === 'number') return load > 0 ? (load >= 9e37 ? Infinity : load) : null
  if (typeof load !== 'string') return null
  const s = load.trim().toUpperCase()
  if (s === 'HIGHZ' || s === 'HZ' || s === 'INF') return Infinity
  const n = Number(s)
  return Number.isFinite(n) && n > 0 ? n : null
}

export function vrmsOf(vpp: number | null, shape: string): number | null {
  const k = peakToRms(shape)
  return vpp == null || k == null ? null : vpp / k
}

export function vppFromVrms(vrms: number, shape: string): number | null {
  const k = peakToRms(shape)
  return k == null ? null : vrms * k
}

/** dBm into a finite load (power of the AC part); null at High-Z or an unknown load. */
export function dbmOf(vpp: number | null, shape: string, ohms: number | null): number | null {
  const vrms = vrmsOf(vpp, shape)
  if (vrms == null || vrms <= 0 || ohms == null || !Number.isFinite(ohms)) return null
  return 10 * Math.log10((vrms * vrms) / ohms / 1e-3)
}

export function vppFromDbm(dbm: number, shape: string, ohms: number | null): number | null {
  if (ohms == null || !Number.isFinite(ohms)) return null
  const vrms = Math.sqrt(1e-3 * ohms * 10 ** (dbm / 10))
  return vppFromVrms(vrms, shape)
}

export function highLow(vpp: number | null, offset: number | null): { high: number; low: number } | null {
  return vpp == null || offset == null ? null : { high: offset + vpp / 2, low: offset - vpp / 2 }
}

export function fromHighLow(high: number, low: number): { amplitude: number; offset: number } {
  return { amplitude: high - low, offset: (high + low) / 2 }
}

/** Rounded to `decimals`, so a value derived from a rounded read-back confirms the entry. */
export function roundTo(x: number | null, decimals: number): number | null {
  if (x == null) return null
  const f = 10 ** decimals
  return Math.round(x * f) / f
}

/** Period units, largest first: the one an editor uses for a given period. */
export const PERIOD_UNITS = [
  { unit: 's', scale: 1 },
  { unit: 'ms', scale: 1e-3 },
  { unit: 'µs', scale: 1e-6 },
  { unit: 'ns', scale: 1e-9 },
] as const

export function periodUnitFor(seconds: number | null): (typeof PERIOD_UNITS)[number] {
  if (seconds == null || !(seconds > 0)) return PERIOD_UNITS[1]
  return PERIOD_UNITS.find((u) => seconds >= u.scale) ?? PERIOD_UNITS[PERIOD_UNITS.length - 1]
}
