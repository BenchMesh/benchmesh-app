/**
 * Bench layout: which instrument panels are shown, in what order and how wide. One panel
 * per instrument class; panels flow into a responsive grid whose column count follows the
 * screen width, so the same arrangement works on a laptop and on a wide monitor.
 */

export type PanelSize = 's' | 'm' | 'l' | 'full'

export interface BenchLayout {
  order: string[]
  size: Record<string, PanelSize>
  hidden: string[]
  /** Panels folded to their header line. */
  folded: string[]
}

export interface PanelRef {
  key: string
  instrumentId: string
  klass: string
}

export const EMPTY_LAYOUT: BenchLayout = { order: [], size: {}, hidden: [], folded: [] }

export const SIZES: { value: PanelSize; label: string; title: string }[] = [
  { value: 's', label: 'S', title: 'One column' },
  { value: 'm', label: 'M', title: 'Two columns' },
  { value: 'l', label: 'L', title: 'Three columns' },
  { value: 'full', label: 'W', title: 'Full width' },
]

const DEFAULT_SIZE: Record<string, PanelSize> = { PSU: 'm', DMM: 'm', ELL: 'm', AWG: 'full', OSC: 'full' }

export function defaultSize(klass: string): PanelSize {
  return DEFAULT_SIZE[klass.toUpperCase()] ?? 's'
}

export function panelKey(instrumentId: string, klass: string) {
  return `${instrumentId}:${klass}`
}

export function panelsOf(instruments: { id: string; classes?: { class: string }[] }[]): PanelRef[] {
  return instruments.flatMap((i) => (i.classes ?? []).map((c) => ({ key: panelKey(i.id, c.class), instrumentId: i.id, klass: c.class })))
}

/** Layout matching the current panels: known ones keep their place, new ones are appended. */
export function reconcile(layout: BenchLayout, panels: PanelRef[]): BenchLayout {
  const keys = new Set(panels.map((p) => p.key))
  const order = layout.order.filter((k) => keys.has(k))
  for (const p of panels) if (!order.includes(p.key)) order.push(p.key)
  const size: Record<string, PanelSize> = {}
  for (const p of panels) size[p.key] = layout.size[p.key] ?? defaultSize(p.klass)
  const hidden = layout.hidden.filter((k) => keys.has(k))
  const folded = (layout.folded ?? []).filter((k) => keys.has(k))
  const same = order.length === layout.order.length && order.every((k, i) => k === layout.order[i]) &&
    hidden.length === layout.hidden.length && Object.keys(layout.size).length === order.length &&
    order.every((k) => layout.size[k] === size[k]) && layout.folded != null && folded.length === layout.folded.length
  return same ? layout : { order, size, hidden, folded }
}

export function move(layout: BenchLayout, key: string, toIndex: number): BenchLayout {
  const order = layout.order.filter((k) => k !== key)
  order.splice(Math.max(0, Math.min(order.length, toIndex)), 0, key)
  return { ...layout, order }
}

export function setSize(layout: BenchLayout, key: string, size: PanelSize): BenchLayout {
  return { ...layout, size: { ...layout.size, [key]: size } }
}

export function hide(layout: BenchLayout, key: string): BenchLayout {
  return layout.hidden.includes(key) ? layout : { ...layout, hidden: [...layout.hidden, key] }
}

export function show(layout: BenchLayout, key: string): BenchLayout {
  return { ...layout, hidden: layout.hidden.filter((k) => k !== key) }
}

export function toggleFold(layout: BenchLayout, key: string): BenchLayout {
  const folded = layout.folded ?? []
  return { ...layout, folded: folded.includes(key) ? folded.filter((k) => k !== key) : [...folded, key] }
}

export const COLUMN_MIN_PX = 300
export const GAP_PX = 14

export function columnsFor(width: number): number {
  return Math.max(1, Math.floor((width + GAP_PX) / (COLUMN_MIN_PX + GAP_PX)))
}

export function spanFor(size: PanelSize, columns: number): number {
  const want = { s: 1, m: 2, l: 3, full: columns }[size]
  return Math.max(1, Math.min(columns, want))
}

const STORAGE_KEY = 'benchmesh:bench-layout'

export function loadLayout(): BenchLayout {
  try {
    const v = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null')
    if (v && Array.isArray(v.order) && Array.isArray(v.hidden) && v.size && typeof v.size === 'object') {
      return { ...v, folded: Array.isArray(v.folded) ? v.folded : [] }
    }
  } catch { /* unavailable or corrupt: start fresh */ }
  return EMPTY_LAYOUT
}

export function saveLayout(layout: BenchLayout) {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(layout)) } catch { /* storage unavailable */ }
}
