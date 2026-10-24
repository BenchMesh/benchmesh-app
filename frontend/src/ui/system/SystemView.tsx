/**
 * Instruments - System: the computer BenchMesh runs on (resources, temperature, uptime) and, when
 * BenchMesh runs headless (`benchmesh --headless`, no window, used through remote access),
 * Restart, Shut down and Start with this computer (contract `ai_context/HEADLESS.md`).
 */
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { MicroSwitch } from '../kit/controls'
import { PageLink } from '../about/links'
import { useViewOnly } from '../remote/RemoteProvider'
import { ApiError } from '../automation/client'
import * as api from './client'
import type { Autostart, SystemInfo } from './client'
import { formatBytes, formatDuration, percentOf, temperatureLevel, type Level } from './format'
import { CopyCommand, Meter, TwoPress } from './parts'
import { extensions } from '../../extensions'
import { useAppVersion } from '../about/version'
import type { Stopping } from '../../extensions/contract'
import './system.css'

const POLL_MS = 2000
const COMEBACK_MS = 90_000

type Phase = 'running' | 'restarting' | 'handed-over' | 'late' | 'stopped'

function Fact({ label, value, level }: { label: string; value: React.ReactNode; level?: Level }) {
  return (
    <div className="bench-metric">
      <dt className="kit-label">{label}</dt>
      <dd className="bench-metric__value" data-state={level && level !== 'ok' ? level : undefined}>{value}</dd>
    </div>
  )
}

function AutostartGroup({ viewOnly }: { viewOnly: boolean }) {
  const [a, setA] = useState<Autostart | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    api.getAutostart().then(setA, (e) => setError(e instanceof Error ? e.message : String(e)))
  }, [])
  const change = async (enabled: boolean) => {
    setError(null)
    try { setA(await api.setAutostart(enabled)) } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      throw e
    }
  }
  return (
    <div className="bench-group" role="group" aria-label="Start with this computer">
      {a && !a.supported && <><span className="kit-label">Start with this computer</span><p className="bench-hint">{a.reason ? a.reason[0].toUpperCase() + a.reason.slice(1) + '.' : null}</p></>}
      {a?.supported && (
        <>
          <div className="bench-group__row">
            <MicroSwitch label="Start with this computer" checked={a.enabled} onChange={change} disabled={viewOnly} />
            <span className="bench-hint">
              {a.enabled ? `Starts ${a.starts_at === 'boot' ? 'when the computer is switched on' : 'when you log in'}` : 'BenchMesh starts only when you start it'}
              {a.method ? ` (${a.method})` : ''}.
            </span>
          </div>
          {a.command && <p className="bench-hint">Runs <code>{a.command}</code>{a.location ? <> - entry {a.location}</> : null}</p>}
          {a.enabled && a.note && <p className="bench-hint">{a.note}</p>}
          {a.enabled && a.boot_command && <CopyCommand command={a.boot_command} />}
        </>
      )}
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}

