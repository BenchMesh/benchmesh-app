import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { appendEntries, bytesOf, escaped, evidence, isBinary, lineText, showBytes, type WireEntry } from '../wire'
import { WireConsole, CONSOLE_LINES, logFileName } from '../WireConsole'

const t0 = new Date(2026, 9, 18, 21, 5, 7, 42).getTime()
const e = (seq: number, dir: WireEntry['dir'], data: string): WireEntry => ({ seq, t: t0 + seq, dir, data, len: data.length })

describe('wire console logic', () => {
  it('joins the chunks of one reply, not across a write', () => {
    const lines = appendEntries([], [e(1, 'tx', '*IDN?\n'), e(2, 'rx', 'TENMA 72-'), e(3, 'rx', '2540 V2.1'), e(4, 'tx', 'VOUT1?'), e(5, 'rx', '05.00')])
    expect(lines.map((l) => [l.dir, escaped(l.bytes)])).toEqual([['tx', '*IDN?\\n'], ['rx', 'TENMA 72-2540 V2.1'], ['tx', 'VOUT1?'], ['rx', '05.00']])
    expect(lines[1].t).toBe(t0 + 2) // the time of the reply's first byte
  })

  it('shows control bytes as escapes and binary protocols as hex', () => {
    const modbus = e(1, 'tx', '\x01\x03\x00\x00\x00\x01\x84\x0a')
    const [line] = appendEntries([], [modbus])
    expect(isBinary(line.bytes)).toBe(true)
    expect(showBytes(line, 'auto')).toBe('hex 01 03 00 00 00 01 84 0a')
    expect(showBytes(line, 'hex')).toBe('01 03 00 00 00 01 84 0a')
    expect(showBytes(line, 'text')).toBe('\\x01\\x03\\x00\\x00\\x00\\x01\\x84\\n')
    expect(lineText(appendEntries([], [e(2, 'tx', ':MEAS:VOLT?\r\n')])[0], 'auto')).toBe('21:05:07.044 → :MEAS:VOLT?\\r\\n')
    expect(lineText(appendEntries([], [e(3, 'info', 'opened /dev/ttyUSB0')])[0], 'hex')).toBe('21:05:07.045 • opened /dev/ttyUSB0')
  })

  it('marks cut payloads with their real size', () => {
    const [line] = appendEntries([], [{ seq: 1, t: t0, dir: 'rx', data: '#9000010000' + 'x'.repeat(10), len: 10011 }])
    expect(showBytes(line, 'text')).toMatch(/… \(10011 bytes\)$/)
  })

  it('evidence starts with what a reader needs to reproduce', () => {
    const lines = appendEntries([], [e(1, 'tx', '*IDN?\n'), e(2, 'rx', 'RIGOL TECHNOLOGIES,DHO804,X,1\n')])
    const text = evidence({ instrument: 'scope', klass: 'OSC', model: 'DHO804', idn: 'RIGOL TECHNOLOGIES,DHO804,X,1', support: 'tested', link: 'tcpip 192.168.1.9', app: 'BenchMesh 0.1.5' }, lines, 'auto')
    const head = text.split('\n')
    expect(head[1]).toBe('Instrument: scope (OSC) model DHO804 [tested]')
    expect(text).toContain('Link: tcpip 192.168.1.9')
    expect(text).toContain('App: BenchMesh 0.1.5')
    expect(text.trim().split('\n').slice(-2)).toEqual(['21:05:07.043 → *IDN?\\n', '21:05:07.044 ← RIGOL TECHNOLOGIES,DHO804,X,1\\n'])
  })

  it('decodes the API Latin-1 byte strings', () => {
    expect(Array.from(bytesOf('\xff\x00A'))).toEqual([255, 0, 65])
  })
})

