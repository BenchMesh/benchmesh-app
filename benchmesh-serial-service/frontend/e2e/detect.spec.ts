/**
 * Detect (Instruments drawer): `POST /devices/detect` lists the USB ports with what answered on
 * them; rows offer Add (new entry identified by `match`) and Lock (configured entry gets `match`).
 * Contract: ai_context/migration/IDENTITY.md. Values are from the real bench.
 */
import { Page } from '@playwright/test';
import { test, expect, gotoBench } from './fixtures/base';
import type { ApiMock } from './utils/api-mock';

const XDM_MATCH = { usb: { vid: '1a86', pid: '7523', revision: '0264', location: '1-1.3.2.2.4' }, idn_serial: '24512010' };
const SPM_MATCH = { usb: { vid: '1a86', pid: '7523', revision: '8134', location: '1-1.3.2.4' }, idn_serial: '24460763' };
const TENMA_MATCH = { usb: { vid: '0416', pid: '5011', serial: 'A02014090305', location: '1-1.3.1' } };

const usb = (o: Record<string, string | null>) => ({
  vid: null, pid: null, serial: null, revision: null, manufacturer: null, product: null, location: null, ...o,
});

const DETECT = {
  ports: [
    {
      port: '/dev/ttyUSB2', kind: 'serial',
      usb: usb({ vid: '1a86', pid: '7523', revision: '0264', product: 'USB Serial', location: '1-1.3.2.2.4' }),
      configured_as: 'owon-xdm1241', locked: false,
      instrument: { driver: 'owon_xdm', model: 'XDM1241', vendor: 'OWON', family: 'XDM', idn: 'OWON,XDM1241,24512010,V4.3.0,3', classes: ['DMM'] },
      match: XDM_MATCH, certainty: 'instrument', error: null,
    },
    {
      port: '/dev/ttyUSB0', kind: 'serial',
      usb: usb({ vid: '1a86', pid: '7523', revision: '8134', product: 'USB Serial', location: '1-1.3.2.4' }),
      configured_as: null, locked: false,
      instrument: { driver: 'owon_spm', model: 'SPM3103', vendor: 'OWON', family: 'SPM', idn: 'OWON,SPM3103,24460763,FV:V2.1.0', classes: ['PSU', 'DMM'] },
      match: SPM_MATCH, certainty: 'instrument', error: null,
    },
    {
      port: '/dev/ttyACM0', kind: 'serial',
      usb: usb({ vid: '0416', pid: '5011', serial: 'A02014090305', manufacturer: 'Nuvoton', product: 'USB Virtual COM', location: '1-1.3.1' }),
      configured_as: null, locked: false,
      instrument: { driver: 'tenma_72', model: '72-2540', vendor: 'TENMA', family: '72', idn: 'TENMA 72-2540 V2.1', classes: ['PSU'] },
      match: TENMA_MATCH, certainty: 'adapter', error: null,
    },
    {
      port: '/dev/ttyUSB1', kind: 'serial',
      usb: usb({ vid: '1a86', pid: '7523', revision: '0254', product: 'USB Serial', location: '1-1.4' }),
      configured_as: null, locked: false, instrument: null, match: null, certainty: null,
      error: 'no definition answered',
    },
  ],
};

const XDM_ENTRY = { id: 'owon-xdm1241', name: 'OWON DMM', driver: 'owon_xdm', model: 'XDM1241', transport: 'serial', port: '/dev/ttyUSB2', baud: 115200, serial: '8N1' };

/** Bench with the XDM configured by port; Detect answered with `DETECT` after `delayMs`. */
async function openWithDetect(page: Page, api: ApiMock, detect: unknown = DETECT, delayMs = 0) {
  api.config = { version: 1, devices: [{ ...XDM_ENTRY }] };
  await page.route('**/devices/detect', async (route) => {
    if (delayMs) await new Promise((r) => setTimeout(r, delayMs));
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(detect) });
  });
  await gotoBench(page, api.instruments.length);
  await page.getByRole('button', { name: 'Instruments', exact: true }).click();
  const drawer = page.getByRole('dialog', { name: 'Instruments' });
  await expect(drawer.locator('.accordion-item')).toHaveCount(1);
  return drawer;
}

const row = (drawer: ReturnType<Page['getByRole']>, port: string) =>
  drawer.getByTestId('detect-row').filter({ hasText: port });

