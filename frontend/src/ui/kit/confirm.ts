import { useCallback, useEffect, useState } from 'react'

/** How long a control waits for the instrument to report a requested value. */
export const CONFIRM_TIMEOUT_MS = 6000

export type ConfirmState = 'idle' | 'waiting' | 'unconfirmed'

/**
 * Confirmation of a requested change by the instrument itself: after `begin(target)` the
 * control is `waiting` until the reported value equals the target (then `idle` again), or
 * `unconfirmed` once CONFIRM_TIMEOUT_MS passes without that. A request returning OK only
 * means the command was sent; the reported value is what the instrument actually did.
 */
export function useConfirmation<T>(reported: T, equals: (a: T, b: T) => boolean = Object.is) {
  const [req, setReq] = useState<{ target: T; expired: boolean } | null>(null)
  const confirmed = req != null && equals(reported, req.target)
  // adjust state while rendering: the instrument reported the target
  if (confirmed) setReq(null)

  useEffect(() => {
    if (!req || req.expired) return
    const t = setTimeout(() => setReq((r) => (r && !r.expired ? { ...r, expired: true } : r)), CONFIRM_TIMEOUT_MS)
    return () => clearTimeout(t)
  }, [req])

  const begin = useCallback((target: T) => setReq({ target, expired: false }), [])
  const cancel = useCallback(() => setReq(null), [])
  const active = req != null && !confirmed
  return {
    state: (!active ? 'idle' : req.expired ? 'unconfirmed' : 'waiting') as ConfirmState,
    target: active ? req.target : null,
    begin,
    cancel,
  }
}

/** Numbers are confirmed when equal at a relative precision (instruments round). */
export const nearly = (a: number | null, b: number | null) =>
  a != null && b != null && Math.abs(a - b) <= Math.max(1e-12, Math.abs(b) * 1e-6)

export const NOT_CONFIRMED = 'Not confirmed: the instrument has not reported this value'