describe('WireConsole', () => {
  afterEach(() => vi.restoreAllMocks())

  it('shows the traffic and copies it as evidence', async () => {
    const replies = [{ entries: [e(1, 'info', 'opened /dev/ttyUSB0'), e(2, 'tx', '*IDN?'), e(3, 'rx', 'TENMA 72-2540 V2.1')], next: 3, dropped: false }]
    vi.spyOn(globalThis, 'fetch').mockImplementation((url) => {
      const u = String(url)
      const body = u.endsWith('/version') ? { releaseVersion: '0.1.5', displayVersion: 'BenchMesh 0.1.5', distribution: 'official' } : (replies.shift() ?? { entries: [], next: 3, dropped: false })
      return Promise.resolve(new Response(JSON.stringify(body), { status: 200 }))
    })
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<WireConsole meta={{ instrument: 't1', klass: 'PSU', model: '72-2540' }} />)
    await screen.findByText(/← TENMA 72-2540 V2.1/)
    expect(screen.getByText(/→ \*IDN\?/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Copy' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce())
    const copied = writeText.mock.calls[0][0] as string
    expect(copied).toContain('Instrument: t1 (PSU) model 72-2540')
    expect(copied).toContain('App: BenchMesh 0.1.5')
    expect(copied).toContain('← TENMA 72-2540 V2.1')
    expect(await screen.findByRole('status')).toHaveTextContent('Copied 3 lines')
  })

  it('asks for the newest lines first, then only what is new', async () => {
    const urls: string[] = []
    vi.spyOn(globalThis, 'fetch').mockImplementation((url) => {
      const u = String(url)
      urls.push(u)
      const body = u.endsWith('/version') ? { version: '0.1.7' } : { entries: urls.length < 3 ? [e(7, 'tx', '*IDN?')] : [], next: 7, dropped: true }
      return Promise.resolve(new Response(JSON.stringify(body), { status: 200 }))
    })
    render(<WireConsole meta={{ instrument: 'rigol', klass: 'OSC' }} />)
    await screen.findByText(/earlier entries are not shown/)
    const wire = urls.filter((u) => u.includes('/wire'))
    expect(wire[0]).toMatch(new RegExp(`/devices/rigol/wire\\?limit=${CONSOLE_LINES}$`))
    await waitFor(() => expect(urls.filter((u) => u.includes('/wire?after=7')).length).toBeGreaterThan(0))
  })

  it('saves everything the server kept as a text file', async () => {
    const urls: string[] = []
    vi.spyOn(globalThis, 'fetch').mockImplementation((url) => {
      const u = String(url)
      urls.push(u)
      const all = { entries: [e(1, 'info', 'opened /dev/usbtmc5'), e(2, 'tx', '*IDN?'), e(3, 'info', 'write failed (timeout), USB recovery: device clear ok')], next: 3, dropped: false }
      const body = u.endsWith('/version') ? { version: '0.1.7' } : u.includes('after=0') ? all : { entries: [], next: 3, dropped: false }
      return Promise.resolve(new Response(JSON.stringify(body), { status: 200 }))
    })
    let saved: Blob | null = null
    const create = vi.fn((b: Blob) => { saved = b; return 'blob:x' })
    Object.assign(URL, { createObjectURL: create, revokeObjectURL: vi.fn() })
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {})
    render(<WireConsole meta={{ instrument: 'rigol', klass: 'OSC', model: 'DHO804' }} />)
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByRole('status')).toHaveTextContent(/^3 lines - Downloaded benchmesh-console-rigol-.*\.txt$/)
    expect(urls.some((u) => u.endsWith('/devices/rigol/wire?after=0'))).toBe(true)
    expect(click).toHaveBeenCalledOnce()
    const text = await new Promise<string>((done) => {
      const r = new FileReader()
      r.onload = () => done(String(r.result))
      r.readAsText(saved!)
    })
    expect(text).toContain('Instrument: rigol (OSC) model DHO804')
    expect(text).toContain('USB recovery: device clear ok')
  })

  it('names the saved file after the instrument and the time', () => {
    expect(logFileName('rigol dho/804', new Date('2026-10-21T00:12:03Z'))).toBe('benchmesh-console-rigol_dho_804-2026-10-21T00-12-03.txt')
  })
})