test.describe('Detect instruments', () => {
  test('lists every port with instrument, identity and certainty; busy while running', async ({ page, api }) => {
    const drawer = await openWithDetect(page, api, DETECT, 600);
    await expect(drawer.getByText('Sends only an identify query to USB ports no instrument is using.')).toBeVisible();
    const btn = drawer.getByTestId('detect-btn');
    await btn.click();
    await expect(btn).toBeDisabled();
    await expect(btn).toHaveText('Detecting…');

    await expect(drawer.getByTestId('detect-row')).toHaveCount(4);
    await expect(btn).toBeEnabled();
    await expect(btn).toHaveText('Detect instruments');

    const xdm = row(drawer, '/dev/ttyUSB2');
    await expect(xdm).toContainText('OWON XDM1241');
    await expect(xdm).toContainText('serial 24512010');
    await expect(xdm).toContainText('identified by serial number');
    await expect(xdm.getByRole('button', { name: 'Lock to this instrument' })).toHaveAttribute('title', /any USB port/);

    const spm = row(drawer, '/dev/ttyUSB0');
    await expect(spm).toContainText('OWON SPM3103');
    await expect(spm).toContainText('serial 24460763');
    await expect(spm.getByRole('button', { name: 'Add' })).toBeVisible();

    const tenma = row(drawer, '/dev/ttyACM0');
    await expect(tenma).toContainText('TENMA 72-2540');
    await expect(tenma).toContainText('USB serial A02014090305');
    await expect(tenma).toContainText('identified by USB serial');

    const unknown = row(drawer, '/dev/ttyUSB1');
    await expect(unknown).toContainText('Unknown device');
    await expect(unknown).toContainText('USB Serial');
    await expect(unknown).toContainText('USB 1a86:7523 rev 0254');
    await expect(unknown).toContainText('no definition answered');
    await expect(unknown.getByRole('button')).toHaveCount(0);
  });

  test('Add posts a new entry identified by match and marks the row configured', async ({ page, api }) => {
    const drawer = await openWithDetect(page, api);
    await drawer.getByTestId('detect-btn').click();
    const spm = row(drawer, '/dev/ttyUSB0');
    await spm.getByRole('button', { name: 'Add' }).click();

    await expect(spm).toContainText('✓ configured as owon-spm3103');
    const posts = api.deviceRequests.filter((r) => r.method === 'POST' && r.path === '/devices');
    expect(posts).toHaveLength(1);
    expect(JSON.parse(posts[0].body!)).toEqual({
      id: 'owon-spm3103', name: 'OWON SPM3103', driver: 'owon_spm', model: 'SPM3103', transport: 'serial',
      match: SPM_MATCH, port: '/dev/ttyUSB0',
    });
  });

  test('Add makes the id unique against the configuration', async ({ page, api }) => {
    const detect = { ports: [{ ...DETECT.ports[0], configured_as: null }] };
    const drawer = await openWithDetect(page, api, detect);
    await drawer.getByTestId('detect-btn').click();
    await row(drawer, '/dev/ttyUSB2').getByRole('button', { name: 'Add' }).click();
    await expect(row(drawer, '/dev/ttyUSB2')).toContainText('✓ configured as owon-xdm1241-2');
    const post = api.deviceRequests.find((r) => r.method === 'POST');
    expect(JSON.parse(post!.body!)).toMatchObject({ id: 'owon-xdm1241-2', match: XDM_MATCH });
  });

  test('Lock puts the existing entry with match', async ({ page, api }) => {
    const drawer = await openWithDetect(page, api);
    await drawer.getByTestId('detect-btn').click();
    const xdm = row(drawer, '/dev/ttyUSB2');
    await xdm.getByRole('button', { name: 'Lock to this instrument' }).click();

    await expect(xdm).toContainText('✓ configured as owon-xdm1241');
    const puts = api.deviceRequests.filter((r) => r.method === 'PUT');
    expect(puts).toHaveLength(1);
    expect(puts[0].path).toBe('/devices/owon-xdm1241');
    expect(JSON.parse(puts[0].body!)).toEqual({ ...XDM_ENTRY, match: XDM_MATCH });
  });

  test('shows the API error on the row', async ({ page, api }) => {
    const drawer = await openWithDetect(page, api);
    await page.route('**/devices', (route) => route.fulfill({
      status: 409, contentType: 'application/json', body: JSON.stringify({ detail: "device id 'owon-spm3103' exists" }),
    }));
    await drawer.getByTestId('detect-btn').click();
    const spm = row(drawer, '/dev/ttyUSB0');
    await spm.getByRole('button', { name: 'Add' }).click();
    await expect(spm.locator('.instrument-config__inline-error')).toHaveText("device id 'owon-spm3103' exists");
    await expect(spm.getByRole('button', { name: 'Add' })).toBeEnabled();
  });

  test('a port told apart only by its USB position carries a warning', async ({ page, api }) => {
    const detect = { ports: [{ ...DETECT.ports[1], certainty: 'position' }] };
    const drawer = await openWithDetect(page, api, detect);
    await drawer.getByTestId('detect-btn').click();
    const cert = row(drawer, '/dev/ttyUSB0').locator('.detect__cert');
    await expect(cert).toHaveText(/USB port position only — moving it to another port loses it/);
    await expect(cert).toHaveClass(/detect__cert--warn/);
  });
});

test.describe('Configured entry with an identity', () => {
  test('shows the identity instead of the port picker; saving keeps match; can go back to a fixed port', async ({ page, api }) => {
    await openWithDetect(page, api);
    api.config = { version: 1, devices: [{ ...XDM_ENTRY, match: XDM_MATCH, extra: 'kept' } as any] };
    // reload the drawer's config with the locked entry
    await page.reload();
    await expect(page.getByRole('article')).toHaveCount(api.instruments.length);
    await page.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = page.getByRole('dialog', { name: 'Instruments' });
    await drawer.locator('.accordion-header').filter({ hasText: 'owon-xdm1241' }).click();
    const form = drawer.getByRole('region');

    const identity = form.getByTestId('identity-field');
    await expect(identity).toContainText('serial 24512010');
    await expect(identity).toContainText('last seen on /dev/ttyUSB2');
    await expect(form.locator('option[value="__custom__"]')).toHaveCount(0); // no port picker

    await form.getByPlaceholder('My Instrument').fill('Bench DMM');
    await form.getByRole('button', { name: 'Save & Apply' }).click();
    await expect(form.getByText('✓ Applied successfully!')).toBeVisible();
    const put = api.deviceRequests.find((r) => r.method === 'PUT');
    expect(JSON.parse(put!.body!)).toEqual({ ...XDM_ENTRY, name: 'Bench DMM', match: XDM_MATCH, extra: 'kept' });

    await identity.getByRole('button', { name: 'Use a fixed port instead' }).click();
    await expect(form.getByTestId('identity-field')).toHaveCount(0);
    await expect(form.locator('option[value="__custom__"]')).toHaveCount(1);
  });
});
