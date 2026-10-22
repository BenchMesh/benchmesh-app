import { BrowserContext, Page, Route } from '@playwright/test';

/** Mocks go on a page or on the whole browser context (then popups are covered too). */
type Target = Page | BrowserContext;

/**
 * Fully mocked BenchMesh API for the E2E suite.
 *
 * `setupApiMocks` installs one route handler for every API path the UI uses and returns an
 * `ApiMock` handle: tests read `posts` to assert the instrument operations the UI sent, and
 * can change `instruments` / `config` before navigating. Routes a test registers itself with
 * `page.route` take precedence (Playwright matches the most recently added route first).
 */

export interface MockInstrument {
  id: string;
  name?: string;
  IDN?: string;
  classes: { class: string; channels: string[] }[];
  health?: { is_alive?: boolean };
}

/** Three instruments: a PSU and a DMM that are connected, and an LCR meter that is offline. */
export const mockInstruments: MockInstrument[] = [
  { id: 'test-psu-1', name: 'Test PSU', IDN: 'TENMA,72-2540,SN123456,V1.0', classes: [{ class: 'PSU', channels: ['1'] }] },
  { id: 'test-dmm-1', name: 'Test DMM', IDN: 'OWON,XDM1041,SN789012,V2.1', classes: [{ class: 'DMM', channels: ['1'] }] },
  { id: 'test-lcr-1', name: 'Test LCR', classes: [{ class: 'LCR', channels: ['1'] }] },
];

/** Accessible names of the panels for `mockInstruments`, in default order. */
export const PANELS = {
  psu: 'Power Supply Unit: Test PSU',
  dmm: 'Digital Multimeter: Test DMM',
  lcr: 'LCR Meter: Test LCR',
};

const si = (val: number) => ({ val, sci: val.toExponential(4), si: { number: val } });

/** WebSocket registry snapshot: PSU and DMM report an IDN (online); the LCR meter does not. */
export const mockRegistry = {
  'test-psu-1': {
    IDN: 'TENMA,72-2540,SN123456,V1.0',
    PSU: { status_ch1: { VOUT: si(4.93), IOUT: si(0.25), OUTPUT: 'OFF', CV: true, CC: false } },
  },
  'test-dmm-1': {
    IDN: 'OWON,XDM1041,SN789012,V2.1',
    DMM: { status_ch1: { MODE: 'VOLT', MEAS: si(4.98), RANGE: 'AUTO' } },
  },
};

/** Features (`GET /instruments/{CLS}/{id}`) per class. */
export const mockFeatures: Record<string, any> = {
  PSU: { absolute_limits: { voltage: { max: 30 }, current: { max: 5 }, power: { max: 150 } } },
  DMM: {
    modes: {
      VOLT: { unit: 'V', ui_name: 'DC Voltage' },
      CURR: { unit: 'A', ui_name: 'DC Current' },
      RES: { unit: 'Ω', ui_name: 'Resistance' },
    },
    RANGe: {
      VOLT: [
        { value: 'AUTO', display: 'Auto' },
        { value: '0.5', display: '500 mV' },
        { value: '5', display: '5 V' },
        { value: '50', display: '50 V' },
        { value: 'MAX', display: 'Max' },
      ],
    },
  },
  LCR: {},
};

/** Methods (`GET /instruments/{CLS}/{id}/methods`) per class; POST entries carry value specs. */
export const mockMethods: Record<string, any[]> = {
  PSU: [
    { name: 'voltage', http_method: 'GET' },
    { name: 'current', http_method: 'GET' },
    { name: 'voltage', http_method: 'POST', value: { type: 'number', min: 0, max: 30, format: '%.2f', unit: 'V' } },
    { name: 'current', http_method: 'POST', value: { type: 'number', min: 0, max: 5, format: '%.3f', unit: 'A' } },
    { name: 'output', http_method: 'POST', value: { type: 'enum', values: ['ON', 'OFF'] }, energizes: true },
  ],
  DMM: [
    { name: 'mode', http_method: 'POST', value: { type: 'enum', values: ['VOLT', 'CURR', 'RES'] } },
    { name: 'range', http_method: 'POST', value: { type: 'enum', values: ['AUTO', '0.5', '5', '50'] } },
  ],
  LCR: [],
};

/** Values returned by `GET /instruments/{CLS}/{id}/{ch}/{op}`. */
export const mockSetpoints: Record<string, unknown> = { voltage: 5, current: 1 };

