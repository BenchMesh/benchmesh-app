/**
 * How a recording is drawn: the chart, or a view a distribution adds (`extensions.recordingViews`,
 * contract `src/extensions/contract.ts`). With no added view there is nothing to choose and no
 * switch is shown. The choice is remembered per browser, for the dock and the drawer alike; a
 * view that fails falls back to the chart.
 */
import React, { Component, ReactNode, useState } from 'react'
import { Selector } from '../kit'
import { extensions } from '../../extensions'
import type { RecordingView, RecordingViewProps } from '../../extensions/contract'
import { LazyChart } from './charts'
import type { RecordingChartProps } from './RecordingChart'

export const VIEW_STORAGE_KEY = 'benchmesh:recording-view'
const CHART = 'chart'

function stored(): string {
  try { return localStorage.getItem(VIEW_STORAGE_KEY) ?? CHART } catch { return CHART }
}

export interface ViewChoice {
  views: RecordingView[]
  /** the chosen added view; null: the chart */
  view: RecordingView | null
  choose: (id: string) => void
}

/** The chosen view among those available (the chart when the remembered one is not there). */
export function useRecordingView(views: RecordingView[] = extensions.recordingViews): ViewChoice {
  const [id, setId] = useState(stored)
  const choose = (v: string) => {
    setId(v)
    try { localStorage.setItem(VIEW_STORAGE_KEY, v) } catch { /* storage unavailable */ }
  }
  return { views, view: views.find((v) => v.id === id) ?? null, choose }
}

/** Chart | <added views>; nothing when no view was added. */
export function ViewSwitch({ views, view, choose }: ViewChoice) {
  if (!views.length) return null
  return (
    <Selector label="Recording view" value={view?.id ?? CHART} onChange={choose}
      options={[{ value: CHART, label: 'Chart', title: 'Values on their own axes' }, ...views.map((v) => ({ value: v.id, label: v.label, title: v.title }))]} />
  )
}

/** The chosen view, or the chart. */
export function RecordingPlot({ view, chart, data }: { view: RecordingView | null; chart: RecordingChartProps; data: RecordingViewProps }) {
  const fallback = <LazyChart {...chart} />
  if (!view) return fallback
  const View = view.component
  return <ViewGuard key={view.id} fallback={fallback}><View {...data} /></ViewGuard>
}

class ViewGuard extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false }
  static getDerivedStateFromError() { return { failed: true } }
  componentDidCatch(e: unknown) { console.error('recording view failed, showing the chart', e) }
  render() { return this.state.failed ? this.props.fallback : this.props.children }
}
