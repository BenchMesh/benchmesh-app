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

/** Three instruments: a PSU and a DMM that are connected, and an LCR meter (no dedicated panel) that is offline. */
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

const API_PREFIXES = [
  '/instruments', '/drivers', '/config', '/devices', '/serial-ports', '/usbtmc-devices',
  '/status', '/version', '/metrics', '/recordings', '/openapi.json', '/openapi.yaml', '/ai',
];

export interface RecordedRequest { method: string; path: string; body?: string | null }

export interface ApiMock {
  /** Instrument list served by `GET /instruments` (mutable; takes effect on the next poll). */
  instruments: MockInstrument[];
  config: typeof mockConfig;
  /** Every `POST /instruments/...` operation, in order (path is URL-decoded). */
  posts: RecordedRequest[];
  /** Every `/devices` request (add / update / remove instrument). */
  deviceRequests: RecordedRequest[];
  /** Instrument operation paths (as recorded in `posts`) that should fail with the given reply. */
  failures: Map<string, { status: number; detail: string }>;
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

/** Install the API mock on `page` and return its handle. */
export async function setupApiMocks(page: Target): Promise<ApiMock> {
  const api: ApiMock = {
    instruments: structuredClone(mockInstruments),
    config: structuredClone(mockConfig),
    posts: [],
    deviceRequests: [],
    failures: new Map(),
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
        case 'devices':
          api.deviceRequests.push({ method, path: url.pathname, body: req.postData() });
          return json(route, { status: 'ok' });
        case 'recordings':
          return json(route, method === 'GET' ? [] : { status: 'ok' });
        case 'status':
          return json(route, { devices_total: api.instruments.length, connected: 2, disconnected: 1 });
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
 * one metrics snapshot. `window.__pushRegistry(data)` replaces (and sends) the registry;
 * `window.__registryPaused = true` stops the periodic sends.
 */
export async function setupWebSocketMock(page: Target, registry: unknown = mockRegistry) {
  await page.addInitScript((registryData) => {
    const w = window as any;
    w.__registry = registryData;
    const sockets: any[] = [];
    w.__pushRegistry = (data: unknown) => {
      w.__registry = data;
      for (const s of sockets) s.emit(data);
    };

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
        const i = sockets.indexOf(this);
        if (i >= 0) sockets.splice(i, 1);
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
