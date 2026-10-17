/**
 * Instrument list polling (ETag / 304, error retry) and the registry WebSocket, as seen in
 * the top bar status pills and the bench.
 */
import { test, expect } from './fixtures/base';
import { mockInstruments } from './utils/api-mock';

const apiPill = (page: import('@playwright/test').Page) => page.locator('.bench-pill').nth(0);
const wsPill = (page: import('@playwright/test').Page) => page.locator('.bench-pill').nth(1);

test.describe('Instrument list polling', () => {
  test('renders the instruments returned by GET /instruments', async ({ page, api }) => {
    let calls = 0;
    await page.route('**/instruments', (route) => {
      calls++;
      return route.fulfill({
        status: 200, contentType: 'application/json', headers: { ETag: '"one"' },
        body: JSON.stringify([{ id: 'api-test-1', IDN: 'TEST,INSTRUMENT,SN001,V1.0', classes: [{ class: 'PSU', channels: ['1'] }] }]),
      });
    });
    api.instruments = [{ id: 'api-test-1', classes: [{ class: 'PSU', channels: ['1'] }] }];
    await page.goto('/ui/');
    // falls back to the id when the instrument has no name
    await expect(page.getByRole('article', { name: 'Power Supply Unit: api-test-1' })).toBeVisible();
    expect(calls).toBeGreaterThan(0);
    await expect(apiPill(page)).toHaveText('API');
  });

  test('sends If-None-Match with the last ETag and keeps the bench on 304', async ({ page }) => {
    const seen: (string | undefined)[] = [];
    await page.route('**/instruments', (route) => {
      const inm = route.request().headers()['if-none-match'];
      seen.push(inm);
      if (inm === '"etag-123"') return route.fulfill({ status: 304, headers: { ETag: '"etag-123"' } });
      return route.fulfill({
        status: 200, contentType: 'application/json', headers: { ETag: '"etag-123"' }, body: JSON.stringify(mockInstruments),
      });
    });
    await page.goto('/ui/');
    await expect(page.getByRole('article')).toHaveCount(3);

    // the list is polled every 5 s
    await expect.poll(() => seen.length, { timeout: 12_000 }).toBeGreaterThanOrEqual(2);
    expect(seen[0]).toBeUndefined();
    expect(seen[1]).toBe('"etag-123"');
    await expect(page.getByRole('article')).toHaveCount(3);
    await expect(apiPill(page)).toHaveText('API');
  });

  test('shows "API unreachable" on errors and recovers on the 1 s retry', async ({ page }) => {
    let calls = 0;
    await page.route('**/instruments', (route) => {
      calls++;
      if (calls === 1) {
        return route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ detail: 'boom' }) });
      }
      return route.fulfill({ status: 200, contentType: 'application/json', headers: { ETag: '"ok"' }, body: JSON.stringify(mockInstruments) });
    });
    await page.goto('/ui/');
    await expect(apiPill(page)).toHaveText('API unreachable');
    await expect(apiPill(page)).toHaveAttribute('data-ok', 'false');

    await expect(apiPill(page)).toHaveText('API', { timeout: 5000 });
    await expect(apiPill(page)).toHaveAttribute('data-ok', 'true');
    await expect(page.getByRole('article')).toHaveCount(3);
  });

  test('treats a malformed JSON reply as an error', async ({ page }) => {
    await page.route('**/instruments', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: 'invalid json {]' }));
    await page.goto('/ui/');
    await expect(apiPill(page)).toHaveText('API unreachable');
  });

  test('shows "connecting…" while the first request is pending', async ({ page }) => {
    await page.route('**/instruments', () => { /* never answered */ });
    await page.goto('/ui/');
    await expect(apiPill(page)).toHaveText('connecting…');
    await expect(page.getByRole('heading', { name: 'No instruments yet' })).toHaveCount(0);
  });
});

test.describe('Registry WebSocket', () => {
  test('shows "live" and keeps receiving updates', async ({ mockPage }) => {
    const pill = wsPill(mockPage);
    await expect(pill).toHaveText('live');
    // the stream is flagged stale after 1 s without data; it must stay live across updates
    for (let i = 0; i < 3; i++) {
      await mockPage.waitForTimeout(700);
      await expect(pill).toHaveText('live');
    }
  });

  test('reports a failing connection and never shows "live"', async ({ page, api }) => {
    void api;
    // replaces the fixture's WebSocket mock (init scripts run in order)
    await page.addInitScript(() => {
      class FailingWebSocket {
        url: string;
        readyState = 0;
        onopen: ((e: Event) => void) | null = null;
        onclose: ((e: CloseEvent) => void) | null = null;
        onerror: ((e: Event) => void) | null = null;
        onmessage: ((e: MessageEvent) => void) | null = null;
        constructor(url: string) {
          this.url = url;
          setTimeout(() => {
            this.readyState = 3;
            this.onerror?.(new Event('error'));
            this.onclose?.(new CloseEvent('close'));
          }, 50);
        }
        close() { this.readyState = 3; }
        send() { /* no-op */ }
      }
      (window as any).WebSocket = FailingWebSocket;
    });
    await page.goto('/ui/');
    const pill = wsPill(page);
    await expect(pill).toHaveText(/disconnected|error|connecting…/);
    await expect(pill).toHaveAttribute('data-ok', 'false');
    // instruments are listed but none is online without registry data
    await expect(page.getByRole('article')).toHaveCount(3);
    await expect(page.locator('.bench-panel__link').filter({ hasText: 'connected' })).toHaveCount(0);
  });

  test('flags the stream as stale when updates stop', async ({ mockPage }) => {
    await expect(wsPill(mockPage)).toHaveText('live');
    await mockPage.evaluate(() => { (window as any).__registryPaused = true; });
    await expect(wsPill(mockPage)).toHaveText('no data >1s');
  });
});
