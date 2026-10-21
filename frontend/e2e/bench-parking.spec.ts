/**
 * Parking (ai_context/PARKING.md): a parked instrument leaves the rack for the parking strip on
 * the left (name and presence always visible), and returns to its place when unparked.
 */
import { test, expect } from './fixtures/base';
import { mockRegistry, PANELS } from './utils/api-mock';

const parkedRegistry = (present: boolean | null, detail: string) => ({
  ...mockRegistry,
  'test-psu-1': { parked: { present, detail } },
});

test.describe('Parking', () => {
  test('parks an instrument into the strip and unparks it back to its place', async ({ mockPage, api }) => {
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await psu.getByRole('button', { name: 'Park instrument' }).click();
    await expect.poll(() => api.deviceRequests.map((r) => `${r.method} ${r.path}`)).toEqual(['POST /devices/test-psu-1/park']);
    await mockPage.evaluate((r) => (window as any).__pushRegistry(r), parkedRegistry(true, '/dev/ttyUSB0'));

    await expect(mockPage.getByRole('article', { name: PANELS.psu })).toHaveCount(0);
    const strip = mockPage.getByRole('complementary', { name: 'Parked instruments' });
    const tab = strip.getByRole('button', { name: 'Test PSU, parked, attached' });
    await expect(tab).toBeVisible();
    // the strip sits left of the rack without covering it
    const stripBox = (await strip.boundingBox())!;
    const first = (await mockPage.getByRole('article').first().boundingBox())!;
    expect(stripBox.x + stripBox.width).toBeLessThanOrEqual(first.x);
    expect(stripBox.width).toBeLessThan(48);

    // presence changes show without opening the strip
    await mockPage.evaluate((r) => (window as any).__pushRegistry(r), parkedRegistry(false, 'not attached: no serial port matches usb 0416:5011'));
    await expect(strip.getByRole('button', { name: 'Test PSU, parked, not found' })).toBeVisible();

    await strip.getByRole('button', { name: 'Test PSU, parked, not found' }).click();
    const sheet = mockPage.getByRole('dialog', { name: 'Parking' });
    await expect(sheet).toContainText('Not found: not attached: no serial port matches usb 0416:5011');
    const box = (await sheet.boundingBox())!;
    expect(box.x + box.width).toBeLessThanOrEqual(mockPage.viewportSize()!.width);
    await sheet.getByRole('button', { name: 'Unpark' }).click();
    await expect.poll(() => api.deviceRequests.at(-1)).toMatchObject({ method: 'DELETE', path: '/devices/test-psu-1/park' });
    await mockPage.evaluate((r) => (window as any).__pushRegistry(r), mockRegistry);

    await expect(mockPage.getByRole('article', { name: PANELS.psu })).toBeVisible();
    await expect(mockPage.getByRole('article').first()).toHaveAttribute('aria-label', PANELS.psu);
    await expect(strip).toHaveCount(0);
  });

  test("shows the server's refusal in the panel", async ({ mockPage, api }) => {
    api.failures.set('/devices/test-psu-1/park', { status: 409, detail: 'test-psu-1 is used by the running flow "Sweep": stop the flow before parking it' });
    const psu = mockPage.getByRole('article', { name: PANELS.psu });
    await psu.getByRole('button', { name: 'Park instrument' }).click();
    await expect(psu.getByRole('alert')).toContainText('used by the running flow "Sweep"');
    await expect(psu).toBeVisible();
  });
});
