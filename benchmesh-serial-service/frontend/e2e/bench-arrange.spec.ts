/**
 * Arranging the bench: reorder (buttons and drag and drop), panel widths, hiding and
 * restoring panels, persistence of the layout, and the responsive grid.
 */
import { Page } from '@playwright/test';
import { test, expect, gotoBench } from './fixtures/base';
import { PANELS } from './utils/api-mock';

const LAYOUT_KEY = 'benchmesh:bench-layout';

const panelOrder = (page: Page) =>
  page.getByRole('article').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));

const storedLayout = (page: Page) =>
  page.evaluate((k) => JSON.parse(localStorage.getItem(k) ?? 'null'), LAYOUT_KEY);

async function startArranging(page: Page) {
  await page.getByRole('button', { name: 'Arrange' }).click();
  await expect(page.getByRole('button', { name: 'Done' })).toHaveAttribute('aria-pressed', 'true');
}

const panel = (page: Page, name: string) => page.getByRole('article', { name });

test.describe('Arrange mode', () => {
  test('toggles the arrange controls on and off', async ({ mockPage }) => {
    await expect(mockPage.getByRole('button', { name: 'Move earlier' })).toHaveCount(0);
    await startArranging(mockPage);
    await expect(mockPage.getByRole('button', { name: 'Move earlier' })).toHaveCount(3);
    await expect(mockPage.getByRole('group', { name: 'Panel width' })).toHaveCount(3);
    await expect(mockPage.getByRole('button', { name: 'Hide panel' })).toHaveCount(3);

    await mockPage.getByRole('button', { name: 'Done' }).click();
    await expect(mockPage.getByRole('button', { name: 'Arrange' })).toHaveAttribute('aria-pressed', 'false');
    await expect(mockPage.getByRole('button', { name: 'Move earlier' })).toHaveCount(0);
  });

  test('Move earlier / Move later reorder the panels', async ({ mockPage }) => {
    expect(await panelOrder(mockPage)).toEqual([PANELS.psu, PANELS.dmm, PANELS.lcr]);
    await startArranging(mockPage);

    await panel(mockPage, PANELS.dmm).getByRole('button', { name: 'Move earlier' }).click();
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.dmm, PANELS.psu, PANELS.lcr]);

    await panel(mockPage, PANELS.psu).getByRole('button', { name: 'Move later' }).click();
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.dmm, PANELS.lcr, PANELS.psu]);

    // the ends are no-ops
    await panel(mockPage, PANELS.dmm).getByRole('button', { name: 'Move earlier' }).click();
    await panel(mockPage, PANELS.psu).getByRole('button', { name: 'Move later' }).click();
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.dmm, PANELS.lcr, PANELS.psu]);
  });

  test('dragging a panel header onto another panel moves it there', async ({ mockPage }) => {
    await startArranging(mockPage);
    const grip = panel(mockPage, PANELS.psu).getByRole('heading', { name: 'Power Supply Unit' });
    await grip.dragTo(panel(mockPage, PANELS.lcr));
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.dmm, PANELS.lcr, PANELS.psu]);

    const lcr = panel(mockPage, PANELS.lcr).getByRole('heading', { name: 'LCR Meter' });
    await lcr.dragTo(panel(mockPage, PANELS.dmm));
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.lcr, PANELS.dmm, PANELS.psu]);
  });

  test('panels cannot be dragged outside arrange mode', async ({ mockPage }) => {
    const grip = panel(mockPage, PANELS.psu).getByRole('heading', { name: 'Power Supply Unit' });
    await grip.dragTo(panel(mockPage, PANELS.lcr));
    expect(await panelOrder(mockPage)).toEqual([PANELS.psu, PANELS.dmm, PANELS.lcr]);
  });

  test('the width selector resizes a panel', async ({ mockPage }) => {
    await startArranging(mockPage);
    const psu = panel(mockPage, PANELS.psu);
    const width = psu.getByRole('group', { name: 'Panel width' });
    const rack = mockPage.locator('.bench-rack');
    const rackWidth = (await rack.boundingBox())!.width;

    // PSU defaults to M
    await expect(width.getByRole('button', { name: 'M', exact: true })).toHaveAttribute('aria-pressed', 'true');

    await width.getByRole('button', { name: 'W', exact: true }).click();
    await expect(width.getByRole('button', { name: 'W', exact: true })).toHaveAttribute('aria-pressed', 'true');
    await expect.poll(async () => (await psu.boundingBox())!.width).toBeGreaterThan(rackWidth * 0.95);

    await width.getByRole('button', { name: 'S', exact: true }).click();
    await expect.poll(async () => (await psu.boundingBox())!.width).toBeLessThan(rackWidth * 0.5);
    expect((await storedLayout(mockPage)).size['test-psu-1:PSU']).toBe('s');
  });

  test('a hidden panel goes to the shelf and comes back from its chip', async ({ mockPage }) => {
    await startArranging(mockPage);
    await panel(mockPage, PANELS.dmm).getByRole('button', { name: 'Hide panel' }).click();
    await expect(mockPage.getByRole('article')).toHaveCount(2);
    await expect(panel(mockPage, PANELS.dmm)).toHaveCount(0);

    const chip = mockPage.getByRole('button', { name: '+ Digital Multimeter · Test DMM' });
    await expect(chip).toBeVisible();
    await chip.click();
    await expect(chip).toHaveCount(0);
    await expect.poll(() => panelOrder(mockPage)).toEqual([PANELS.psu, PANELS.dmm, PANELS.lcr]);
  });

  // UI bug: on an offline panel the "Offline — reconnecting" badge (.bench-panel__offline,
  // absolutely positioned at top: 46px) covers the wrapped arrange controls of a narrow
  // panel and intercepts the click on "Hide panel" / the width buttons.
  test('an offline panel can be hidden', async ({ mockPage }) => {
    await startArranging(mockPage);
    await panel(mockPage, PANELS.lcr).getByRole('button', { name: 'Hide panel' }).click({ timeout: 5000 });
    await expect(panel(mockPage, PANELS.lcr)).toHaveCount(0);
  });

  test('order, widths and hidden panels persist across a reload', async ({ mockPage }) => {
    await startArranging(mockPage);
    await panel(mockPage, PANELS.dmm).getByRole('button', { name: 'Move earlier' }).click();
    await panel(mockPage, PANELS.psu).getByRole('group', { name: 'Panel width' }).getByRole('button', { name: 'L', exact: true }).click();
    await panel(mockPage, PANELS.psu).getByRole('button', { name: 'Move later' }).click();
    await panel(mockPage, PANELS.dmm).getByRole('button', { name: 'Hide panel' }).click();
    await mockPage.getByRole('button', { name: 'Done' }).click();

    await expect.poll(() => storedLayout(mockPage)).toEqual({
      order: ['test-dmm-1:DMM', 'test-lcr-1:LCR', 'test-psu-1:PSU'],
      size: { 'test-psu-1:PSU': 'l', 'test-dmm-1:DMM': 'm', 'test-lcr-1:LCR': 's' },
      hidden: ['test-dmm-1:DMM'],
      folded: [],
    });

    await gotoBench(mockPage, 2);
    expect(await panelOrder(mockPage)).toEqual([PANELS.lcr, PANELS.psu]);
    await startArranging(mockPage);
    await expect(panel(mockPage, PANELS.psu).getByRole('group', { name: 'Panel width' }).getByRole('button', { name: 'L', exact: true }))
      .toHaveAttribute('aria-pressed', 'true');
    await expect(mockPage.getByRole('button', { name: '+ Digital Multimeter · Test DMM' })).toBeVisible();
  });

  test('a corrupt stored layout is ignored', async ({ mockPage }) => {
    await mockPage.evaluate((k) => localStorage.setItem(k, '{not json'), LAYOUT_KEY);
    await gotoBench(mockPage, 3);
    expect(await panelOrder(mockPage)).toEqual([PANELS.psu, PANELS.dmm, PANELS.lcr]);
  });
});

test.describe('Responsive rack', () => {
  test('a phone-width screen stacks the panels without horizontal scrolling', async ({ page, api }) => {
    void api;
    await page.setViewportSize({ width: 400, height: 800 });
    await gotoBench(page, 3);

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);

    const boxes = await page.getByRole('article').evaluateAll((els) => els.map((e) => e.getBoundingClientRect().toJSON()));
    for (const b of boxes) expect(b.right).toBeLessThanOrEqual(400);
    for (let i = 1; i < boxes.length; i++) {
      expect(boxes[i].left).toBeCloseTo(boxes[0].left, 0);
      expect(boxes[i].top).toBeGreaterThanOrEqual(boxes[i - 1].bottom);
    }
  });

  test('a wide screen puts several panels in a row', async ({ page, api }) => {
    void api;
    await page.setViewportSize({ width: 1920, height: 1080 });
    await gotoBench(page, 3);

    const tops = await page.getByRole('article').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    const firstRow = tops.filter((t) => t === tops[0]).length;
    expect(firstRow).toBeGreaterThanOrEqual(2);
  });
});
