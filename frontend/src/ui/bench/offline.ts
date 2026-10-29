/**
 * Why an instrument is offline, in words for its panel. A port that opens but stays silent is
 * the common case of an instrument that is switched off: its USB interface (a USB-serial chip,
 * often powered by the computer) works without the instrument.
 */
import type { Instrument } from '../types'

export const NO_ANSWER = 'No answer. Is it switched on? Its USB interface can work while the instrument is off.'

/** The panel header's LED: connected (green) or offline and reconnecting (red), with why. */
export function linkLed(online: boolean, health: Instrument['health']): { state: 'ok' | 'alert'; label: string; title: string } {
  if (online) return { state: 'ok', label: 'Connected', title: 'Connected: the instrument answers BenchMesh.' }
  const why = offlineReason(health)
  return { state: 'alert', label: 'Offline', title: `Offline: BenchMesh keeps trying to reconnect.${why ? ` ${why}` : ''}` }
}

export function offlineReason(health: Instrument['health']): string | null {
  const err = health?.last_error?.trim()
  if (!err) return null
  if (/\b0 bytes received\b|no reply|no answer/i.test(err)) return NO_ANSWER
  return err
}
