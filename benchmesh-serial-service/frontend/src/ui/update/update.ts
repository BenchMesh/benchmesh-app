/**
 * App updates (desktop app only): the app checks once per start for a newer published release
 * and installs it on request (crates/benchmesh-app/src/update.rs). In a browser there is
 * nothing to update — the server serves whatever UI it was built with.
 */

export interface Available {
  version: string
  current: string
  notes?: string | null
  /** The installation can replace itself; otherwise the releases page is offered. */
  canInstall: boolean
}

export interface Progress { downloaded: number; total?: number | null }

type TauriIpc = {
  core: { invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> }
  event: { listen<T>(event: string, cb: (e: { payload: T }) => void): Promise<() => void> }
}

function ipc(): TauriIpc | null {
  const t = (window as unknown as { __TAURI__?: TauriIpc }).__TAURI__
  return t?.core && t?.event ? t : null
}

/** The newer version, or `null` (none, offline, or not the desktop app). */
export async function checkForUpdate(): Promise<Available | null> {
  const t = ipc()
  if (!t) return null
  try {
    return (await t.core.invoke<Available | null>('update_check')) ?? null
  } catch {
    return null
  }
}

/** Download and install; the app restarts when it succeeds, so this only resolves by rejecting. */
export async function installUpdate(onProgress: (p: Progress) => void): Promise<void> {
  const t = ipc()
  if (!t) throw new Error('updates are installed by the desktop app')
  const unlisten = await t.event.listen<Progress>('update://progress', (e) => onProgress(e.payload))
  try {
    await t.core.invoke('update_install')
  } finally {
    unlisten()
  }
}

export function openReleasesPage() {
  void ipc()?.core.invoke('update_open_page')
}
