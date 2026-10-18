/**
 * Detect section of the Instruments drawer: `POST /devices/detect` identifies the instruments on
 * the USB ports; each row offers to add the instrument or to lock a configured entry to its
 * identity (`match`) — ai_context/migration/IDENTITY.md.
 */

import { useState } from 'react';
import {
  apiError, buildAddEntry, buildLockEntry, certaintyLabel, identitySummary, instrumentName,
  type DetectedPort, type DetectResponse,
} from './detect';
import { InstallDriverButton } from './InstallDriverButton';

interface DetectViewProps {
  apiBase: string;
  /** Saved config entries (ids for uniqueness, entries to lock). */
  configured: ReadonlyArray<{ id: string }>;
  /** A config entry was added or changed on the server. */
  onChanged: () => void;
}

interface RowState { busy?: boolean; error?: string | null }

export function DetectView({ apiBase, configured, onChanged }: DetectViewProps) {
  const [running, setRunning] = useState(false);
  const [ports, setPorts] = useState<DetectedPort[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rows, setRows] = useState<Record<string, RowState>>({});
  // USBTMC instruments BenchMesh can give a driver (Windows; from /usbtmc-devices)
  const [installable, setInstallable] = useState<Set<string>>(new Set());

  const setRow = (port: string, s: RowState) => setRows((prev) => ({ ...prev, [port]: s }));
  const patchPort = (port: string, patch: Partial<DetectedPort>) =>
    setPorts((prev) => prev && prev.map((p) => (p.port === port ? { ...p, ...patch } : p)));

  async function detect() {
    setRunning(true);
    setError(null);
    setRows({});
    try {
      const resp = await fetch(`${apiBase}/devices/detect`, { method: 'POST' });
      if (!resp.ok) throw new Error(await apiError(resp));
      const data: DetectResponse = await resp.json();
      const found = Array.isArray(data?.ports) ? data.ports : [];
      setPorts(found);
      setInstallable(found.some((p) => p.kind === 'usbtmc' && p.error) ? await driverInstallable() : new Set());
    } catch (e: any) {
      console.error('Detect failed:', e);
      setError(e?.message || String(e));
    } finally {
      setRunning(false);
    }
  }

  async function driverInstallable(): Promise<Set<string>> {
    try {
      const resp = await fetch(`${apiBase}/usbtmc-devices`);
      if (!resp.ok) return new Set();
      const list: Array<{ device: string; driver_installable?: boolean }> = await resp.json();
      return new Set(list.filter((d) => d.driver_installable).map((d) => d.device));
    } catch {
      return new Set();
    }
  }

  async function send(p: DetectedPort, method: 'POST' | 'PUT', url: string, body: object, done: Partial<DetectedPort>) {
    setRow(p.port, { busy: true, error: null });
    try {
      const resp = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!resp.ok) throw new Error(await apiError(resp));
      setRow(p.port, {});
      patchPort(p.port, done);
      onChanged();
    } catch (e: any) {
      console.error(`${method} ${url} failed:`, e);
      setRow(p.port, { error: e?.message || String(e) });
    }
  }

  function add(p: DetectedPort) {
    const entry = buildAddEntry(p, configured.map((d) => d.id));
    void send(p, 'POST', `${apiBase}/devices`, entry, { configured_as: entry.id, locked: true });
  }

  function lock(p: DetectedPort) {
    const entry = configured.find((d) => d.id === p.configured_as);
    if (!entry || !p.match) {
      setRow(p.port, { error: `${p.configured_as} is not in the configuration` });
      return;
    }
    void send(p, 'PUT', `${apiBase}/devices/${encodeURIComponent(entry.id)}`, buildLockEntry(entry, p.match), { locked: true });
  }

  function action(p: DetectedPort) {
    const st = rows[p.port];
    if (p.configured_as) {
      if (p.locked) return <span className="detect__done">✓ configured as {p.configured_as}</span>;
      if (!p.match) return <span className="detect__done">configured as {p.configured_as}</span>;
      return (
        <button
          type="button" className="detect__btn" onClick={() => lock(p)} disabled={st?.busy}
          title="Find it by its identity on any USB port instead of by this port path"
        >
          {st?.busy ? 'Locking…' : 'Lock to this instrument'}
        </button>
      );
    }
    if (!p.instrument || !p.match) return null;
    return (
      <button type="button" className="detect__btn" onClick={() => add(p)} disabled={st?.busy} title="Add this instrument to the configuration">
        {st?.busy ? 'Adding…' : 'Add'}
      </button>
    );
  }

  return (
    <section className="detect" aria-label="Detect instruments" data-testid="detect-view">
      <div className="detect__bar">
        <button
          type="button" className={`detect__run${running ? ' detect__run--busy' : ''}`}
          onClick={detect} disabled={running} aria-busy={running} data-testid="detect-btn"
        >
          {running ? 'Detecting…' : 'Detect instruments'}
        </button>
        <span className="detect__hint">Sends only an identify query to USB ports no instrument is using.</span>
      </div>

      {error && <div className="instrument-config__inline-error">{error}</div>}

      {ports && ports.length === 0 && <p className="detect__empty">No USB serial ports or USBTMC instruments found.</p>}

      {ports && ports.length > 0 && (
        <ul className="detect__list">
          {ports.map((p) => {
            const cert = p.instrument ? certaintyLabel(p.certainty) : null;
            const summary = identitySummary(p.match ?? (p.usb ? { usb: p.usb } : null));
            const st = rows[p.port];
            return (
              <li key={p.port} className="detect__row" data-testid="detect-row">
                <div className="detect__main">
                  <span className="detect__name">
                    {p.instrument ? instrumentName(p.instrument) : 'Unknown device'}
                    {!p.instrument && (p.usb?.product || p.usb?.manufacturer) && (
                      <span className="detect__product"> · {p.usb.product || p.usb.manufacturer}</span>
                    )}
                  </span>
                  <span className="detect__action">{action(p)}</span>
                </div>
                <div className="detect__meta">
                  <span className="detect__port">{p.port}</span>
                  {summary && <span>{summary}</span>}
                </div>
                {cert && <div className={`detect__cert${cert.warn ? ' detect__cert--warn' : ''}`}>{cert.text}</div>}
                {p.error && <div className="detect__muted">{p.error}</div>}
                {installable.has(p.port) && <InstallDriverButton apiBase={apiBase} device={p.port} onInstalled={detect} />}
                {st?.error && <div className="instrument-config__inline-error">{st.error}</div>}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
