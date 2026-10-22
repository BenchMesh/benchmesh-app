/** Figures of Instruments - System: sizes, durations and how full something is. */

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB']

/** 1536 -> "1.5 KB" (powers of 1024, one decimal below 10). */
export function formatBytes(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n) || n < 0) return '-'
  let v = n
  let i = 0
  while (v >= 1024 && i < UNITS.length - 1) { v /= 1024; i++ }
  return `${i === 0 || v >= 10 ? Math.round(v) : v.toFixed(1)} ${UNITS[i]}`
}

/** 93784 -> "1 d 2 h", 3720 -> "1 h 2 min", 59 -> "59 s". */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return '-'
  const s = Math.floor(seconds)
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60)
  if (d > 0) return `${d} d ${h} h`
  if (h > 0) return `${h} h ${m} min`
  if (m > 0) return `${m} min`
  return `${s} s`
}

export type Level = 'ok' | 'warn' | 'alert'

/** How full: below 75 % fine, from 90 % alarming. */
export function levelOf(percent: number): Level {
  return percent >= 90 ? 'alert' : percent >= 75 ? 'warn' : 'ok'
}

/** Processor temperature: a Raspberry Pi throttles from 80 °C. */
export function temperatureLevel(c: number): Level {
  return c >= 80 ? 'alert' : c >= 70 ? 'warn' : 'ok'
}

/** used of total as a percentage (0 when total is unknown). */
export function percentOf(used: number, total: number): number {
  return total > 0 ? Math.min(100, Math.max(0, (used / total) * 100)) : 0
}
