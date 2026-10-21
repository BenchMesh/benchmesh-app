/**
 * Web pages BenchMesh links to. The desktop app's webview does not follow external links, so
 * there the app opens the page in the system browser by name (`open_page`, a fixed list in
 * crates/benchmesh-app/src/links.rs); a browser opens it in a new tab.
 */
import React from 'react'

export type Page = 'issues' | 'new-issue' | 'releases'

export const PAGE_URLS: Record<Page, string> = {
  issues: 'https://github.com/MarkoVcode/BenchMesh/issues',
  'new-issue': 'https://github.com/MarkoVcode/BenchMesh/issues/new',
  releases: 'https://github.com/MarkoVcode/BenchMesh/releases/latest',
}

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>

function invoker(): Invoke | null {
  const core = (window as unknown as { __TAURI__?: { core?: { invoke?: Invoke } } }).__TAURI__?.core
  return core?.invoke ? core.invoke.bind(core) : null
}

/** A link to `page`: the system browser in the desktop app, a new tab elsewhere. */
export function PageLink({ page, className, children }: { page: Page; className?: string; children: React.ReactNode }) {
  const onClick = (e: React.MouseEvent) => {
    const invoke = invoker()
    if (!invoke) return // the browser follows the link
    e.preventDefault()
    void invoke('open_page', { page }).catch(() => window.open(PAGE_URLS[page], '_blank', 'noopener'))
  }
  return <a href={PAGE_URLS[page]} target="_blank" rel="noreferrer" className={className} onClick={onClick}>{children}</a>
}
