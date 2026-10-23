/**
 * Instruments → System (ai_context/HEADLESS.md): the computer's resources for everyone; for a
 * headless BenchMesh used from a remote browser Restart, Shut down (two presses) and Start with
 * this computer; a view-only session only looks; the desktop app explains how to run headless.
 */
import { test, expect } from './fixtures/base';
import type { Page } from '@playwright/test';

const openSystem = async (page: Page) => {
  await page.getByRole('button', { name: 'Instruments', exact: true }).click();
  const drawer = page.getByRole('dialog', { name: 'Instruments' });
  await drawer.getByTestId('settings-tab-system').click();
  return drawer.getByRole('region', { name: 'System' });
};

test.describe('System, headless from a remote browser', () => {
  test.beforeEach(async ({ api }) => {
    api.system.headless = true;
    api.remote.session = { principal: 'remote', role: 'control', address: '192.168.1.30', since: Date.now() };
  });

  test('shows the computer and starts BenchMesh with it', async ({ mockPage, api }) => {
    const section = await openSystem(mockPage);
    const computer = section.getByRole('group', { name: 'This computer', exact: true });
    await expect(computer.getByText('benchpi')).toBeVisible();
    await expect(computer.getByText('aarch64, 4 cores')).toBeVisible();
    await expect(computer.getByText('61.2 °C')).toBeVisible();
    await expect(section.getByRole('meter', { name: 'Memory' })).toHaveAttribute('data-level', 'warn');
    await expect(section.getByRole('meter', { name: 'Disk' })).toContainText('21 GB free of 29 GB');

    const sw = section.getByRole('switch', { name: 'Start with this computer' });
    await expect(sw).toHaveAttribute('aria-checked', 'false');
    await sw.click();
    await expect(sw).toHaveAttribute('aria-checked', 'true');
    await expect(section.getByText('sudo loginctl enable-linger "$USER"')).toBeVisible();
    expect(api.system.requests).toEqual([{ method: 'PUT', path: '/system/autostart', body: '{"enabled":true}' }]);
  });

  test('restarts only on the second press and waits for BenchMesh', async ({ mockPage, api }) => {
    const section = await openSystem(mockPage);
    await section.getByRole('button', { name: 'Restart' }).click();
    expect(api.system.requests).toEqual([]);
    await section.getByRole('button', { name: 'Press again: restart' }).click();
    await expect(section.getByText('Restarting…')).toBeVisible();
    expect(api.system.requests.map((r) => `${r.method} ${r.path}`)).toEqual(['POST /system/restart']);
  });


  test('shuts down on the second press', async ({ mockPage, api }) => {
    const section = await openSystem(mockPage);
    await section.getByRole('button', { name: 'Shut down' }).click();
    await section.getByRole('button', { name: 'Press again: shut down' }).click();
    await expect(section.getByText('BenchMesh is stopped')).toBeVisible();
    expect(api.system.requests.map((r) => `${r.method} ${r.path}`)).toEqual(['POST /system/shutdown']);
  });
});

test.describe('System, view only', () => {
  test('looks but cannot restart or change the start', async ({ api, page }) => {
    api.system.headless = true;
    api.remote.session = { principal: 'remote', role: 'view', address: '192.168.1.31', since: Date.now() };
    await page.goto('/ui/');
    const section = await openSystem(page);
    await expect(section.getByRole('group', { name: 'This computer', exact: true })).toBeVisible();
    await expect(section.getByRole('button', { name: 'Restart' })).toBeDisabled();
    await expect(section.getByRole('button', { name: 'Shut down' })).toBeDisabled();
    await expect(section.getByRole('switch', { name: 'Start with this computer' })).toBeDisabled();
  });
});

test.describe('System, desktop app', () => {
  test('resources only, and how to run without a window', async ({ mockPage }) => {
    const section = await openSystem(mockPage);
    await expect(section.getByRole('group', { name: 'Resources' })).toBeVisible();
    await expect(section.getByRole('button', { name: 'Restart' })).toHaveCount(0);
    await expect(section.getByRole('switch')).toHaveCount(0);
    await expect(section.getByRole('link', { name: 'Headless BenchMesh' })).toHaveAttribute('href', 'https://benchmesh.online/docs/headless');
  });
});
