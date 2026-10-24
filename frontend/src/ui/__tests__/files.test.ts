import { afterEach, describe, expect, it, vi } from 'vitest'
import { saveFile, savedText } from '../files'

type W = { __TAURI__?: unknown }

afterEach(() => {
  delete (window as W).__TAURI__
  vi.restoreAllMocks()
})

function desktop(answer: string | null) {
  const invoke = vi.fn((_cmd: string, _args?: Record<string, unknown>) => Promise.resolve(answer))
  ;(window as W).__TAURI__ = { core: { invoke } }
  return invoke
}

describe('saving a file', () => {
  it('in the desktop app asks where with the Save dialog and says where it went', async () => {
    const invoke = desktop('/home/me/Load test.csv')
    const saved = await saveFile('Load test.csv', { text: 'a,b\n1,2\n' })
    expect(invoke).toHaveBeenCalledWith('save_file', { name: 'Load test.csv', contents: 'a,b\n1,2\n' })
    expect(savedText(saved)).toBe('Saved to /home/me/Load test.csv')
  })

  it('in the desktop app fetches a download address first', async () => {
    const invoke = desktop('/tmp/x.csv')
    const fetch = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('time_utc\n'))
    await saveFile('x.csv', { url: '/recordings/1/export' })
    expect(fetch).toHaveBeenCalledWith('/recordings/1/export')
    expect(invoke).toHaveBeenCalledWith('save_file', { name: 'x.csv', contents: 'time_utc\n' })
  })

  it('reports a failed fetch and saves nothing', async () => {
    const invoke = desktop('/tmp/x.csv')
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 404 }))
    await expect(saveFile('x.csv', { url: '/recordings/9/export' })).rejects.toThrow('HTTP 404')
    expect(invoke).not.toHaveBeenCalled()
  })

  it('is nothing when the user cancels the dialog', async () => {
    desktop(null)
    const saved = await saveFile('flow.json', { text: '{}' })
    expect(saved).toBeNull()
    expect(savedText(saved)).toBeNull()
  })

  it('in a browser downloads by address or from the text', async () => {
    const anchors: HTMLAnchorElement[] = []
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) { anchors.push(this) })
    Object.assign(URL, { createObjectURL: vi.fn(() => 'blob:x'), revokeObjectURL: vi.fn() })
    expect(savedText(await saveFile('Bench run.csv', { url: '/recordings/1/export' }))).toBe('Downloaded Bench run.csv')
    await saveFile('flow.json', { text: '{}', type: 'application/json' })
    expect(anchors.map((a) => [a.getAttribute('href'), a.download])).toEqual([['/recordings/1/export', 'Bench run.csv'], ['blob:x', 'flow.json']])
  })
})