export const mockConfig = {
  version: 1,
  devices: [
    { id: 'test-psu-1', name: 'TENMA PSU', driver: 'tenma_72', port: '/dev/ttyUSB0', baud: 9600, serial: '8N1', model: '72-2540' },
    { id: 'test-dmm-1', name: 'OWON DMM', driver: 'owon_xdm', port: '/dev/ttyUSB1', baud: 115200, serial: '8N1', model: 'XDM1041' },
  ],
};

export const mockDrivers = {
  tenma_72: { vendor: 'TENMA', family: '72-Series', supported_transports: ['serial'] },
  owon_xdm: { vendor: 'OWON', family: 'XDM', supported_transports: ['serial', 'usbtmc'] },
};

const mockModels: Record<string, string[]> = {
  tenma_72: ['72-2540', '72-2545', '72-2550'],
  owon_xdm: ['XDM1041', 'XDM1241'],
};

const mockPorts = [
  { device: '/dev/ttyUSB2', description: 'USB Serial Port 2', manufacturer: 'FTDI', serial_number: '12345', hwid: 'USB VID:PID=0403:6001' },
  { device: '/dev/ttyUSB3', description: 'USB Serial Port 3', manufacturer: 'CH340', serial_number: '67890', hwid: 'USB VID:PID=1a86:7523' },
];

/** A small `/openapi.json` like the server's (crates/bm-server/src/openapi.rs), with its extensions. */
export const mockOpenApi = {
  openapi: '3.1.0',
  info: { title: 'BenchMesh API', version: '0.1.7', description: 'Lab instrument control.' },
  tags: [
    { name: 'system', description: 'Service status, version, documentation' },
    { name: 'instrument-control', description: 'Read and set instrument operations' },
    { name: 'recordings', description: 'Data recording' },
    { name: 'websockets', description: 'Real-time WebSocket streams' },
  ],
  paths: {
    '/': { get: { tags: ['system'], summary: 'Redirect to the UI', parameters: [], 'x-no-try': 'Redirects a browser to the UI' } },
    '/status': { get: { tags: ['system'], summary: 'Device connection counts', parameters: [] } },
    '/instruments/{klass}/{id}/{ch}/{op}': { get: { tags: ['instrument-control'], summary: 'Read an operation → {path, value}', parameters: ['klass', 'id', 'ch', 'op'].map((name) => ({ name, in: 'path', required: true })) } },
    '/recordings': { post: { tags: ['recordings'], summary: 'Create a recording {name, description?, interval_s 1-3600}', parameters: [], requestBody: { content: { 'application/json': { schema: { type: 'object' } } } } } },
    '/ws/registry': { get: { tags: ['websockets'], summary: 'WebSocket: live registry', parameters: [], 'x-websocket': true } },
  },
};

const API_PREFIXES = [
  '/instruments', '/drivers', '/config', '/devices', '/serial-ports', '/usbtmc-devices',
  '/status', '/version', '/metrics', '/recordings', '/openapi.json', '/openapi.yaml', '/ai', '/remote', '/scpi',
];

export interface RecordedRequest { method: string; path: string; body?: string | null }

/** Recordings v2 objects as the API returns them (ai_context/migration/RECORDINGS.md). */
export interface MockSeries {
  id: number; device_id: string; class: string; channel: number; field: string; label: string; unit: string;
  added_at: number; removed_at: number | null;
}
export interface MockRecording {
  id: number; name: string; description: string; interval_s: number; state: 'recording' | 'paused' | 'stopped';
  started_at: number; stopped_at: number | null; paused_at: number | null; samples: number; last_t: number | null;
  series: MockSeries[];
}

/** Remote access (ai_context/REMOTE_ACCESS.md): who the UI is, and the network listener's state. */
export interface MockRemoteSession { id: string; address: string; agent: string; since: number; last_seen: number }
export interface MockRemote {
  /** `GET /remote/session`; `{principal: 'remote', ...}` makes the UI a remote browser (admin routes answer 403). */
  session: { principal: 'local' } | { principal: 'remote'; address: string; since: number };
  enabled: boolean;
  password_set: boolean;
  sessions: MockRemoteSession[];
  /** Every request to `/remote...` other than `GET /remote` and `GET /remote/session`. */
  requests: RecordedRequest[];
}

