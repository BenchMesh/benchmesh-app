import { test, expect } from '@playwright/test';

/**
 * Basic E2E tests that run against the REAL BenchMesh service
 *
 * Prerequisites:
 * - Backend service must be running on port 57666
 * - Service should have a valid config.yaml
 *
 * Run with: npx playwright test --config=playwright.config.with-service.ts with-service/
 */

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('benchmesh:disclaimer-accepted', 'true'));
});

const apiPill = (page: import('@playwright/test').Page) => page.locator('.bench-pill').nth(0);

test.describe('BenchMesh App - Real Service', () => {
  test('should load the app and connect to real API', async ({ page }) => {
    await page.goto('/ui/');
    await expect(page.locator('.bench-brand')).toHaveText('BenchMesh');
    await expect(apiPill(page)).toHaveText('API', { timeout: 10000 });
  });

  test('should connect to WebSocket', async ({ page }) => {
    await page.goto('/ui/');
    await expect(page.locator('.bench-pill').nth(1)).toHaveText('live', { timeout: 10000 });
  });

  test('should display navigation elements', async ({ page }) => {
    await page.goto('/ui/');
    for (const name of ['Arrange', 'Instruments', 'Recording', 'Log', 'Metrics', 'API']) {
      await expect(page.getByRole('button', { name, exact: true })).toBeVisible();
    }
  });

  test('should open the Instruments drawer with the configuration', async ({ page }) => {
    await page.goto('/ui/');
    await page.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = page.getByRole('dialog', { name: 'Instruments' });
    await expect(drawer.getByTestId('instrument-config-view')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(drawer).toBeHidden();
  });

  test('should display instruments from config', async ({ page }) => {
    await page.goto('/ui/');
    await expect(apiPill(page)).toHaveText('API', { timeout: 10000 });
    // one panel per instrument class, or the empty state (depends on config.yaml)
    await expect(page.getByRole('article').first().or(page.getByRole('heading', { name: 'No instruments yet' }))).toBeVisible();
  });

  test('should access documentation', async ({ page }) => {
    await page.goto('/ui/');
    await page.getByRole('button', { name: 'API', exact: true }).click();
    await expect(page.locator('.docs-modal-content')).toBeVisible();
    await page.locator('.modal-close').first().click();
    await expect(page.locator('.docs-modal-content')).toBeHidden();
  });

  test('should access metrics viewer', async ({ page }) => {
    await page.goto('/ui/');
    await page.getByRole('button', { name: 'Metrics', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Serial Port Utilization Metrics' })).toBeVisible();
    await page.locator('.modal-close').first().click();
    await expect(page.locator('.modal-overlay')).toBeHidden();
  });

  test('should redirect non-existent routes to main UI', async ({ page }) => {
    await page.goto('/ui/nonexistent');
    await expect(page.locator('.bench-brand')).toBeVisible();
    await expect(apiPill(page)).toHaveText('API', { timeout: 10000 });
  });
});
