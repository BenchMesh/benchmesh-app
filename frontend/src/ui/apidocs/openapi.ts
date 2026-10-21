/**
 * The server's OpenAPI document (`/openapi.json`) as the API reference drawer needs it: operations
 * grouped by tag, plus the pure helpers for sending one (URL, curl line, response text).
 * Extensions from the server: `x-websocket` (a stream, not a request) and `x-no-try` (why the
 * operation cannot be sent from here).
 */

export type Method = 'get' | 'post' | 'put' | 'patch' | 'delete'
const METHODS: Method[] = ['get', 'post', 'put', 'patch', 'delete']

export interface ApiOp {
  id: string
  method: Method
  path: string
  tag: string
  summary: string
  params: string[]
  hasBody: boolean
  websocket: boolean
  noTry?: string
}

export interface ApiGroup { tag: string; description?: string; ops: ApiOp[] }

export interface ApiDoc { title: string; version: string; description: string; groups: ApiGroup[] }

const obj = (v: unknown): Record<string, any> | null => (v && typeof v === 'object' && !Array.isArray(v) ? v as Record<string, any> : null)

const pathParams = (path: string) => [...path.matchAll(/\{([^}]+)\}/g)].map((m) => m[1])

export function parseSpec(spec: unknown): ApiDoc {
  const s = obj(spec)
  const paths = obj(s?.paths)
  if (!s || !paths) throw new Error('The server did not return an OpenAPI document (not an OpenAPI document)')
  const info = obj(s.info) ?? {}
  const tagOrder: { name: string; description?: string }[] = (Array.isArray(s.tags) ? s.tags : [])
    .map(obj).filter(Boolean).map((t: any) => ({ name: String(t.name), description: t.description }))

  const groups = new Map<string, ApiGroup>(tagOrder.map((t) => [t.name, { tag: t.name, description: t.description, ops: [] }]))
  for (const [path, item] of Object.entries(paths)) {
    for (const method of METHODS) {
      const op = obj(obj(item)?.[method])
      if (!op) continue
      const tag = Array.isArray(op.tags) && op.tags.length ? String(op.tags[0]) : 'other'
      const listed = (Array.isArray(op.parameters) ? op.parameters : []).filter((p: any) => p?.in === 'path').map((p: any) => String(p.name))
      if (!groups.has(tag)) groups.set(tag, { tag, ops: [] })
      groups.get(tag)!.ops.push({
        id: `${method} ${path}`,
        method,
        path,
        tag,
        summary: typeof op.summary === 'string' ? op.summary : '',
        params: listed.length ? listed : pathParams(path),
        hasBody: !!op.requestBody,
        websocket: op['x-websocket'] === true,
        noTry: typeof op['x-no-try'] === 'string' ? op['x-no-try'] : undefined,
      })
    }
  }
  return {
    title: typeof info.title === 'string' ? info.title : 'API',
    version: typeof info.version === 'string' ? info.version : '',
    description: typeof info.description === 'string' ? info.description : '',
    groups: [...groups.values()].filter((g) => g.ops.length),
  }
}

/** Groups with the operations whose method, path or summary contain `query`. */
export function filterGroups(groups: ApiGroup[], query: string): ApiGroup[] {
  const q = query.trim().toLowerCase()
  if (!q) return groups
  return groups
    .map((g) => ({ ...g, ops: g.ops.filter((o) => `${o.method} ${o.path} ${o.summary}`.toLowerCase().includes(q)) }))
    .filter((g) => g.ops.length)
}

/** The request path with its parameters filled in (encoded) and the query appended. */
export function buildUrl(path: string, values: Record<string, string>, query: string): { url: string | null; missing: string[] } {
  const missing = pathParams(path).filter((p) => !(values[p] ?? '').trim())
  if (missing.length) return { url: null, missing }
  const filled = path.replace(/\{([^}]+)\}/g, (_, p: string) => encodeURIComponent(values[p].trim()))
  const q = query.trim().replace(/^\?/, '')
  return { url: q ? `${filled}?${q}` : filled, missing: [] }
}

const quote = (s: string) => `'${s.replace(/'/g, `'\\''`)}'`

export function curlFor(origin: string, method: Method, url: string, body?: string): string {
  const parts = ['curl']
  if (method !== 'get') parts.push('-X', method.toUpperCase())
  parts.push(quote(origin + url))
  if (body !== undefined) parts.push('-H', quote('Content-Type: application/json'), '-d', quote(body))
  return parts.join(' ')
}

export const wsUrl = (origin: string, path: string) => origin.replace(/^http/, 'ws') + path

const MAX_BODY = 200_000

/** Response text for display: JSON pretty-printed, very long bodies cut. */
export function formatBody(text: string, contentType: string): string {
  let out = text
  if (/json/i.test(contentType) && text.trim()) {
    try { out = JSON.stringify(JSON.parse(text), null, 2) } catch { /* not valid JSON: show as sent */ }
  }
  return out.length > MAX_BODY ? `${out.slice(0, MAX_BODY)}\n… (cut: ${out.length} characters in total)` : out
}
