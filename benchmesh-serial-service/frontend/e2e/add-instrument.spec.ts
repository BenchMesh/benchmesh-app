/**
 * Adding an instrument: the Instruments drawer (instrument configuration tab) and the
 * empty-state "Add instrument" shortcut that opens it with a new form expanded.
 */
import { Page } from '@playwright/test';
import { test, expect } from './fixtures/base';

async function openInstrumentsDrawer(page: Page) {
  await page.getByRole('button', { name: 'Instruments', exact: true }).click();
  const drawer = page.getByRole('dialog', { name: 'Instruments' });
  await expect(drawer.getByTestId('instrument-config-view')).toBeVisible();
  // the configured devices are listed once the config has loaded
  await expect(drawer.locator('.accordion-item')).toHaveCount(2);
  return drawer;
}

/** Open the drawer and add a new instrument form; returns [drawer, new form item]. */
async function addNewForm(page: Page) {
  const drawer = await openInstrumentsDrawer(page);
  await drawer.getByTestId('add-instrument-btn').click();
  await expect(drawer.locator('.accordion-item').filter({ hasText: 'New Instrument' })).toHaveCount(1);
  // new forms are appended; `.last()` keeps matching after the name is edited
  const item = drawer.locator('.accordion-item').last();
  await expect(item).toContainText('New Instrument');
  return [drawer, item] as const;
}

test.describe('Instruments drawer', () => {
  test('opens on the instrument configuration tab listing the configured devices', async ({ mockPage }) => {
    const drawer = await openInstrumentsDrawer(mockPage);
    await expect(drawer.getByTestId('settings-tab-instruments')).toHaveClass(/settings-view__tab--active/);
    await expect(drawer.locator('.accordion-header')).toHaveText([/TENMA PSU \(test-psu-1\)/, /OWON DMM \(test-dmm-1\)/]);
  });

  test('expanding a configured device shows its settings and a Remove button', async ({ mockPage }) => {
    const drawer = await openInstrumentsDrawer(mockPage);
    const header = drawer.locator('.accordion-header').filter({ hasText: 'test-psu-1' });
    await header.click();
    await expect(header).toHaveAttribute('aria-expanded', 'true');
    const form = drawer.getByRole('region');
    await expect(form.getByPlaceholder('psu-1')).toHaveValue('test-psu-1');
    await expect(form.getByRole('button', { name: 'Remove' })).toBeVisible();
  });
});

