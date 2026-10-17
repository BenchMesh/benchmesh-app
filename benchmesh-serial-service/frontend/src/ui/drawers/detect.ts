/**
 * Detect (`POST /devices/detect`, ai_context/migration/IDENTITY.md): response types and the pure
 * logic behind the Detect section of the Instruments drawer — identity summaries, certainty
 * labels, ids for new entries and the add / lock request bodies.
 */

/** USB fingerprint of a config `match` (4 lowercase hex for vid/pid/revision). */
export interface UsbMatch {
  vid: string;
  pid: string;
  serial?: string;
  revision?: string;
  location?: string;
}

/** Config `match`: find the instrument by who it is instead of by its port path. */
export interface DeviceMatch {
  usb: UsbMatch;
  idn_serial?: string;
}

export interface DetectedUsb {
  vid: string | null;
  pid: string | null;
  serial: string | null;
  revision: string | null;
  manufacturer: string | null;
  product: string | null;
  location: string | null;
}

export interface DetectedInstrument {
  driver: string;
  model: string;
  vendor: string;
  family: string;
  idn: string;
  classes: string[];
}

export type Certainty = 'instrument' | 'adapter' | 'chip' | 'position';

export interface DetectedPort {
  port: string;
  kind: 'serial' | 'usbtmc';
  usb: DetectedUsb | null;
  configured_as: string | null;
  locked: boolean;
  instrument: DetectedInstrument | null;
  match: DeviceMatch | null;
  certainty: Certainty | null;
  error: string | null;
}

export interface DetectResponse {
  ports: DetectedPort[];
}

/** A config entry as `GET /config` returns it (unknown keys are kept). */
export type ConfigEntry = { id: string; [key: string]: unknown };

type UsbLike = { vid?: string | null; pid?: string | null; serial?: string | null; revision?: string | null };

/**
 * One-line identity: the instrument's serial number when it has one, else the adapter's USB
 * serial, else the adapter type ("USB 1a86:7523 rev 0264").
 */
export function identitySummary(id: { usb?: UsbLike | null; idn_serial?: string | null } | null | undefined): string {
  if (!id) return '';
  if (id.idn_serial) return `serial ${id.idn_serial}`;
  const usb = id.usb;
  if (!usb) return '';
  if (usb.serial) return `USB serial ${usb.serial}`;
  if (!usb.vid || !usb.pid) return '';
  return `USB ${usb.vid}:${usb.pid}${usb.revision ? ` rev ${usb.revision}` : ''}`;
}

/** How sure the identity is; `warn` when only the USB port position tells it apart. */
export function certaintyLabel(c: Certainty | null | undefined): { text: string; warn: boolean } | null {
  switch (c) {
    case 'instrument': return { text: 'identified by serial number', warn: false };
    case 'adapter': return { text: 'identified by USB serial', warn: false };
    case 'chip': return { text: 'identified by adapter type', warn: false };
    case 'position': return { text: 'identified by USB port position only — moving it to another port loses it', warn: true };
    default: return null;
  }
}

/** "OWON XDM1241"; a definition without a vendor falls back to its driver name. */
export function instrumentName(i: DetectedInstrument): string {
  return `${i.vendor || i.driver} ${i.model}`.trim();
}

/** Max id length the drawer's id field allows. */
const ID_MAX = 20;

function slug(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

/** Id for a new entry: slug of vendor + model, unique against `taken` (`-2`, `-3`, …). */
export function uniqueId(vendor: string, model: string, taken: Iterable<string>): string {
  const used = new Set(taken);
  const base = slug(`${vendor} ${model}`).slice(0, ID_MAX).replace(/-+$/, '') || 'instrument';
  if (!used.has(base)) return base;
  for (let n = 2; ; n++) {
    const suffix = `-${n}`;
    const id = base.slice(0, ID_MAX - suffix.length).replace(/-+$/, '') + suffix;
    if (!used.has(id)) return id;
  }
}

/** Path field of a transport kind: serial entries have `port`, USBTMC entries `device`. */
export function pathField(kind: string | undefined): 'port' | 'device' {
  return kind === 'usbtmc' ? 'device' : 'port';
}

/** `POST /devices` body for a detected, not yet configured instrument. */
export function buildAddEntry(p: DetectedPort, takenIds: Iterable<string>): ConfigEntry {
  if (!p.instrument || !p.match) throw new Error('nothing identified on this port');
  const i = p.instrument;
  return {
    id: uniqueId(i.vendor || i.driver, i.model, takenIds),
    name: instrumentName(i),
    driver: i.driver,
    model: i.model,
    transport: p.kind,
    match: p.match,
    [pathField(p.kind)]: p.port, // last port seen: a hint only
  };
}

/** `PUT /devices/{id}` body locking a configured entry to the detected identity (other fields kept). */
export function buildLockEntry<T extends { id: string }>(entry: T, match: DeviceMatch): Omit<T, '_uuid'> & { match: DeviceMatch } {
  const out: T & { match: DeviceMatch; _uuid?: unknown } = { ...entry, match };
  delete out._uuid; // UI-only accordion key
  return out;
}

/** Text of an API error reply (`{detail}`), else the HTTP status. */
export async function apiError(resp: Response): Promise<string> {
  const data = await resp.json().catch(() => null);
  const d = data?.detail;
  if (typeof d === 'string') return d;
  if (d != null) return JSON.stringify(d);
  return `HTTP ${resp.status}`;
}