export const REMOTE_URL = 'https://192.168.1.20:57667/';
export const REMOTE_FINGERPRINT = '3F:A2:9C:11:5E:07:D4:B8:6A:20:91:CC:4E:73:F0:1B:88:5D:2A:E6:0C:97:B3:44:1F:6E:A8:52:D9:0B:37:C5';

export interface ApiMock {
  /** Instrument list served by `GET /instruments` (mutable; takes effect on the next poll). */
  instruments: MockInstrument[];
  config: typeof mockConfig;
  /** Every `POST /instruments/...` operation, in order (path is URL-decoded). */
  posts: RecordedRequest[];
  /** Every `/devices` request (add / update / remove instrument). */
  deviceRequests: RecordedRequest[];
  /** Instrument operation paths (as recorded in `posts`) and `/devices/...` paths that should fail with the given reply. */
  failures: Map<string, { status: number; detail: string }>;
  /** Recordings served by the v2 recordings API (mutable; newest first in replies). */
  recordings: MockRecording[];
  /** Every request to `/recordings...` other than GET. */
  recordingRequests: RecordedRequest[];
  /** Samples per series id for `GET /recordings/{id}/data`. */
  samples: Map<number, { t: number[]; v: number[] }>;
  remote: MockRemote;
  /** Virtual SCPI settings (`GET|PUT /scpi`). */
  scpi: { access: 'off' | 'read' | 'control'; base_port: number };
}

const json = (route: Route, body: unknown, status = 200, headers: Record<string, string> = {}) =>
  route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body), headers });

async function handleInstruments(route: Route, api: ApiMock, segs: string[]) {
  const req = route.request();
  const method = req.method();
  // segs: ['instruments', CLS?, id?, ch|'methods'?, op?, value?]
  if (segs.length === 1) {
    return json(route, api.instruments, 200, { ETag: '"test-etag-12345"' });
  }
  const [, cls, id, ch, op] = segs;
  const known = api.instruments.some((i) => i.id === id && i.classes.some((c) => c.class === cls));
  if (!known) return json(route, { detail: `Unknown instrument ${cls}/${id}` }, 404);
  if (segs.length === 3) return json(route, mockFeatures[cls] ?? {});
  if (segs.length === 4 && ch === 'methods') {
    return json(route, { device_id: id, class: cls, methods: mockMethods[cls] ?? [] });
  }
  if (method === 'GET' && segs.length === 5) {
    if (!(op in mockSetpoints)) return json(route, { detail: `No query ${op}` }, 404);
    return json(route, { value: mockSetpoints[op] });
  }
  if (method === 'POST' && segs.length === 6) {
    const path = '/' + segs.join('/');
    api.posts.push({ method, path });
    const fail = api.failures.get(path);
    if (fail) return json(route, { detail: fail.detail }, fail.status);
    return route.fulfill({ status: 204 });
  }
  return json(route, { detail: 'Not found' }, 404);
}

const NUMERIC_FIELDS: Record<string, string[]> = {
  PSU: ['VOUT', 'IOUT', 'POUT'], DMM: ['MEAS'], ELL: ['VIN', 'IIN', 'PIN'], AWG: ['FREQ', 'AMPL', 'OFFS'], OSC: ['VPP', 'VAVG', 'VRMS', 'FREQ'],
};

