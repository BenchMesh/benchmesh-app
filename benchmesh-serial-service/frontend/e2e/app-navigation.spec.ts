import { test, expect } from './fixtures/base';
import { setupApiMocks, setupWebSocketMock } from './utils/api-mock';

test.describe('Top bar', () => {
  test('shows the brand and healthy API / live status pills', async ({ mockPage }) => {
    const top = mockPage.locator('header.bench-top');
    await expect(top.getByText('BenchMesh', { exact: true })).toBeVisible();
    const pills = top.locator('.bench-pill');
    await expect(pills).toHaveText(['API', 'live']);
    await expect(pills.nth(0)).toHaveAttribute('data-ok', 'true');
    await expect(pills.nth(1)).toHaveAttribute('data-ok', 'true');
  });

  test('shows all navigation buttons', async ({ mockPage }) => {
    const nav = mockPage.getByRole('navigation', { name: 'Main' });
    for (const name of ['Arrange', 'Instruments', 'Recording', 'Log', 'Metrics', 'API']) {
      await expect(nav.getByRole('button', { name, exact: true })).toBeVisible();
    }
  });
});

test.describe('Drawers', () => {
  test('Instruments drawer opens and closes with the close button', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Instruments' });
    await expect(drawer).toBeVisible();
    await expect(drawer.getByTestId('settings-view')).toBeVisible();

    await drawer.getByRole('button', { name: 'Close' }).click();
    await expect(drawer).toBeHidden();
  });

  test('Instruments drawer closes with Escape', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Instruments' });
    await expect(drawer).toBeVisible();
    await mockPage.keyboard.press('Escape');
    await expect(drawer).toBeHidden();
  });

  test('Log drawer opens with the request log and closes on the scrim', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'Log', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Request log' });
    await expect(drawer).toBeVisible();
    await expect(drawer.locator('.logs-panel')).toBeVisible();

    // the scrim covers the page left of the sheet
    await drawer.locator('.bench-drawer__scrim').click({ position: { x: 10, y: 200 } });
    await expect(drawer).toBeHidden();
  });

  test('Recording opens the recordings drawer', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'Recording', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Recordings' });
    await expect(drawer).toBeVisible();
    await expect(drawer).toContainText('No recordings yet');
    await drawer.getByRole('button', { name: 'Close' }).click();
    await expect(drawer).toBeHidden();
  });
});

test.describe('Modals', () => {
  test('Metrics modal opens, shows device metrics and closes', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'Metrics', exact: true }).click();
    const heading = mockPage.getByRole('heading', { name: 'Serial Port Utilization Metrics' });
    await expect(heading).toBeVisible();
    await expect(mockPage.locator('.modal-content').getByRole('heading', { name: 'test-psu-1' })).toBeVisible();
    await mockPage.locator('.modal-close').click();
    await expect(heading).toBeHidden();
  });

  test('API docs modal opens with documentation and closes', async ({ mockPage }) => {
    await mockPage.getByRole('button', { name: 'API', exact: true }).click();
    const docs = mockPage.locator('.docs-modal-content');
    await expect(docs).toBeVisible();
    await expect(docs.locator('h1').first()).toBeVisible();
    await mockPage.locator('.modal-close').click();
    await expect(docs).toBeHidden();
  });
});

test.describe('Safety disclaimer', () => {
  test('shows on first load and is remembered when "don\'t show again" is ticked', async ({ page }) => {
    await setupApiMocks(page);
    await setupWebSocketMock(page);
    await page.goto('/ui/');

    const title = page.getByRole('heading', { name: /Important Safety Information/ });
    await expect(title).toBeVisible();
    await page.getByRole('checkbox', { name: "Don't show this message again" }).check();
    await page.getByRole('button', { name: 'I Agree and Continue' }).click();
    await expect(title).toBeHidden();
    expect(await page.evaluate(() => localStorage.getItem('benchmesh:disclaimer-accepted'))).toBe('true');

    await page.reload();
    await expect(page.getByRole('article').first()).toBeVisible();
    await expect(title).toBeHidden();
  });
});
