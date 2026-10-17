import { test as base, expect, Page } from '@playwright/test';
import { ApiMock, acceptDisclaimer, setupApiMocks, setupWebSocketMock } from '../utils/api-mock';

/**
 * Test fixtures:
 * - `api`: the mocked API (installed, not yet navigated); WebSocket mocked, disclaimer accepted.
 * - `mockPage`: the bench loaded at /ui/ with the mock instruments' panels rendered and live.
 */
export const test = base.extend<{ api: ApiMock; mockPage: Page }>({
  api: async ({ page }, use) => {
    const api = await setupApiMocks(page);
    await setupWebSocketMock(page);
    await acceptDisclaimer(page);
    await use(api);
    // don't let in-flight route handlers (e.g. a deliberately unanswered request) hold up teardown
    await page.unrouteAll({ behavior: 'ignoreErrors' });
  },

  mockPage: async ({ page, api }, use) => {
    await gotoBench(page, api.instruments.length);
    await use(page);
  },
});

/** Open the bench and wait until `panels` panels are shown and the registry stream is live. */
export async function gotoBench(page: Page, panels: number) {
  await page.goto('/ui/');
  await expect(page.getByRole('article')).toHaveCount(panels);
  await expect(page.locator('header.bench-top')).toContainText('live');
}

export { expect };