/** The v2 recordings API over `api.recordings` (validation and conflicts as the server does them). */
async function handleRecordings(route: Route, api: ApiMock, segs: string[], url: URL) {
  const req = route.request();
  const method = req.method();
  const body = (() => { try { return JSON.parse(req.postData() ?? 'null'); } catch { return null; } })();
  if (method !== 'GET') api.recordingRequests.push({ method, path: url.pathname, body: req.postData() });
  const newest = () => [...api.recordings].sort((a, b) => b.started_at - a.started_at || b.id - a.id);
  const now = Date.now();
  if (segs.length === 1) {
    if (method === 'GET') return json(route, newest());
    if (method === 'POST') {
      const name = typeof body?.name === 'string' ? body.name.trim() : '';
      const iv = body?.interval_s;
      if (!name || name.length > 100 || !Number.isInteger(iv) || iv < 1 || iv > 3600) return json(route, { detail: 'Invalid recording' }, 422);
      if (api.recordings.some((r) => r.name === name)) return json(route, { detail: `Recording name '${name}' is taken` }, 409);
      const r: MockRecording = {
        id: Math.max(0, ...api.recordings.map((x) => x.id)) + 1, name, description: body.description ?? '', interval_s: iv,
        state: 'recording', started_at: now, stopped_at: null, paused_at: null, samples: 0, last_t: null, series: [],
      };
      api.recordings.push(r);
      return json(route, r, 201);
    }
  }
  const r = api.recordings.find((x) => x.id === Number(segs[1]));
  if (!r) return json(route, { detail: 'Recording not found' }, 404);
  const [, , op, sid] = segs;
  if (segs.length === 2) {
    if (method === 'GET') return json(route, r);
    if (method === 'DELETE') { api.recordings.splice(api.recordings.indexOf(r), 1); return route.fulfill({ status: 204 }); }
    if (method === 'PATCH') {
      if (typeof body?.name === 'string' && api.recordings.some((x) => x !== r && x.name === body.name.trim())) {
        return json(route, { detail: `Recording name '${body.name}' is taken` }, 409);
      }
      if (typeof body?.name === 'string') r.name = body.name.trim();
      if (typeof body?.description === 'string') r.description = body.description;
      return json(route, r);
    }
  }
  if (op === 'series' && method === 'POST' && segs.length === 3) {
    if (r.state === 'stopped') return json(route, { detail: 'Recording is stopped' }, 409);
    if (!(NUMERIC_FIELDS[body?.class] ?? []).includes(body?.field)) return json(route, { detail: 'Not a numeric status field' }, 422);
    if (r.series.some((x) => x.removed_at == null && x.device_id === body.device_id && x.class === body.class && x.channel === body.channel && x.field === body.field)) {
      return json(route, { detail: 'Signal is already active in this recording' }, 409);
    }
    const s: MockSeries = {
      id: Math.max(0, ...api.recordings.flatMap((x) => x.series.map((y) => y.id))) + 1,
      device_id: body.device_id, class: body.class, channel: body.channel, field: body.field,
      label: body.label ?? `${body.device_id} · ${body.field}`, unit: body.unit ?? '', added_at: now, removed_at: null,
    };
    r.series.push(s);
    return json(route, s, 201);
  }
  if (op === 'series' && method === 'DELETE' && sid) {
    const s = r.series.find((x) => x.id === Number(sid));
    if (!s) return json(route, { detail: 'Series not found' }, 404);
    s.removed_at = now;
    return route.fulfill({ status: 204 });
  }
  if (method === 'POST' && (op === 'pause' || op === 'resume' || op === 'stop')) {
    if (r.state !== 'stopped') {
      if (op === 'pause') { r.state = 'paused'; r.paused_at = now; }
      if (op === 'resume') { r.state = 'recording'; r.paused_at = null; }
      if (op === 'stop') { r.state = 'stopped'; r.stopped_at = now; }
    }
    return json(route, r);
  }
  if (op === 'data' && method === 'GET') {
    const from = Number(url.searchParams.get('from') ?? -Infinity);
    const to = Number(url.searchParams.get('to') ?? Infinity);
    const series = r.series.map((s) => {
      const d = api.samples.get(s.id) ?? { t: [], v: [] };
      const idx = d.t.map((t, i) => [t, i]).filter(([t]) => t >= from && t <= to).map(([, i]) => i);
      return { id: s.id, t: idx.map((i) => d.t[i]), v: idx.map((i) => d.v[i]) };
    });
    return json(route, { recording_id: r.id, from: Number.isFinite(from) ? from : null, to: Number.isFinite(to) ? to : null, series });
  }
  if (op === 'export' && method === 'GET') {
    const header = ['time_utc', 'time_ms', ...r.series.map((s) => `${s.label} [${s.unit}]`)].join(',');
    return route.fulfill({ status: 200, contentType: 'text/csv', body: header + '\n' });
  }
  return json(route, { detail: 'Not found' }, 404);
}

