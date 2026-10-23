/**
 * What a BenchMesh distribution may add to the UI (contract `ai_context/OPEN_CORE.md`). The
 * open-source build adds nothing (`index.ts`); the official distribution replaces `index.ts` at
 * build time with its own (app updates). This file stays: it is the shape both agree on.
 */
import type { ComponentType } from 'react'

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

export interface UiExtensions {
  /** Rendered once in the main window, above the bench (dialogs such as an update offer). */
  root: ComponentType[]
  /** Groups at the end of Instruments - System. */
  systemGroups: ComponentType<SystemGroupProps>[]
}
