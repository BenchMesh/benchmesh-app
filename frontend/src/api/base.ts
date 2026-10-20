/**
 * Where the BenchMesh API lives.
 *
 * The server serves this UI under /ui, so the API is the page's own origin. That works
 * for the desktop shells (UI loaded from the embedded server), remote browsers and
 * reverse proxies alike. During development the Vite dev server proxies the API paths
 * to the backend (vite.config.ts). `window.__BENCHMESH_API_BASE__` overrides the origin,
 * e.g. for a UI bundled into a shell that does not load it from the server.
 */
export function apiBaseUrl(): string {
  const override = (window as { __BENCHMESH_API_BASE__?: string }).__BENCHMESH_API_BASE__
  return (override ?? window.location.origin).replace(/\/+$/, '')
}

/** WebSocket URL for an API path, e.g. `wsUrl('/ws/registry')`. */
export function wsUrl(path: string): string {
  return apiBaseUrl().replace(/^http/, 'ws') + path
}