/** Virtual SCPI instruments: one port per mocked instrument from the base port. */
async function handleScpi(route: Route, api: ApiMock) {
  const req = route.request();
  if (req.method() === 'PUT') {
    const body = (() => { try { return JSON.parse(req.postData() ?? 'null'); } catch { return null; } })();
    if (body?.access !== undefined && !['off', 'read', 'control'].includes(body.access)) return json(route, { detail: 'access: use off, read or control' }, 422);
    if (body?.access) api.scpi.access = body.access;
    if (typeof body?.base_port === 'number') api.scpi.base_port = body.base_port;
  }
  const on = api.scpi.access !== 'off';
  const instruments = api.instruments.map((i: any, n: number) => {
    const port = api.scpi.base_port + n;
    return { id: i.id, name: i.name ?? i.id, port, address: `TCPIP0::127.0.0.1::${port}::SOCKET`, listening: on, error: null };
  });
  return json(route, { ...api.scpi, host: '127.0.0.1', instruments });
}

const remoteStatus = (r: MockRemote) => ({
  enabled: r.enabled, port: 57667, password_set: r.password_set, listening: r.enabled, error: null,
  urls: r.enabled ? [REMOTE_URL] : [], fingerprint: r.enabled ? REMOTE_FINGERPRINT : null, sessions: r.enabled ? r.sessions : [],
});

/** Remote access with the server's rules: admin routes are local only, enabling needs a password. */
async function handleRemote(route: Route, api: ApiMock, segs: string[]) {
  const req = route.request();
  const method = req.method();
  const r = api.remote;
  const path = '/' + segs.join('/');
  const body = (() => { try { return JSON.parse(req.postData() ?? 'null'); } catch { return null; } })();
  const signIn = () => route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><title>Sign in</title><h1>Sign in to BenchMesh</h1>' });
  if (path === '/remote/session') return json(route, r.session);
  if (path === '/remote/login') return signIn();
  if (!(method === 'GET' && path === '/remote')) r.requests.push({ method, path, body: req.postData() });
  // the server answers 303 to /remote/login; Playwright does not intercept the redirected request,
  // so the mock serves the page the redirect leads to
  if (path === '/remote/logout' && method === 'POST') return signIn();
  if (r.session.principal !== 'local') return json(route, { detail: 'Remote access settings can be changed only from the BenchMesh app on this computer' }, 403);
  if (path === '/remote' && method === 'GET') return json(route, remoteStatus(r));
  if (path === '/remote' && method === 'PUT') {
    if (typeof body?.enabled !== 'boolean') return json(route, { detail: 'enabled must be a boolean' }, 422);
    if (body.enabled && !r.password_set) return json(route, { detail: 'Set a password first' }, 409);
    r.enabled = body.enabled;
    if (!r.enabled) r.sessions = [];
    return json(route, remoteStatus(r));
  }
  if (path === '/remote/password' && method === 'PUT') {
    if (typeof body?.password !== 'string' || body.password.length < 10) return json(route, { detail: 'The password needs at least 10 characters' }, 400);
    r.password_set = true;
    r.sessions = [];
    return route.fulfill({ status: 204 });
  }
  if (segs[1] === 'sessions' && method === 'DELETE') {
    r.sessions = segs.length === 3 ? r.sessions.filter((x) => x.id !== segs[2]) : [];
    return route.fulfill({ status: 204 });
  }
  return json(route, { detail: 'Not found' }, 404);
}

/** Install the API mock on `page` and return its handle. */
export async function setupApiMocks(page: Target): Promise<ApiMock> {
  const api: ApiMock = {
    instruments: structuredClone(mockInstruments),
    config: structuredClone(mockConfig),
    posts: [],
    deviceRequests: [],
    failures: new Map(),
    recordings: [],
    recordingRequests: [],
    samples: new Map(),
    remote: { session: { principal: 'local' }, enabled: false, password_set: false, sessions: [], requests: [] },
    scpi: { access: 'off', base_port: 5025 },
  };

  await page.route(
    (url) => API_PREFIXES.some((p) => url.pathname === p || url.pathname.startsWith(p + '/')),
    async (route) => {
      const req = route.request();
      const url = new URL(req.url());
      const segs = url.pathname.split('/').filter(Boolean).map(decodeURIComponent);
      const method = req.method();

      switch (segs[0]) {
        case 'instruments':
          return handleInstruments(route, api, segs);
        case 'config':
          return json(route, api.config);
        case 'drivers':
          if (segs.length === 1) return json(route, mockDrivers);
          return json(route, mockModels[segs[1]] ?? []);
        case 'serial-ports':
          return json(route, mockPorts);
        case 'usbtmc-devices':
          return json(route, []);
        case 'devices': {
          api.deviceRequests.push({ method, path: url.pathname, body: req.postData() });
          const fail = api.failures.get(url.pathname);
          if (fail) return json(route, { detail: fail.detail }, fail.status);
          // park / unpark: the WebSocket registry is the test's to update (`__pushRegistry`)
          if (segs[2] === 'park') return json(route, { id: segs[1], parked: method === 'POST' });
          return json(route, { status: 'ok' });
        }
        case 'recordings':
          return handleRecordings(route, api, segs, url);
        case 'remote':
          return handleRemote(route, api, segs);
        case 'scpi':
          return handleScpi(route, api);
        case 'status':
          return json(route, { devices_total: api.instruments.length, connected: 2, disconnected: 1 });
        case 'openapi.json':
          return json(route, mockOpenApi);
        case 'version':
          return json(route, { releaseVersion: '0.1.7', version: '0.1.7', displayVersion: 'BenchMesh 0.1.7', name: 'BenchMesh' });
        default:
          return json(route, { detail: 'Not mocked' }, 404);
      }
    },
  );

  return api;
}

