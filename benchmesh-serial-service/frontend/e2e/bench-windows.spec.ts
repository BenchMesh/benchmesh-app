/**
 * Panel window controls: folding a panel to its header line, and detaching it into its own
 * window (a popup in the browser; an undecorated window in the desktop app).
 */
import { test, expect, gotoBench } from './fixtures/base';
import { PANELS } from './utils/api-mock';

test.describe('Folding', () => {
  test('folds a panel to its header and keeps it folded after a reload', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await expect(psu.getByRole('spinbutton', { name: 'Voltage setpoint' })).toBeVisible();
    await psu.getByRole('button', { name: 'Fold' }).click();
    await expect(psu.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveCount(0);
    await expect(psu.getByRole('heading', { name: 'Power Supply Unit' })).toBeVisible();
    expect((await psu.boundingBox())!.height).toBeLessThan(80);

    await gotoBench(mockPage, 3);
    const again = mockPage.getByRole('article', { name: PANELS.psu });
    await expect(again.getByRole('button', { name: 'Unfold' })).toBeVisible();
    await again.getByRole('button', { name: 'Unfold' }).click();
    await expect(again.getByRole('spinbutton', { name: 'Voltage setpoint' })).toBeVisible();
  });

  test('double-clicking the header folds too', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await psu.getByRole('heading', { name: 'Power Supply Unit' }).dblclick();
    await expect(psu.getByRole('button', { name: 'Unfold' })).toBeVisible();
  });
});

test.describe('Own window', () => {
  test('a detached panel moves to its own window and comes back when it closes', async ({ mockPage }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    const [win] = await Promise.all([
      mockPage.waitForEvent('popup'),
      psu.getByRole('button', { name: 'Open in own window' }).click(),
    ]);
    // the bench no longer shows it (not drawn twice) and lists it
    await expect(mockPage.getByRole('article', { name: PANELS.psu })).toHaveCount(0);
    const strip = mockPage.getByLabel('Panels in their own windows');
    await expect(strip).toContainText('Power Supply Unit · Test PSU');

    // the window holds just that panel, with fold and close (no detach, no browser pin)
    const own = win.getByRole('article', { name: PANELS.psu });
    await expect(own).toBeVisible();
    await expect(win.getByRole('article')).toHaveCount(1);
    await expect(own.getByRole('button', { name: 'Open in own window' })).toHaveCount(0);
    await expect(own.getByRole('button', { name: 'Keep on top' })).toHaveCount(0);
    await expect(win).toHaveTitle('Power Supply Unit · Test PSU');
    await own.getByRole('button', { name: 'Fold' }).click();
    await expect(own.getByRole('spinbutton', { name: 'Voltage setpoint' })).toHaveCount(0);

    await own.getByRole('button', { name: 'Close window' }).click();
    await expect(mockPage.getByRole('article', { name: PANELS.psu })).toHaveCount(1);
    await expect(strip).toHaveCount(0);
  });

  test('"back to the bench" closes the window from the main view', async ({ mockPage }) => {
    const [win] = await Promise.all([
      mockPage.waitForEvent('popup'),
      mockPage.getByRole('article', { name: PANELS.dmm }).getByRole('button', { name: 'Open in own window' }).click(),
    ]);
    await expect(win.getByRole('article', { name: PANELS.dmm })).toBeVisible();
    await mockPage.getByRole('button', { name: /Back to the bench/ }).click();
    await expect(mockPage.getByRole('article', { name: PANELS.dmm })).toHaveCount(1);
  });

  test('in its own window the header stays put; only the panel body scrolls', async ({ mockPage }) => {
    const [win] = await Promise.all([
      mockPage.waitForEvent('popup'),
      mockPage.getByRole('article', { name: PANELS.psu }).getByRole('button', { name: 'Open in own window' }).click(),
    ]);
    await win.setViewportSize({ width: 420, height: 160 });
    const head = win.locator('.bench-panel__head');
    await expect(head).toBeVisible();
    const top = (await head.boundingBox())!.y;
    await win.mouse.move(200, 120);
    await win.mouse.wheel(0, 2000);
    await expect.poll(() => win.locator('.bench-panel__body').evaluate((e) => e.scrollTop)).toBeGreaterThan(0);
    expect((await head.boundingBox())!.y).toBe(top);
    expect(await win.evaluate(() => document.scrollingElement!.scrollTop)).toBe(0);
  });
});
