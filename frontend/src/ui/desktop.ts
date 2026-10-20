/**
 * Window handling for detached instrument panels. In the desktop app (Tauri, `withGlobalTauri`)
 * panels open in undecorated windows that can be dragged, resized, folded and kept on top; in a
 * browser they fall back to popup windows (no always-on-top there).
 */

type TauriWindow = {
  label: string
  setFocus(): Promise<void>
  close(): Promise<void>
  startDragging(): Promise<void>
  startResizeDragging(direction: string): Promise<void>
  setAlwaysOnTop(on: boolean): Promise<void>
  setSize(size: unknown): Promise<void>
  once(event: string, cb: (e: { payload: unknown }) => void): Promise<unknown>
}

type TauriGlobal = {
  window: { getCurrentWindow(): TauriWindow }
  webviewWindow: {
    WebviewWindow: { new (label: string, options: Record<string, unknown>): TauriWindow; getByLabel(label: string): Promise<TauriWindow | null> }
  }
  dpi: { LogicalSize: new (w: number, h: number) => unknown }
}

function tauri(): TauriGlobal | null {
  const t = (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__
  return t?.window && t?.webviewWindow ? t : null
}

/** Running inside the BenchMesh desktop app. */
export const isDesktop = () => tauri() != null

/** Window label for a panel key (Tauri labels: letters, digits, `-`, `_`). */
export const panelLabel = (key: string) => `panel-${key.replace(/[^A-Za-z0-9_-]/g, '_')}`

export const panelUrl = (key: string) => `${window.location.origin}/ui/?panel=${encodeURIComponent(key)}`

const popups = new Map<string, Window>()

/** Open (or focus) the panel's own window, sized like the panel was on the bench. */
export async function openPanelWindow(key: string, title: string, width: number, height: number): Promise<void> {
  const label = panelLabel(key)
  // a little slack: at its own width the panel may wrap slightly differently than on the bench
  const w = Math.round(Math.max(280, width + 4))
  const h = Math.round(Math.max(80, height + 12))
  const t = tauri()
  if (t) {
    const existing = await t.webviewWindow.WebviewWindow.getByLabel(label)
    if (existing) { await existing.setFocus(); return }
    const win = new t.webviewWindow.WebviewWindow(label, {
      url: panelUrl(key), title, width: w, height: h, minWidth: 280, minHeight: 40,
      decorations: false, resizable: true, focus: true,
    })
    await new Promise<void>((resolve, reject) => {
      win.once('tauri://created', () => resolve())
      win.once('tauri://error', (e) => reject(new Error(String(e.payload))))
    })
    return
  }
  const popup = window.open(panelUrl(key), label, `popup,width=${w},height=${h}`)
  if (popup) popups.set(key, popup)
}

/** Bring a detached panel's window to the front. */
export async function focusPanelWindow(key: string) {
  const t = tauri()
  if (t) { await (await t.webviewWindow.WebviewWindow.getByLabel(panelLabel(key)))?.setFocus(); return }
  popups.get(key)?.focus()
}

/** Close a detached panel's window (the panel returns to the bench). */
export async function closePanelWindow(key: string) {
  const t = tauri()
  if (t) { await (await t.webviewWindow.WebviewWindow.getByLabel(panelLabel(key)))?.close(); return }
  popups.get(key)?.close()
  popups.delete(key)
}

// ---- the current (detached) window ----------------------------------------------------------

export function startWindowDrag() {
  void tauri()?.window.getCurrentWindow().startDragging()
}

export function startWindowResize() {
  void tauri()?.window.getCurrentWindow().startResizeDragging('SouthEast')
}

export async function setWindowOnTop(on: boolean) {
  await tauri()?.window.getCurrentWindow().setAlwaysOnTop(on)
}

export async function resizeWindow(width: number, height: number) {
  const t = tauri()
  if (t) { await t.window.getCurrentWindow().setSize(new t.dpi.LogicalSize(Math.round(width), Math.round(height))); return }
  window.resizeTo(Math.round(width + window.outerWidth - window.innerWidth), Math.round(height + window.outerHeight - window.innerHeight))
}

export function closeWindow() {
  const t = tauri()
  if (t) void t.window.getCurrentWindow().close()
  else window.close()
}
