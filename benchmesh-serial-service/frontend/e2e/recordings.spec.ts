/**
 * Recordings v2: the recorder dock, recordable readings (context menu on a reading), the
 * recordings drawer. The API and `/ws/recordings` are mocked (utils/api-mock.ts).
 */
import { test, expect } from './fixtures/base';
import { PANELS } from './utils/api-mock';
import type { Page } from '@playwright/test';

async function startRecording(page: Page, name: string, interval = '1') {
  const dock = page.getByRole('region', { name: 'Recorder' });
  await dock.getByRole('button', { name: '+ New recording' }).click();
  const form = dock.getByRole('form', { name: 'New recording' });
  await form.getByLabel('Name').fill(name);
  await form.getByLabel('Interval (s)').fill(interval);
  await form.getByRole('button', { name: 'Start recording' }).click();
  await expect(form).toHaveCount(0);
  return dock;
}

test.describe('Recorder', () => {
  test('records a reading from the dock to the drawer', async ({ mockPage, api }) => {
    const dock = await startRecording(mockPage, 'Bench run');
    // the dock opens on the new recording
    await expect(dock.getByRole('heading', { name: 'Bench run' })).toBeVisible();
    await expect(dock.getByRole('list', { name: 'Active recordings' })).toContainText('Bench run');
    expect(api.recordingRequests[0]).toMatchObject({ method: 'POST', path: '/recordings' });
    expect(JSON.parse(api.recordingRequests[0].body!)).toEqual({ name: 'Bench run', interval_s: 1 });

    // right-click the PSU output voltage: the menu offers the recording
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    const vout = psu.getByRole('group', { name: 'Test PSU · VOUT' });
    await vout.click({ button: 'right' });
    const menu = mockPage.getByRole('menu', { name: 'Record Test PSU · VOUT' });
    const item = menu.getByRole('menuitemcheckbox', { name: 'Bench run' });
    await expect(item).toHaveAttribute('aria-checked', 'false');
    await item.click();
    await expect(item).toHaveAttribute('aria-checked', 'true');
    await mockPage.keyboard.press('Escape');
    await expect(menu).toHaveCount(0);
    await expect(vout.getByRole('img', { name: 'Recorded in Bench run' })).toBeVisible();
    const add = api.recordingRequests.find((r) => r.path === '/recordings/1/series')!;
    expect(JSON.parse(add.body!)).toEqual({ device_id: 'test-psu-1', class: 'PSU', channel: 1, field: 'VOUT', label: 'Test PSU · VOUT', unit: 'V' });

    // live samples reach the dock's legend and chart
    await expect(dock.getByRole('list', { name: 'Series' })).toContainText('Test PSU · VOUT');
    await mockPage.evaluate(() => {
      const t = Math.floor(Date.now() / 1000) * 1000;
      (window as any).__pushRecordings({ type: 'samples', recording_id: 1, t, values: { 1: 4.93 } });
      (window as any).__pushRecordings({ type: 'samples', recording_id: 1, t: t + 1000, values: { 1: 4.95 } });
    });
    await expect(dock.getByRole('button', { name: /Test PSU · VOUT.*4\.95 V/ })).toBeVisible();
    await expect(dock.getByRole('img', { name: 'Live chart of Bench run' })).toBeVisible();
    await expect(dock.getByRole('list', { name: 'Active recordings' })).toContainText('1 ser.');

    // stop needs a second, explicit step
    await dock.getByRole('button', { name: 'Stop', exact: true }).click();
    await expect(dock.getByRole('group', { name: /Stop for good/ })).toBeVisible();
    expect(api.recordingRequests.some((r) => r.path.endsWith('/stop'))).toBe(false);
    await dock.getByRole('button', { name: 'Stop recording' }).click();
    await expect(dock.getByText('No active recordings')).toBeVisible();
    expect(api.recordingRequests.some((r) => r.method === 'POST' && r.path === '/recordings/1/stop')).toBe(true);
    // a stopped recording no longer marks the reading
    await expect(vout.getByRole('img', { name: /Recorded/ })).toHaveCount(0);

    // the drawer has it in the history; open it
    await mockPage.getByRole('button', { name: 'Recording', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Recordings' });
    await drawer.getByRole('button', { name: 'Open Bench run' }).click();
    await expect(drawer.getByRole('heading', { name: 'Bench run' })).toBeVisible();
    await expect(drawer.getByText('Stopped', { exact: true })).toBeVisible();
    await expect(drawer.getByRole('link', { name: 'Export CSV' })).toHaveAttribute('href', /\/recordings\/1\/export$/);
    await expect(drawer.getByRole('img', { name: 'Chart of Bench run' })).toBeVisible();
    await expect(drawer.getByRole('list', { name: 'Series' })).toContainText('Test PSU · VOUT');
  });

  test('the new-recording form checks its fields and shows server errors', async ({ mockPage, api }) => {
    api.recordings.push({
      id: 1, name: 'Taken', description: '', interval_s: 1, state: 'stopped', started_at: Date.now() - 60_000,
      stopped_at: Date.now(), paused_at: null, samples: 0, last_t: null, series: [],
    });
    const dock = mockPage.getByRole('region', { name: 'Recorder' });
    await dock.getByRole('button', { name: '+ New recording' }).click();
    const form = dock.getByRole('form', { name: 'New recording' });
    await form.getByRole('button', { name: 'Start recording' }).click();
    await expect(form.getByText('Name is required')).toBeVisible();
    await form.getByLabel('Name').fill('Taken');
    await form.getByLabel('Interval (s)').fill('0.5');
    await expect(form.getByText('Whole seconds, 1 or more')).toBeVisible();
    await form.getByLabel('Interval (s)').fill('2');
    await form.getByRole('button', { name: 'Start recording' }).click();
    await expect(form.getByRole('alert')).toHaveText("Recording name 'Taken' is taken");
    await form.getByRole('button', { name: 'Cancel' }).click();
    await expect(form).toHaveCount(0);
  });

  test('a reading opens its menu from the keyboard and can start a new recording', async ({ mockPage }) => {
    const dmm = mockPage.getByRole('article', { name: PANELS.dmm });
    const meas = dmm.getByRole('group', { name: 'Test DMM · MEAS' });
    await meas.focus();
    await mockPage.keyboard.press('Shift+F10');
    const menu = mockPage.getByRole('menu', { name: 'Record Test DMM · MEAS' });
    await expect(menu.getByRole('menuitem', { name: 'New recording…' })).toBeFocused();
    await mockPage.keyboard.press('Enter');
    const dialog = mockPage.getByRole('dialog', { name: 'New recording with Test DMM · MEAS' });
    await dialog.getByLabel('Name').fill('DMM drift');
    await dialog.getByRole('button', { name: 'Start recording' }).click();
    await expect(dialog).toHaveCount(0);
    await expect(meas.getByRole('img', { name: 'Recorded in DMM drift' })).toBeVisible();
  });

  test('the dock remembers being open, never covers the panels and fits a narrow window', async ({ mockPage }) => {
    await mockPage.setViewportSize({ width: 400, height: 700 });
    const dock = await startRecording(mockPage, 'Narrow');
    await expect(dock.getByRole('heading', { name: 'Narrow' })).toBeVisible();
    expect(await mockPage.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(400);

    // scrolled to the end, the last panel ends above the dock
    await mockPage.evaluate(() => window.scrollTo(0, document.scrollingElement!.scrollHeight));
    const last = mockPage.getByRole('article').last();
    const panelBottom = await last.evaluate((e) => e.getBoundingClientRect().bottom);
    const dockTop = await dock.evaluate((e) => e.getBoundingClientRect().top);
    expect(panelBottom).toBeLessThanOrEqual(dockTop);

    await mockPage.reload();
    await expect(mockPage.getByRole('region', { name: 'Recorder' }).getByRole('button', { name: 'Collapse recorder' })).toBeVisible();
    await mockPage.getByRole('button', { name: 'Collapse recorder' }).click();
    await expect(mockPage.getByRole('region', { name: 'Recorder' }).getByRole('heading', { name: 'Narrow' })).toHaveCount(0);
  });

  test('readings are recordable in a detached panel window', async ({ mockPage }) => {
    await startRecording(mockPage, 'Popup run');
    const [win] = await Promise.all([
      mockPage.waitForEvent('popup'),
      mockPage.getByRole('article', { name: PANELS.psu }).getByRole('button', { name: 'Open in own window' }).click(),
    ]);
    const iout = win.getByRole('group', { name: 'Test PSU · IOUT' });
    await iout.click({ button: 'right' });
    await win.getByRole('menuitemcheckbox', { name: 'Popup run' }).click();
    await expect(win.getByRole('menuitemcheckbox', { name: 'Popup run' })).toHaveAttribute('aria-checked', 'true');
    await expect(iout.getByRole('img', { name: 'Recorded in Popup run' })).toBeVisible();
    // no recorder dock in a panel window
    await expect(win.getByRole('region', { name: 'Recorder' })).toHaveCount(0);
  });
});
