/**
 * Saving a file the user asked for (a recording's CSV, a flow's JSON, the console log). In the
 * desktop app the system's Save dialog asks where (`save_file`, crates/benchmesh-app/src/save.rs):
 * the webview would otherwise drop the file into Downloads without a word. A browser (remote
 * access, dev server) downloads it as usual.
 */
import { invoker } from './desktop'

/** What became of the file: saved at `path` (desktop app), handed to the browser, or cancelled. */
export type Saved = { path: string } | { downloaded: string } | null

/** The file's contents, or the address the browser downloads it from (fetched in the desktop app). */
export type Source = { text: string; type?: string } | { url: string }

export async function saveFile(name: string, source: Source): Promise<Saved> {
  const invoke = invoker()
  if (invoke) {
    const contents = 'text' in source ? source.text : await fetchText(source.url)
    const path = (await invoke('save_file', { name, contents })) as string | null
    return path ? { path } : null
  }
  const href = 'url' in source ? source.url : URL.createObjectURL(new Blob([source.text], { type: source.type ?? 'text/plain' }))
  const a = document.createElement('a')
  a.href = href
  a.download = name
  a.click()
  if (!('url' in source)) setTimeout(() => URL.revokeObjectURL(href), 1000)
  return { downloaded: name }
}

/** The line a view shows after saving (nothing when cancelled). */
export function savedText(saved: Saved): string | null {
  if (!saved) return null
  return 'path' in saved ? `Saved to ${saved.path}` : `Downloaded ${saved.downloaded}`
}

async function fetchText(url: string): Promise<string> {
  const r = await fetch(url)
  if (!r.ok) throw new Error(`HTTP ${r.status}`)
  return r.text()
}
