/**
 * What a BenchMesh distribution may add to the UI (contract `ai_context/OPEN_CORE.md`). The
 * open-source build adds nothing (`index.ts`); the official distribution replaces `index.ts` at
 * build time with its own (app updates). This file stays: it is the shape both agree on.
 */
import type { ComponentType } from 'react'
import type { ChartSeries } from '../ui/recording/chartModel'
import type { Recording } from '../ui/recording/types'

/** BenchMesh is about to stop and start again; the System page waits and reloads. */
export interface Stopping {
  /** e.g. "Installing the update…" */
  title: string
  /** what happens meanwhile */
  hint: string
}

export interface SystemGroupProps {
  /** a view-only remote session: show, change nothing */
  viewOnly: boolean
  /** the group has asked BenchMesh to stop and start again */
  onStopping: (what: Stopping) => void
}

/** What a recording view draws: the same data the chart gets (recorder dock and Recordings drawer). */
export interface RecordingViewProps {
  recording: Recording
  /** every series with its colour, label, unit and points (`null` breaks a gap) */
  series: ChartSeries[]
  /** series the user switched off in the legend */
  hidden: Set<number>
  /** time range to show (ms); undefined: the data's own */
  min?: number
  max?: number
  /** the dock's live view (new samples keep arriving), not the history */
  live: boolean
  /** the view now shows this time range (ms): the drawer loads finer data for it */
  onRange?: (from: number, to: number) => void
}

/** Another way to draw a recording, offered next to the chart. */
export interface RecordingView {
  /** stable id, remembered as the user's choice */
  id: string
  /** switch label, e.g. "Trends" */
  label: string
  /** switch tooltip */
  title?: string
  component: ComponentType<RecordingViewProps>
}

export interface UiExtensions {
  /** Rendered once in the main window, above the bench (dialogs such as an update offer). */
  root: ComponentType[]
  /** Groups at the end of Instruments - System. */
  systemGroups: ComponentType<SystemGroupProps>[]
  /** Views of a recording besides the chart; with none, the dock and the drawer show no switch. */
  recordingViews: RecordingView[]
}
