/**
 * Right-click belongs to the app (add a reading to a recording, kit `ContextMenu`): the browser's
 * or webview's own context menu (Back, Reload, Inspect…) is never shown, on any page or window.
 * Captured on `window` so it applies before any element handler; the app's own menus still open.
 */
export function suppressNativeContextMenu(target: Window = window): () => void {
  const block = (e: Event) => e.preventDefault()
  target.addEventListener('contextmenu', block, { capture: true })
  return () => target.removeEventListener('contextmenu', block, { capture: true })
}