export function SystemView() {
  const viewOnly = useViewOnly()
  const version = useAppVersion()
  const [info, setInfo] = useState<SystemInfo | null>(null)
  /** when `info` arrived (for how long BenchMesh has been running) */
  const [received, setReceived] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const [phase, setPhase] = useState<Phase>('running')
  const before = useRef<number | null>(null)

  const load = useCallback(async () => {
    try {
      const s = await api.getSystem()
      setInfo(s)
      setReceived(Date.now())
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }, [])

  useEffect(() => {
    if (phase !== 'running') return
    const first = setTimeout(() => void load(), 0)
    const t = setInterval(() => void load(), POLL_MS)
    return () => { clearTimeout(first); clearInterval(t) }
  }, [phase, load])

  // after Restart / an extension stopped BenchMesh: reload once a new BenchMesh answers (a remote
  // browser then signs in again)
  useEffect(() => {
    if (phase !== 'restarting' && phase !== 'handed-over') return
    const since = Date.now()
    const t = setInterval(async () => {
      try {
        const s = await api.getSystem()
        if (s.process.started_ms !== before.current) window.location.reload()
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) window.location.reload()
        else if (Date.now() - since > COMEBACK_MS) setPhase('late')
      }
    }, 1000)
    return () => clearInterval(t)
  }, [phase])

  const act = async (what: 'restart' | 'shutdown') => {
    setActionError(null)
    before.current = info?.process.started_ms ?? null
    try {
      await (what === 'restart' ? api.restart() : api.shutdown())
      setPhase(what === 'restart' ? 'restarting' : 'stopped')
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e))
    }
  }

  const startedMs = info?.process.started_ms ?? null
  const [stopping, setStopping] = useState<Stopping | null>(null)
  const onStopping = useCallback((what: Stopping) => {
    before.current = startedMs
    setStopping(what)
    setPhase('handed-over')
  }, [startedMs])

  if (phase !== 'running') {
    return (
      <section className="bench-page" aria-label="System" data-testid="system-view">
        <h3 className="settings-view__section-title">System</h3>
        <div className="bench-group" role="status">
          {phase === 'restarting' && <><span className="kit-label">Restarting…</span><p className="bench-hint">Waiting for BenchMesh to answer again; this page reloads by itself.</p></>}
          {phase === 'handed-over' && stopping && <><span className="kit-label">{stopping.title}</span><p className="bench-hint">{stopping.hint} This page reloads by itself.</p></>}
          {phase === 'late' && <><span className="kit-label">BenchMesh did not come back</span><p className="bench-hint">It has not answered for 90 s: check the computer it runs on.</p></>}
          {phase === 'stopped' && <><span className="kit-label">BenchMesh is stopped</span><p className="bench-hint">Start it again on the computer it runs on.</p></>}
        </div>
      </section>
    )
  }

  const memory = info ? percentOf(info.memory.used, info.memory.total) : 0
  const disk = info?.disk ? percentOf(info.disk.total - info.disk.available, info.disk.total) : 0
  return (
    <section className="bench-page" aria-label="System" data-testid="system-view">
      <h3 className="settings-view__section-title">System</h3>
      <p className="bench-hint">The computer BenchMesh runs on, updated every 2 s.</p>
      {error && <span className="kit-error" role="alert">{error}</span>}
      {info && (
        <>
          <div className="bench-group" role="group" aria-label="This computer">
            <span className="kit-label">{info.host ?? 'This computer'}</span>
            <dl className="bench-metrics">
              <Fact label="System" value={info.os ?? '-'} />
              <Fact label="Processor" value={`${info.arch}, ${info.cpus} ${info.cpus === 1 ? 'core' : 'cores'}`} />
              <Fact label="Up for" value={formatDuration(info.uptime_s)} />
              {info.temperature_c != null && <Fact label="Temperature" value={`${info.temperature_c.toFixed(1)} °C`} level={temperatureLevel(info.temperature_c)} />}
              {info.load && <Fact label="Load 1 / 5 / 15 min" value={info.load.map((l) => l.toFixed(2)).join(' / ')} />}
            </dl>
          </div>
          <div className="bench-group" role="group" aria-label="Resources">
            <span className="kit-label">Resources</span>
            <Meter label="Processor" percent={info.cpu_percent} text={`${info.cpu_percent.toFixed(0)} %`} />
            <Meter label="Memory" percent={memory} text={`${formatBytes(info.memory.used)} of ${formatBytes(info.memory.total)}`} />
            {info.disk && <Meter label="Disk" percent={disk} text={`${formatBytes(info.disk.available)} free of ${formatBytes(info.disk.total)}`} />}
            {info.disk && <p className="bench-hint">Disk of the data folder {info.disk.path} (recordings grow there).</p>}
          </div>
          <div className="bench-group" role="group" aria-label="BenchMesh">
            <span className="kit-label">BenchMesh{version?.official ? ` ${info.version}` : ''}</span>
            <dl className="bench-metrics">
              <Fact label="Running for" value={formatDuration((received - info.process.started_ms) / 1000)} />
              <Fact label="Memory" value={formatBytes(info.process.memory)} />
              <Fact label="Processor" value={info.process.cpu_percent == null ? '-' : `${info.process.cpu_percent.toFixed(1)} %`} />
            </dl>
            {info.headless ? (
              <>
                <div className="bench-group__row">
                  <TwoPress label="Restart" confirmLabel="Press again: restart" disabled={viewOnly} onConfirm={() => act('restart')} />
                  <TwoPress label="Shut down" confirmLabel="Press again: shut down" danger disabled={viewOnly} onConfirm={() => act('shutdown')} />
                </div>
                <p className="bench-hint">Both stop the instruments cleanly first. After a restart remote browsers sign in again; after shutting down, BenchMesh starts again only on the computer itself (or with it, below).</p>
                {actionError && <span className="kit-error" role="alert">{actionError}</span>}
              </>
            ) : (
              <p className="bench-hint">
                This BenchMesh runs in the desktop app: closing its window stops it. Run without a window - for example on a
                Raspberry Pi next to the bench - with <code>benchmesh --headless</code>; Restart, Shut down and Start with this
                computer are then here. <PageLink page="headless">Headless BenchMesh</PageLink>
              </p>
            )}
          </div>
          {info.headless && <AutostartGroup viewOnly={viewOnly} />}
          {extensions.systemGroups.map((Group, i) => <Group key={i} viewOnly={viewOnly} onStopping={onStopping} />)}
        </>
      )}
    </section>
  )
}
