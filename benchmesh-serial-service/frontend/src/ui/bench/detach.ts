/**
 * Panels detached into their own window. Each detached window writes a heartbeat into shared
 * storage (all windows of the app share one origin); the bench leaves a panel out while its
 * heartbeat is fresh. A closed - or crashed - window stops beating and the panel comes back.
 */
import { useEffect, useState } from 'react'

export const DETACHED_KEY = 'benchmesh:detached'
export const BEAT_MS = 1000
export const STALE_MS = 3500

function read(): Record<string, number> {
  try {
    const v = JSON.parse(localStorage.getItem(DETACHED_KEY) ?? '{}')
    return v && typeof v === 'object' && !Array.isArray(v) ? v : {}
  } catch {
    return {}
  }
}

function write(v: Record<string, number>) {
  try { localStorage.setItem(DETACHED_KEY, JSON.stringify(v)) } catch { /* storage unavailable */ }
}

export function markDetached(key: string, now = Date.now()) {
  write({ ...read(), [key]: now })
}

export function clearDetached(key: string) {
  const v = read()
  delete v[key]
  write(v)
}

export function detachedKeys(now = Date.now()): string[] {
  return Object.entries(read()).filter(([, t]) => typeof t === 'number' && now - t < STALE_MS).map(([k]) => k).sort()
}

/** Bench side: keys of the panels currently shown in their own window. */
export function useDetachedKeys(): string[] {
  const [keys, setKeys] = useState<string[]>(() => detachedKeys())
  useEffect(() => {
    const update = () => setKeys((prev) => {
      const next = detachedKeys()
      return next.join('|') === prev.join('|') ? prev : next
    })
    const t = setInterval(update, BEAT_MS)
    const onStorage = (e: StorageEvent) => { if (e.key === DETACHED_KEY) update() }
    window.addEventListener('storage', onStorage)
    return () => { clearInterval(t); window.removeEventListener('storage', onStorage) }
  }, [])
  return keys
}

/** Detached-window side: keep the panel registered as detached while this window lives. */
export function useDetachedHeartbeat(key: string) {
  useEffect(() => {
    markDetached(key)
    const t = setInterval(() => markDetached(key), BEAT_MS)
    const bye = () => clearDetached(key)
    window.addEventListener('pagehide', bye)
    return () => { clearInterval(t); window.removeEventListener('pagehide', bye); bye() }
  }, [key])
}
