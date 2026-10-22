/**
 * Who this UI is (`GET /remote/session`, once) and, for the local principal, the state of remote
 * access (`GET /remote` every 3 s: the top-bar badge and the settings). A remote browser re-checks
 * its session every 15 s and goes to the sign-in page when it has ended (401: logged out,
 * disconnected from the app, password changed, remote access switched off).
 *
 * An older server without remote access answers 404: the UI is then local and the feature absent
 * (`status` stays null). Errors of the polls are tolerated silently; the next poll retries.
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import { ApiError } from '../automation/client'
import { loginUrl, Principal, remoteClient, RemoteClient, RemoteStatus, SessionInfo } from './client'

export const SESSION_CHECK_MS = 15000
export const STATUS_POLL_MS = 3000

export interface RemoteApi {
  /** null until the server answered */
  principal: Principal | null
  session: SessionInfo | null
  /** local principal only; null while unknown or when the server has no remote access */
  status: RemoteStatus | null
  client: RemoteClient
  /** Take a status the server just returned (after `PUT /remote`). */
  setStatus(s: RemoteStatus | null): void
  /** Re-read `GET /remote` now. */
  refresh(): Promise<void>
}

const Ctx = createContext<RemoteApi | null>(null)

/** The remote access state, or null outside a `RemoteProvider` (then the UI is local). */
export const useRemote = () => useContext(Ctx)

/** True for a browser on the network listener: admin-only controls are hidden (they would get 403). */
export const useIsRemote = () => useContext(Ctx)?.principal === 'remote'

/** True for a remote browser signed in with the view-only password: controls are not offered (the server refuses them). */
export const useViewOnly = () => {
  const r = useContext(Ctx)
  return r?.principal === 'remote' && r.session?.role === 'view'
}

const goToLogin = () => window.location.assign(loginUrl())

interface Props {
  children: React.ReactNode
  client?: RemoteClient
  /** Poll `GET /remote` for the local principal (the main window); panel windows only need the session check. */
  watchStatus?: boolean
  /** Where an ended remote session goes (tests replace it). */
  onSessionEnded?: () => void
}

export function RemoteProvider({ children, client = remoteClient, watchStatus = true, onSessionEnded = goToLogin }: Props) {
  const [session, setSession] = useState<SessionInfo | null>(null)
  const [status, setStatus] = useState<RemoteStatus | null>(null)
  const mounted = useRef(true)
  useEffect(() => { mounted.current = true; return () => { mounted.current = false } }, [])
  const ended = useRef(onSessionEnded)
  useEffect(() => { ended.current = onSessionEnded }, [onSessionEnded])

  // who we are: once (a remote browser keeps checking below)
  useEffect(() => {
    client.session()
      .then((s) => { if (mounted.current) setSession(s) })
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) ended.current()
        else if (mounted.current) setSession({ principal: 'local' }) // older server, or not reachable yet
      })
  }, [client])

  const principal = session?.principal ?? null

  useEffect(() => {
    if (principal !== 'remote') return
    const t = setInterval(() => {
      client.session().catch((e) => { if (e instanceof ApiError && e.status === 401) ended.current() })
    }, SESSION_CHECK_MS)
    return () => clearInterval(t)
  }, [client, principal])

  const refresh = useCallback(() => client.status().then(
    (s) => { if (mounted.current) setStatus(s) },
    // 404: a server without remote access; anything else keeps the last known state
    (e) => { if (e instanceof ApiError && e.status === 404 && mounted.current) setStatus(null) },
  ), [client])

  useEffect(() => {
    if (principal !== 'local' || !watchStatus) return
    void refresh()
    const t = setInterval(() => void refresh(), STATUS_POLL_MS)
    return () => clearInterval(t)
  }, [principal, watchStatus, refresh])

  const api = useMemo<RemoteApi>(() => ({ principal, session, status: principal === 'local' ? status : null, client, setStatus, refresh }),
    [principal, session, status, client, refresh])
  return <Ctx.Provider value={api}>{children}</Ctx.Provider>
}