/**
 * Replace `window.WebSocket` with a mock: `/ws/registry` sends `registry` on open and then
 * every 400 ms (the UI flags the stream stale after 1 s without data); `/ws/metrics` sends
 * one metrics snapshot; `/ws/recordings` sends a snapshot of the mocked `GET /recordings`. `window.__pushRegistry(data)` replaces (and sends) the registry;
 * `window.__registryPaused = true` stops the periodic sends; `window.__pushRecordings(msg)`
 * sends a recordings message (`samples`, `recording`, `deleted`, `snapshot`).
 */
export async function setupWebSocketMock(page: Target, registry: unknown = mockRegistry) {
  await page.addInitScript((registryData) => {
    const w = window as any;
    w.__registry = registryData;
    const sockets: any[] = [];
    const recSockets: any[] = [];
    w.__pushRegistry = (data: unknown) => {
      w.__registry = data;
      for (const s of sockets) s.emit(data);
    };
    w.__pushRecordings = (msg: unknown) => { for (const s of recSockets) s.emit(msg); };

    class MockWebSocket {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;
      url: string;
      readyState = 0;
      onopen: ((event: Event) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent) => void) | null = null;
      private timer: ReturnType<typeof setInterval> | undefined;

      constructor(url: string) {
        this.url = url;
        setTimeout(() => {
          if (this.readyState !== 0) return;
          this.readyState = 1;
          this.onopen?.(new Event('open'));
          if (this.url.includes('/ws/registry')) {
            sockets.push(this);
            this.emit(w.__registry);
            this.timer = setInterval(() => { if (!w.__registryPaused) this.emit(w.__registry); }, 400);
          } else if (this.url.includes('/ws/recordings')) {
            recSockets.push(this);
            // the snapshot is what the (mocked) REST API holds
            fetch('/recordings').then((r) => r.json()).then((recordings) => this.emit({ type: 'snapshot', recordings }))
              .catch(() => this.emit({ type: 'snapshot', recordings: [] }));
          } else if (this.url.includes('/ws/metrics')) {
            this.emit({
              'test-psu-1': {
                device_id: 'test-psu-1', window_duration_s: 30.0, utilization_pct: 12.5, qps: 2.5,
                api_request_count: 5, api_latency_p95_ms: 11.2, api_latency_p99_ms: 14.5,
                avg_queue_depth: 0.42, avg_poll_duration_ms: 120.5, total_operations: 75,
              },
            });
          }
        }, 50);
      }

      emit(data: unknown) {
        if (this.readyState === 1) this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(data) }));
      }

      close() {
        if (this.readyState === 3) return;
        this.readyState = 3;
        clearInterval(this.timer);
        for (const list of [sockets, recSockets]) {
          const i = list.indexOf(this);
          if (i >= 0) list.splice(i, 1);
        }
        this.onclose?.(new CloseEvent('close'));
      }

      send() { /* the UI never sends */ }
      addEventListener() { /* unused by the UI */ }
      removeEventListener() { /* unused by the UI */ }
    }

    w.WebSocket = MockWebSocket;
  }, registry);
}

/** Skip the first-run safety disclaimer. */
export async function acceptDisclaimer(page: Target) {
  await page.addInitScript(() => {
    try { localStorage.setItem('benchmesh:disclaimer-accepted', 'true'); } catch { /* ignore */ }
  });
}
