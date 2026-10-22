/**
 * Remote access (ai_context/REMOTE_ACCESS.md): the settings in the app, the top-bar badge while
 * remote browsers are connected, and what a remote browser sees (no admin controls, Log out).
 */
import { Page } from '@playwright/test';
import { test, expect, gotoBench } from './fixtures/base';
import { REMOTE_FINGERPRINT, REMOTE_URL } from './utils/api-mock';

const SESSIONS = [
  { id: 's1', address: '192.168.1.30', agent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0 Safari/537.36', since: Date.now() - 60000, last_seen: Date.now() },
  { id: 's2', address: '192.168.1.31', agent: 'Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0', since: Date.now() - 30000, last_seen: Date.now() },
];

async function openRemoteSettings(page: Page) {
  await page.getByRole('button', { name: 'Instruments', exact: true }).click();
  const drawer = page.getByRole('dialog', { name: 'Instruments' });
  await drawer.getByTestId('settings-tab-remote').click();
  return drawer.getByRole('region', { name: 'Remote access settings' });
}

test.describe('Remote access settings (app on this computer)', () => {
  test('set a password, switch on, see the address and the certificate fingerprint', async ({ mockPage, api }) => {
    const section = await openRemoteSettings(mockPage);
    await expect(section.getByText(/Control this bench from a browser on your local network/)).toBeVisible();
    const sw = section.getByRole('switch', { name: 'Remote access' });
    await expect(sw).toBeDisabled();

    await section.getByLabel('Password', { exact: true }).fill('too short');
    await section.getByLabel('Repeat password').fill('too short');
    await section.getByRole('button', { name: 'Set password' }).click();
    await expect(section.getByRole('alert')).toHaveText(/At least 10 characters/);
    await section.getByLabel('Password', { exact: true }).fill('bench password 1');
    await section.getByLabel('Repeat password').fill('bench password 2');
    await section.getByRole('button', { name: 'Set password' }).click();
    await expect(section.getByRole('alert')).toHaveText(/do not match/);

    await section.getByLabel('Repeat password').fill('bench password 1');
    await section.getByRole('button', { name: 'Set password' }).click();
    await expect(section.getByText('Password saved.')).toBeVisible();
    await expect(section.getByTestId('password-state')).toHaveText('Password set');
    expect(api.remote.requests).toContainEqual({ method: 'PUT', path: '/remote/password', body: JSON.stringify({ password: 'bench password 1' }) });

    await expect(sw).toBeEnabled();
    await sw.click();
    await expect(sw).toHaveAttribute('aria-checked', 'true');
    await expect(section.getByTestId('remote-state')).toHaveText('Listening on port 57667');
    await expect(section.getByRole('link', { name: REMOTE_URL })).toHaveAttribute('href', REMOTE_URL);
    await expect(section.getByTestId('remote-fingerprint')).toHaveText(REMOTE_FINGERPRINT);
    await expect(section.getByText(/check that its SHA-256 fingerprint matches/)).toBeVisible();
    await expect(section.getByText('Nobody is connected.')).toBeVisible();
  });

  test('a view-only password next to the remote access password', async ({ mockPage, api }) => {
    Object.assign(api.remote, { password_set: true });
    const section = await openRemoteSettings(mockPage);
    const form = section.getByRole('form', { name: 'View-only password' });
    await expect(form.getByTestId('view-password-state')).toHaveText(/change nothing\. Not set\./);
    await form.getByLabel('View-only password', { exact: true }).fill('look only please');
    await form.getByLabel('Repeat view-only password').fill('look only please');
    await form.getByRole('button', { name: 'Set view-only password' }).click();
    await expect(form.getByText('Password saved.')).toBeVisible();
    await expect(form.getByTestId('view-password-state')).toHaveText('Password set');
    expect(api.remote.requests).toContainEqual({ method: 'PUT', path: '/remote/view-password', body: JSON.stringify({ password: 'look only please' }) });
    await form.getByRole('button', { name: 'Remove' }).click();
    await expect(form.getByText('View-only password removed.')).toBeVisible();
    expect(api.remote.view_password_set).toBe(false);
  });

  test('is not offered by a server without remote access', async ({ page, api }) => {
    await page.route('**/remote', (route) => route.fulfill({ status: 404, contentType: 'application/json', body: '{"detail":"Not found"}' }));
    await gotoBench(page, api.instruments.length);
    await page.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = page.getByRole('dialog', { name: 'Instruments' });
    await expect(drawer.getByTestId('settings-tab-miscellaneous')).toBeVisible();
    await expect(drawer.getByTestId('settings-tab-remote')).toHaveCount(0);
  });
});

test.describe('Remote badge (app on this computer)', () => {
  test('shows who is connected; Disconnect all and switch off ends remote access', async ({ page, api }) => {
    Object.assign(api.remote, { enabled: true, password_set: true, sessions: structuredClone(SESSIONS) });
    await gotoBench(page, api.instruments.length);
    const top = page.locator('header.bench-top');
    const badge = top.getByRole('button', { name: 'Remote: 2' });
    await expect(badge).toBeVisible();
    await badge.click();
    const pop = page.getByRole('dialog', { name: 'Remote sessions' });
    await expect(pop.getByText('192.168.1.30')).toBeVisible();
    await expect(pop.getByText('Chrome on Windows')).toBeVisible();

    await pop.getByRole('button', { name: 'Disconnect 192.168.1.31' }).click();
    await expect(top.getByRole('button', { name: 'Remote: 1' })).toBeVisible();
    expect(api.remote.requests).toContainEqual({ method: 'DELETE', path: '/remote/sessions/s2', body: null });

    await pop.getByRole('button', { name: 'Disconnect all and switch off' }).click();
    await expect(top.getByRole('button', { name: /Remote:/ })).toHaveCount(0);
    expect(api.remote.requests).toContainEqual({ method: 'PUT', path: '/remote', body: JSON.stringify({ enabled: false }) });
    expect(api.remote.enabled).toBe(false);
  });

  test('appears when someone connects and goes when nobody is', async ({ mockPage, api }) => {
    await expect(mockPage.getByRole('button', { name: /Remote:/ })).toHaveCount(0);
    Object.assign(api.remote, { enabled: true, password_set: true, sessions: structuredClone(SESSIONS.slice(0, 1)) });
    await expect(mockPage.getByRole('button', { name: 'Remote: 1' })).toBeVisible({ timeout: 8000 });
    api.remote.sessions = [];
    await expect(mockPage.getByRole('button', { name: /Remote:/ })).toHaveCount(0, { timeout: 8000 });
  });
});

test.describe('A remote browser', () => {
  test.beforeEach(async ({ api }) => {
    api.remote.session = { principal: 'remote', address: '192.168.1.30', since: Date.now() };
  });

  test('has no admin controls and can log out', async ({ mockPage, api }) => {
    const top = mockPage.locator('header.bench-top');
    await expect(top.getByText('Remote', { exact: true })).toBeVisible();

    // no remote access settings
    await mockPage.getByRole('button', { name: 'Instruments', exact: true }).click();
    const drawer = mockPage.getByRole('dialog', { name: 'Instruments' });
    await expect(drawer.getByTestId('settings-tab-miscellaneous')).toBeVisible();
    await expect(drawer.getByTestId('settings-tab-remote')).toHaveCount(0);
    await drawer.getByRole('button', { name: 'Close' }).click();

    // no assistant key or MCP access
    await mockPage.getByRole('button', { name: 'Automation', exact: true }).click();
    const auto = mockPage.getByRole('dialog', { name: 'Automation' });
    await auto.getByRole('tab', { name: 'Settings' }).click();
    await expect(auto.getByText(/Change this in the BenchMesh app on the bench computer/)).toHaveCount(2);
    await expect(auto.getByLabel(/API key/)).toHaveCount(0);
    await expect(auto.getByRole('radio')).toHaveCount(0);
    await auto.getByRole('button', { name: 'Close' }).click();
    expect(api.remote.requests.filter((r) => r.path !== '/remote/logout')).toEqual([]);

    await top.getByRole('button', { name: 'Log out' }).click();
    await expect(mockPage.getByRole('heading', { name: 'Sign in to BenchMesh' })).toBeVisible();
    expect(api.remote.requests.some((r) => r.method === 'POST' && r.path === '/remote/logout')).toBe(true);
  });

  test('signed in with the view-only password: sees the bench, gets no controls', async ({ page, api }) => {
    api.remote.session = { principal: 'remote', role: 'view', address: '192.168.1.31', since: Date.now() };
    await gotoBench(page, api.instruments.length);
    await expect(page.locator('header.bench-top').getByText('Remote - view only')).toBeVisible();
    const panels = page.locator('article.bench-panel');
    await expect(panels.first()).toBeVisible();
    await expect(page.locator('article.bench-panel:not(.bench-panel--view)')).toHaveCount(0);
    await expect(page.locator('.bench-panel__body:not([inert])')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Park instrument' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: '+ New recording' })).toHaveCount(0);
  });
});
