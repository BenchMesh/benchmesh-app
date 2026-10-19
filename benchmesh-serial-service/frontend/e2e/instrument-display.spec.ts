import { test, expect } from './fixtures/base';
import { PANELS } from './utils/api-mock';

test.describe('Instrument panels', () => {
  test('shows one panel per instrument class, named "<class>: <instrument>"', async ({ mockPage }) => {
    const panels = mockPage.getByRole('article');
    await expect(panels).toHaveCount(3);
    for (const name of Object.values(PANELS)) {
      await expect(mockPage.getByRole('article', { name })).toBeVisible();
    }
  });

  test('panel headers show class title, name and model from the IDN', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await expect(psu.getByRole('heading', { name: 'Power Supply Unit' })).toBeVisible();
    await expect(psu.locator('.bench-panel__name')).toHaveText('Test PSU');
    await expect(psu.locator('.bench-panel__model')).toHaveText('TENMA 72-2540');

    const dmm = mockPage.getByRole('article', { name: PANELS.dmm });
    await expect(dmm.getByRole('heading', { name: 'Digital Multimeter' })).toBeVisible();
    await expect(dmm.locator('.bench-panel__model')).toHaveText('OWON XDM1041');
  });

  test('connected instruments show "connected", the one without IDN shows "offline"', async ({ mockPage }) => {
    await expect(mockPage.getByRole('article', { name: PANELS.psu }).locator('.bench-panel__link')).toHaveText('connected');
    await expect(mockPage.getByRole('article', { name: PANELS.dmm }).locator('.bench-panel__link')).toHaveText('connected');
    const lcr = mockPage.getByRole('article', { name: PANELS.lcr });
    await expect(lcr.locator('.bench-panel__link')).toHaveText('offline');
    await expect(lcr.getByText('Offline - reconnecting')).toBeVisible();
  });

  test('an instrument goes offline when it disappears from the registry', async ({ mockPage }) => {
    const psuLink = mockPage.getByRole('article', { name: PANELS.psu }).locator('.bench-panel__link');
    await expect(psuLink).toHaveText('connected');
    await mockPage.evaluate(() => {
      const w = window as any;
      const { ['test-psu-1']: _gone, ...rest } = w.__registry;
      w.__pushRegistry(rest);
    });
    await expect(psuLink).toHaveText('offline');
    await expect(mockPage.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveAttribute('aria-disabled', 'true');
  });

  test('PSU panel shows live readouts, setpoints from the instrument and annunciators', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await expect(psu.getByRole('img', { name: 'Output voltage 04.93' })).toBeVisible();
    await expect(psu.getByRole('img', { name: 'Output current 0.250' })).toBeVisible();
    await expect(psu.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveAttribute('aria-valuenow', '5');
    await expect(psu.getByRole('spinbutton', { name: 'Current limit' })).toHaveAttribute('aria-valuenow', '1');
    await expect(psu.getByLabel('CV on')).toBeVisible();
    await expect(psu.getByLabel('CC off')).toBeVisible();
    await expect(psu.getByLabel('OUT off')).toBeVisible();
    await expect(psu.getByRole('button', { name: /^Output/ })).toHaveAttribute('data-state', 'off');
  });

  test('readouts follow registry updates', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await mockPage.evaluate(() => {
      const w = window as any;
      const r = structuredClone(w.__registry);
      r['test-psu-1'].PSU.status_ch1.VOUT = { val: 12.34, sci: '1.2340e+1', si: { number: 12.34 } };
      r['test-psu-1'].PSU.status_ch1.OUTPUT = 'ON';
      w.__pushRegistry(r);
    });
    await expect(psu.getByRole('img', { name: 'Output voltage 12.34' })).toBeVisible();
    await expect(psu.getByLabel('OUT on')).toBeVisible();
    await expect(psu.getByRole('button', { name: /^Output/ })).toHaveAttribute('data-state', 'on');
  });

  test('DMM panel shows the function dial, reading and range chips from the features', async ({ mockPage }) => {
    const dmm = mockPage.getByRole('article', { name: PANELS.dmm });
    await expect(dmm.getByRole('slider', { name: 'Function' })).toHaveAttribute('aria-valuetext', 'DC Voltage');
    await expect(dmm.getByRole('img', { name: /^DC Voltage 4\.98/ })).toBeVisible();
    const ranges = dmm.getByRole('group', { name: 'Range' });
    // MIN/MAX are not offered as ranges
    await expect(ranges.getByRole('button')).toHaveText(['Auto', '500 mV', '5 V', '50 V']);
    await expect(ranges.getByRole('button', { name: 'Auto' })).toHaveAttribute('aria-pressed', 'true');
    await expect(dmm.getByLabel('AUTO on')).toBeVisible();
  });

  test('a class without a dedicated panel falls back to the generic panel', async ({ mockPage }) => {
    await expect(mockPage.getByRole('article', { name: PANELS.lcr }).getByText('No status reported yet.')).toBeVisible();
  });

  test('shows the empty state when no instruments are configured', async ({ page, api }) => {
    api.instruments = [];
    await page.goto('/ui/');
    await expect(page.getByRole('heading', { name: 'No instruments yet' })).toBeVisible();
    await expect(page.getByRole('article')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Arrange' })).toBeDisabled();
    await expect(page.locator('.bench-pill').first()).toHaveText('API');
  });

  test('a newly configured instrument appears on the next poll', async ({ mockPage, api }) => {
    api.instruments.push({ id: 'test-load-1', name: 'Test Load', IDN: 'OWON,OEL1515,SN1,V1', classes: [{ class: 'ELL', channels: ['1'] }] });
    // the instrument list is polled every 5 s
    await expect(mockPage.getByRole('article', { name: 'Electronic Load: Test Load' })).toBeVisible({ timeout: 10_000 });
    await expect(mockPage.getByRole('article')).toHaveCount(4);
  });
});
