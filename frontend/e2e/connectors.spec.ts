/**
 * Instruments → Connectors (ai_context/SCPI.md): virtual SCPI access, each instrument's VISA
 * address with code for it, the Python package; not offered to a remote browser.
 */
import { test, expect } from './fixtures/base';

test.describe('Connectors', () => {
  test('switch virtual SCPI on and get an instrument address with code', async ({ mockPage, api }) => {
    await mockPage.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Instruments' });
    await drawer.getByTestId('settings-tab-connectors').click();
    const section = drawer.getByRole('region', { name: 'Connectors' });
    await expect(section.getByRole('radio', { name: /Off/ })).toBeChecked();
    await expect(section.getByRole('list', { name: 'Virtual SCPI addresses' })).toHaveCount(0);
    await expect(section.getByText(/pip install benchmesh/)).toBeVisible();

    await section.getByRole('radio', { name: /Read/ }).click();
    await expect(section.getByRole('radio', { name: /Read/ })).toBeChecked();
    const list = section.getByRole('list', { name: 'Virtual SCPI addresses' });
    await expect(list.getByText('TCPIP0::127.0.0.1::5025::SOCKET')).toBeVisible();
    await expect(section.getByText(/visadev\("TCPIP0::127\.0\.0\.1::5025::SOCKET"\)/)).toBeVisible();
    expect(api.scpi.access).toBe('read');

    await section.getByLabel('First port').fill('6000');
    await section.getByRole('button', { name: 'Save' }).click();
    await expect(list.getByText('TCPIP0::127.0.0.1::6000::SOCKET')).toBeVisible();
  });
});
