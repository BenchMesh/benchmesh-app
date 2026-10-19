# BenchMesh E2E Tests

End-to-end tests for the BenchMesh bench UI using Playwright. The API and the registry
WebSocket are fully mocked, so the suite needs no backend or hardware.

## Structure

```
e2e/
├── fixtures/base.ts                 # `api` (mock handle) and `mockPage` (bench loaded) fixtures
├── utils/api-mock.ts                # API route mock, WebSocket mock, mock instruments/registry
├── app-navigation.spec.ts           # top bar, drawers (Instruments, Log), modals, disclaimer
├── instrument-display.spec.ts       # panels per class, names, model, online/offline, live values
├── instrument-interaction.spec.ts   # setpoints (Enter sends), output arm switch, DMM dial/range, errors
├── add-instrument.spec.ts           # Instruments drawer: configured devices, add-instrument form
├── settings-view.spec.ts            # Instruments drawer tabs, history retention setting
├── history-retention.spec.ts        # request log persistence, retention pruning, 1000-entry cap
├── api-integration.spec.ts          # instrument polling (ETag/304, retry), WebSocket status
├── bench-arrange.spec.ts            # arrange mode: reorder, drag and drop, widths, hide, persistence, responsive
└── with-service/                    # against a real service (separate config, not run by default)
```

## Running

```bash
npx vite build                 # the suite serves dist/ via `npm run preview` on :5173
npm run test:e2e               # all mocked specs (headless)
npx playwright test bench-arrange.spec.ts
npx playwright test -g "output"
npm run test:e2e:ui | test:e2e:headed | test:e2e:debug
npm run test:e2e:service       # with-service/ against a running backend on :57666
```

## Mocks

`utils/api-mock.ts`:

- **Instruments**: a PSU (`test-psu-1`) and a DMM (`test-dmm-1`), both online, and an
  oscilloscope (`test-osc-1`) without IDN (offline, generic panel). `PANELS` holds their
  panel names (`"<class description>: <name>"`).
- **Features / methods**: PSU `absolute_limits` and numeric setters with value specs; DMM
  `modes` and `RANGe`; served at `GET /instruments/{CLS}/{id}` and `/methods`.
- **Operations**: `GET .../{ch}/{op}` returns `{value}`; `POST .../{ch}/{op}/{value}` returns
  204 and is recorded in `api.posts`. `api.failures.set(path, {status, detail})` makes an
  operation fail.
- **WebSocket**: `/ws/registry` sends the registry every 400 ms. In the page,
  `window.__pushRegistry(data)` replaces it and `window.__registryPaused = true` stops it.
- The fixture pre-accepts the safety disclaimer.

## Writing tests

```typescript
import { test, expect } from './fixtures/base';
import { PANELS } from './utils/api-mock';

test('sets the voltage', async ({ mockPage, api }) => {
  const v = mockPage.getByRole('article', { name: PANELS.psu }).getByRole('spinbutton', { name: 'Voltage setpoint' });
  await v.focus();
  await v.press('ArrowUp');
  await v.press('Enter');
  await expect.poll(() => api.posts.map((p) => p.path)).toEqual(['/instruments/PSU/test-psu-1/1/voltage/6']);
});
```

- Use `api` without `mockPage` to change the mock (e.g. `api.instruments = []`) or add
  `page.route` overrides before `page.goto('/ui/')`; routes added later win.
- Prefer roles and labels (`getByRole('article', { name })`, `spinbutton`, `slider`, `img`).
- No fixed sleeps: use web-first assertions and `expect.poll`.

## Known UI bugs (`test.fixme`)

- Empty bench "Add instrument" opens the drawer without the new form (config load race).
- Switching the Instruments drawer tab discards an unsaved new instrument.
- The "Offline - reconnecting" badge blocks the arrange controls of a narrow offline panel.
