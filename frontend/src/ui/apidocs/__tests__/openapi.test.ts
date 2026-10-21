import { describe, it, expect } from 'vitest'
import { buildUrl, curlFor, filterGroups, formatBody, parseSpec, wsUrl } from '../openapi'

const spec = {
  openapi: '3.1.0',
  info: { title: 'BenchMesh API', version: '0.1.9', description: 'Lab instrument control.' },
  tags: [{ name: 'system', description: 'Service status' }, { name: 'instrument-control', description: 'Read and set' }, { name: 'websockets', description: 'Streams' }],
  paths: {
    '/status': { get: { tags: ['system'], summary: 'Device connection counts', parameters: [] } },
    '/': { get: { tags: ['system'], summary: 'Redirect to the UI', parameters: [], 'x-no-try': 'Redirects a browser to the UI' } },
    '/instruments/{klass}/{id}/{ch}/{op}': {
      get: { tags: ['instrument-control'], summary: 'Read an operation', parameters: [{ name: 'klass', in: 'path' }, { name: 'id', in: 'path' }, { name: 'ch', in: 'path' }, { name: 'op', in: 'path' }] },
    },
    '/recordings': { post: { tags: ['recordings'], summary: 'Create a recording', parameters: [], requestBody: { content: { 'application/json': {} } } } },
    '/ws/registry': { get: { tags: ['websockets'], summary: 'WebSocket: live registry', parameters: [], 'x-websocket': true } },
  },
}

describe('parseSpec', () => {
  it('groups operations by tag in the order of the spec tags, then untagged groups', () => {
    const doc = parseSpec(spec)
    expect(doc.title).toBe('BenchMesh API')
    expect(doc.version).toBe('0.1.9')
    expect(doc.groups.map((g) => g.tag)).toEqual(['system', 'instrument-control', 'websockets', 'recordings'])
    expect(doc.groups[0].description).toBe('Service status')
    const read = doc.groups[1].ops[0]
    expect(read).toMatchObject({ method: 'get', path: '/instruments/{klass}/{id}/{ch}/{op}', params: ['klass', 'id', 'ch', 'op'], hasBody: false })
    expect(doc.groups[3].ops[0]).toMatchObject({ method: 'post', hasBody: true })
    expect(doc.groups[2].ops[0].websocket).toBe(true)
    expect(doc.groups[0].ops.find((o) => o.path === '/')?.noTry).toBe('Redirects a browser to the UI')
  })

  it('takes path parameters from the path when the spec lists none and ignores unknown methods', () => {
    const doc = parseSpec({ paths: { '/a/{x}': { get: { summary: 's' }, parameters: [], trace: {} } } })
    expect(doc.groups).toHaveLength(1)
    expect(doc.groups[0].tag).toBe('other')
    expect(doc.groups[0].ops).toHaveLength(1)
    expect(doc.groups[0].ops[0].params).toEqual(['x'])
  })

  it('throws a readable error for something that is not an OpenAPI document', () => {
    expect(() => parseSpec(null)).toThrow('not an OpenAPI document')
    expect(() => parseSpec({ info: {} })).toThrow('not an OpenAPI document')
  })
})

describe('filterGroups', () => {
  it('matches path, summary and method, case-insensitively, and drops empty groups', () => {
    const { groups } = parseSpec(spec)
    expect(filterGroups(groups, '').length).toBe(4)
    const r = filterGroups(groups, 'REGISTRY')
    expect(r.map((g) => g.tag)).toEqual(['websockets'])
    expect(filterGroups(groups, 'post').flatMap((g) => g.ops.map((o) => o.path))).toEqual(['/recordings'])
    expect(filterGroups(groups, 'connection counts')[0].ops[0].path).toBe('/status')
    expect(filterGroups(groups, 'nothing like this')).toEqual([])
  })
})

describe('buildUrl', () => {
  it('fills and encodes path parameters and appends the query', () => {
    expect(buildUrl('/instruments/{klass}/{id}', { klass: 'PSU', id: 'a b/c' }, '')).toEqual({ url: '/instruments/PSU/a%20b%2Fc', missing: [] })
    expect(buildUrl('/recordings/{id}/data', { id: '3' }, '?from=1&to=2').url).toBe('/recordings/3/data?from=1&to=2')
    expect(buildUrl('/serial-ports', {}, ' exclude=a ').url).toBe('/serial-ports?exclude=a')
  })

  it('reports missing parameters (blank counts as missing)', () => {
    expect(buildUrl('/instruments/{klass}/{id}', { klass: ' ' }, '')).toEqual({ url: null, missing: ['klass', 'id'] })
  })
})

describe('curlFor and wsUrl', () => {
  it('builds a paste-ready curl line, with a JSON body when there is one', () => {
    expect(curlFor('http://127.0.0.1:57666', 'get', '/status')).toBe("curl 'http://127.0.0.1:57666/status'")
    expect(curlFor('http://127.0.0.1:57666', 'post', '/recordings', '{"name":"it\'s"}'))
      .toBe(`curl -X POST 'http://127.0.0.1:57666/recordings' -H 'Content-Type: application/json' -d '{"name":"it'\\''s"}'`)
    expect(curlFor('http://h', 'delete', '/recordings/1')).toBe("curl -X DELETE 'http://h/recordings/1'")
  })

  it('turns the page origin into the WebSocket address', () => {
    expect(wsUrl('http://127.0.0.1:57666', '/ws/registry')).toBe('ws://127.0.0.1:57666/ws/registry')
    expect(wsUrl('https://192.168.1.5:57667', '/ws/metrics')).toBe('wss://192.168.1.5:57667/ws/metrics')
  })
})

describe('formatBody', () => {
  it('pretty-prints JSON and leaves other text as it is', () => {
    expect(formatBody('{"a":1}', 'application/json')).toBe('{\n  "a": 1\n}')
    expect(formatBody('{"a":', 'application/json')).toBe('{"a":')
    expect(formatBody('<html>', 'text/html')).toBe('<html>')
    expect(formatBody('', 'application/json')).toBe('')
  })

  it('cuts very long bodies', () => {
    const long = 'x'.repeat(300_000)
    const out = formatBody(long, 'text/plain')
    expect(out.length).toBeLessThan(210_000)
    expect(out.endsWith('(cut: 300000 characters in total)')).toBe(true)
  })
})