test.describe('Add instrument', () => {
  test('"+ Add Instrument" adds one expanded form', async ({ mockPage }) => {
    const [drawer, item] = await addNewForm(mockPage);
    await expect(item.locator('.accordion-header')).toHaveAttribute('aria-expanded', 'true');
    await expect(drawer.locator('.accordion-item')).toHaveCount(3);
  });

  test('the form keeps its values while typing and choosing a driver', async ({ mockPage }) => {
    const [, item] = await addNewForm(mockPage);
    const id = item.getByPlaceholder('psu-1');
    const name = item.getByPlaceholder('My Instrument');
    await id.fill('bench-psu-2');
    await name.fill('Second PSU');
    await item.locator('select').filter({ hasText: 'Select driver...' }).selectOption('tenma_72');

    await expect(id).toHaveValue('bench-psu-2');
    await expect(name).toHaveValue('Second PSU');
    // models of the chosen driver are offered
    const model = item.locator('select').filter({ hasText: 'Select model...' });
    await expect(model.locator('option')).toHaveText(['Select model...', '72-2540', '72-2545', '72-2550']);
    // the header follows the edited name and id; the form stays open
    const header = mockPage.getByRole('button', { name: /Second PSU \(bench-psu-2\)/ });
    await expect(header).toHaveAttribute('aria-expanded', 'true');
  });

  test('IDs are sanitised to lowercase letters, digits and "-"', async ({ mockPage }) => {
    const [, item] = await addNewForm(mockPage);
    const id = item.getByPlaceholder('psu-1');
    await id.fill('My PSU_#1');
    await expect(id).not.toHaveValue(/[A-Z _#]/);
  });

  test('saves a valid instrument with POST /devices and reports success', async ({ mockPage, api }) => {
    const [, item] = await addNewForm(mockPage);
    await item.getByPlaceholder('psu-1').fill('bench-psu-2');
    await item.getByPlaceholder('My Instrument').fill('Second PSU');
    await item.locator('select').filter({ hasText: 'Select driver...' }).selectOption('tenma_72');
    await item.locator('select').filter({ hasText: 'Select model...' }).selectOption('72-2545');
    await item.getByRole('button', { name: 'Add Instrument' }).click();

    await expect(mockPage.getByText('✓ Applied successfully!')).toBeVisible();
    expect(api.deviceRequests).toHaveLength(1);
    expect(api.deviceRequests[0].method).toBe('POST');
    expect(JSON.parse(api.deviceRequests[0].body!)).toMatchObject({
      id: 'bench-psu-2', name: 'Second PSU', driver: 'tenma_72', model: '72-2545', port: '/dev/ttyUSB0',
    });
  });

  test('shows the server\'s validation error inline', async ({ mockPage, page }) => {
    await page.route('**/devices', (route) => route.fulfill({
      status: 422, contentType: 'application/json', body: JSON.stringify({ detail: 'driver is required' }),
    }));
    const [, item] = await addNewForm(mockPage);
    await item.getByRole('button', { name: 'Add Instrument' }).click();
    await expect(item.locator('.instrument-config__inline-error')).toHaveText('driver is required');
  });

  test('collapses and re-expands via its header', async ({ mockPage }) => {
    const [, item] = await addNewForm(mockPage);
    const header = item.locator('.accordion-header');
    await header.click();
    await expect(header).toHaveAttribute('aria-expanded', 'false');
    await expect(item.getByPlaceholder('psu-1')).toHaveCount(0);
    await header.click();
    await expect(header).toHaveAttribute('aria-expanded', 'true');
  });

  test('Cancel discards the unsaved instrument', async ({ mockPage }) => {
    const [drawer, item] = await addNewForm(mockPage);
    await item.getByRole('button', { name: 'Cancel' }).click();
    await expect(drawer.locator('.accordion-item').filter({ hasText: 'New Instrument' })).toHaveCount(0);
    await expect(drawer.locator('.accordion-item')).toHaveCount(2);
  });

  // UI bug: SettingsView unmounts InstrumentConfigView on a tab switch, so an unsaved new
  // instrument (and anything typed into it) is discarded.
  test('form data survives switching to Miscellaneous and back', async ({ mockPage }) => {
    const [drawer, item] = await addNewForm(mockPage);
    await item.getByPlaceholder('psu-1').fill('tab-test');
    await drawer.getByTestId('settings-tab-miscellaneous').click();
    await expect(drawer.getByText('History & Logs')).toBeVisible();
    await drawer.getByTestId('settings-tab-instruments').click();
    await expect(drawer.getByPlaceholder('psu-1')).toHaveValue('tab-test');
  });
});

test.describe('Empty bench', () => {
  // UI bug: InstrumentConfigView adds the auto-opened form on mount, before GET /config
  // resolves; loadConfig() then replaces the device list and the new form disappears, so
  // the drawer opens with "No instruments configured." and no form.
  test('"Add instrument" opens the Instruments drawer with one new form expanded', async ({ page, api }) => {
    api.instruments = [];
    api.config = { version: 1, devices: [] };
    await page.goto('/ui/');
    await page.getByRole('button', { name: 'Add instrument' }).click();

    const drawer = page.getByRole('dialog', { name: 'Instruments' });
    const item = drawer.locator('.accordion-item').filter({ hasText: 'New Instrument' });
    await expect(item).toHaveCount(1);
    await expect(item.locator('.accordion-header')).toHaveAttribute('aria-expanded', 'true');
    await expect(item.getByPlaceholder('psu-1')).toBeVisible();
  });
});
