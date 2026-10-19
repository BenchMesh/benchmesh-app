/**
 * REST client of the automation API. Errors reject with `ApiError` carrying the server's
 * `detail`, the status and the whole body (409 `energizes`, 422 `issues`).
 */
import { apiBaseUrl } from '../../api/base'
import type {
  AssistantSettings, AssistantUpdate, AssistResult, Energize, FlowDoc, FlowSummary, Issue, McpAccess, McpInfo,
  ModelInfo, RunRequest, RunStatus, SavedFlow, Validation, Vocab,
} from './types'

export class ApiError extends Error {
  constructor(message: string, readonly status: number, readonly body: any = null) { super(message) }
  get issues(): Issue[] { return Array.isArray(this.body?.issues) ? this.body.issues : [] }
  get energizes(): Energize[] | null { return Array.isArray(this.body?.energizes) ? this.body.energizes : null }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBaseUrl()}${path}`, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!r.ok) {
    let detail = `HTTP ${r.status}`
    let j: any = null
    try {
      j = await r.json()
      if (typeof j?.detail === 'string') detail = j.detail
    } catch { /* not JSON */ }
    throw new ApiError(detail, r.status, j)
  }
  if (r.status === 204) return undefined as T
  return r.json() as Promise<T>
}

export const automationClient = {
  vocabulary: () => request<Vocab>('GET', '/automation/vocabulary'),
  listFlows: () => request<FlowSummary[]>('GET', '/automation/flows'),
  getFlow: (id: string) => request<FlowDoc>('GET', `/automation/flows/${encodeURIComponent(id)}`),
  createFlow: (flow: FlowDoc) => request<SavedFlow>('POST', '/automation/flows', flow),
  updateFlow: (id: string, flow: FlowDoc) => request<SavedFlow>('PUT', `/automation/flows/${encodeURIComponent(id)}`, flow),
  deleteFlow: (id: string) => request<void>('DELETE', `/automation/flows/${encodeURIComponent(id)}`),
  validate: (flow: FlowDoc) => request<Validation>('POST', '/automation/validate', flow),
  run: (req: RunRequest) => request<RunStatus>('POST', '/automation/run', req),
  runStatus: (after = 0) => request<RunStatus>('GET', `/automation/run${after > 0 ? `?after=${after}` : ''}`),
  abort: () => request<void>('POST', '/automation/run/abort'),
  resume: () => request<void>('POST', '/automation/run/continue'),
  assistant: () => request<AssistantSettings>('GET', '/automation/assistant'),
  updateAssistant: (u: AssistantUpdate) => request<AssistantSettings>('PUT', '/automation/assistant', u),
  models: () => request<ModelInfo[]>('GET', '/automation/assistant/models'),
  assist: (instruction: string, flow?: FlowDoc) => request<AssistResult>('POST', '/automation/assist', flow ? { instruction, flow } : { instruction }),
  mcp: () => request<McpInfo>('GET', '/automation/mcp'),
  setMcp: (access: McpAccess) => request<{ access: McpAccess }>('PUT', '/automation/mcp', { access }),
}

export type AutomationClient = typeof automationClient

export const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))
