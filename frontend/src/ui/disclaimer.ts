/**
 * Whether the start screen (DisclaimerModal) was accepted with "don't show again": one
 * `localStorage` flag, read at start by App and switched back in Instruments - Miscellaneous.
 */
export const DISCLAIMER_KEY = 'benchmesh:disclaimer-accepted'

/** True when the start screen should not be shown (storage unavailable: show it). */
export function disclaimerHidden(): boolean {
  try { return localStorage.getItem(DISCLAIMER_KEY) === 'true' } catch { return false }
}

export function setDisclaimerHidden(hidden: boolean): void {
  try {
    if (hidden) localStorage.setItem(DISCLAIMER_KEY, 'true')
    else localStorage.removeItem(DISCLAIMER_KEY)
  } catch { /* storage unavailable */ }
}
