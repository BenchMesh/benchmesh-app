/** Shapes of the automation API (contract `ai_context/AUTOMATION.md`, server `crates/bm-server/src/automation`). */

export const FLOW_SCHEMA = 'benchmesh.flow/v1'

export type Scalar = number | string | boolean

export interface Param {
  name: string
  value: Scalar
  unit?: string | null
  description?: string | null
  min?: number | null
  max?: number | null
}

/** One step of a flow, tagged by `do`. Kept loose: the editor edits any field, the server validates. */
export interface Step {
  do: StepType
  [field: string]: any
}

export type StepType = 'set' | 'read' | 'wait' | 'let' | 'if' | 'repeat' | 'sweep' | 'check' | 'log' | 'prompt' | 'record' | 'stop'

export interface FlowDoc {
  schema: string
  name: string
  description?: string
  params?: Param[]
  setup?: Step[]
  steps?: Step[]
  cleanup?: Step[]
  timeout_s?: number | null
  [field: string]: any
}

export interface FlowSummary { id: string; name: string; description: string; updated_at: number | string | null }

export interface Issue { path: string; level: 'error' | 'warning'; message: string }

export interface Energize { path: string; device: string; class: string; channel: number; op: string }

export interface Validation { ok: boolean; issues: Issue[]; energizes: Energize[] }

export interface SavedFlow { id: string; flow: FlowDoc; issues: Issue[] }

// ---- vocabulary ----------------------------------------------------------------------

export interface ValueSpec {
  type: 'number' | 'integer' | 'enum' | 'bool' | 'string' | string
  min?: number
  max?: number
  format?: string
  unit?: string
  values?: string[]
}

export interface VocabField { type: string; values?: string[]; description?: string }

export interface VocabOp { get: boolean; set: boolean; value?: ValueSpec; energizes?: boolean; description?: string }

export interface VocabClass { channels: number; fields: Record<string, VocabField>; operations: Record<string, VocabOp> }

export interface VocabDevice {
  id: string; name: string; vendor: string; model?: string | null; online: boolean
  classes: Record<string, VocabClass>
}

export interface Vocab { devices: VocabDevice[] }

// ---- runs ----------------------------------------------------------------------------

export type RunState = 'idle' | 'running' | 'waiting' | 'cleanup' | 'finished' | 'failed' | 'aborted'

export interface RunEvent { seq: number; t: number; level: 'info' | 'warn' | 'error'; path: string; msg: string }

export interface RunStatus {
  id?: number
  flow?: { id: string | null; name: string }
  state: RunState
  started_at?: number
  ended_at?: number | null
  path?: string
  vars?: Record<string, unknown>
  error?: string | null
  prompt?: string | null
  recording_id?: number | null
  last_seq?: number
  events: RunEvent[]
}

export const isActiveState = (s: RunState | undefined) => s === 'running' || s === 'waiting' || s === 'cleanup'

export interface RunRequest { flow_id?: string; flow?: FlowDoc; params?: Record<string, Scalar>; arm?: boolean }

// ---- assistant / MCP -----------------------------------------------------------------

export type Provider = 'anthropic' | 'openai' | 'openrouter' | 'custom'

export interface AssistantSettings {
  provider: Provider
  model: string
  models?: Record<string, string>
  base_url: string
  /** Per provider: a hint of the stored key (`sk-…a1b2`) or null. Never the key itself. */
  keys: Record<string, string | null>
  providers?: Provider[]
}

export interface AssistantUpdate { provider?: Provider; model?: string; base_url?: string; api_key?: string }

export interface ModelInfo { id: string; name?: string }

export interface AssistResult { flow: FlowDoc; explanation: string; issues: Issue[]; attempts?: number }

export type McpAccess = 'off' | 'read' | 'control'

export interface McpInfo { access: McpAccess; endpoint?: string; command?: string | null; args?: string[]; env?: Record<string, string> | null }
