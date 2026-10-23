/** The application version (`GET /version`): what the server, and so the app window, runs. */
import { useEffect, useState } from 'react'
import { apiBaseUrl } from '../../api/base'

export interface AppVersion {
  /** e.g. `0.1.7` */
  version: string
  /** e.g. `BenchMesh 0.1.7`, or `BenchMesh 0.1.7 (community build)` for a build of the open-source repository */
  display: string
}

export async function fetchVersion(): Promise<AppVersion | null> {
  try {
    const r = await fetch(`${apiBaseUrl()}/version`)
    if (!r.ok) return null
    const v = await r.json()
    const version = String(v.releaseVersion ?? v.version ?? '')
    const name = v.displayVersion ?? (version ? `BenchMesh ${version}` : '')
    if (!name) return null
    // the official distribution names itself plainly; a build from source says so (issue reports)
    return { version, display: v.distribution === 'community' ? `${name} (community build)` : name }
  } catch {
    return null
  }
}

/** The version once known; `null` while loading or when the server is unreachable. */
export function useAppVersion(): AppVersion | null {
  const [v, setV] = useState<AppVersion | null>(null)
  useEffect(() => {
    let live = true
    void fetchVersion().then((x) => live && setV(x))
    return () => { live = false }
  }, [])
  return v
}
