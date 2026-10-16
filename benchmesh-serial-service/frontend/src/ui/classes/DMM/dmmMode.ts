/**
 * Mode the DMM panel should show: the instrument's reported MODE (registry) when it is
 * one of the known modes, otherwise the current selection (or the first mode).
 * Front-panel changes on the instrument therefore show up in the UI.
 */
export function displayedMode(current: string, reported: unknown, modes: string[]): string {
  if (typeof reported === 'string' && modes.includes(reported)) return reported
  return current || modes[0] || ''
}
