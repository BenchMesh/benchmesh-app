/**
 * Instrument console (`GET /devices/{id}/wire`): what went to and came from an instrument.
 * Pure helpers - decoding the API's Latin-1 byte strings, showing bytes as text or hex, joining
 * the chunks of one reply, and the evidence text users copy into a bug report or forum post.
 */

export type WireDir = 'tx' | 'rx' | 'info'

/** As the API sends it: `data` is one char per byte for tx/rx, plain text for info. */
export interface WireEntry { seq: number; t: number; dir: WireDir; data: string; len: number }

/** One console line: a write, a whole reply (its chunks joined) or a port event. */
export interface WireLine { seq: number; t: number; dir: WireDir; bytes: Uint8Array; len: number; text?: string }

export type WireView = 'auto' | 'text' | 'hex'

export function bytesOf(data: string): Uint8Array {
  const b = new Uint8Array(data.length)
  for (let i = 0; i < data.length; i++) b[i] = data.charCodeAt(i) & 0xff
  return b
}

/** Replies split over several reads become one line, as long as nothing was sent in between. */
export function appendEntries(lines: WireLine[], entries: WireEntry[], max = 2000): WireLine[] {
  const out = lines.slice()
  for (const e of entries) {
    const last = out[out.length - 1]
    if (e.dir === 'rx' && last?.dir === 'rx') {
      const b = bytesOf(e.data)
      const joined = new Uint8Array(last.bytes.length + b.length)
      joined.set(last.bytes)
      joined.set(b, last.bytes.length)
      out[out.length - 1] = { ...last, seq: e.seq, bytes: joined, len: last.len + e.len }
      continue
    }
    out.push(e.dir === 'info'
      ? { seq: e.seq, t: e.t, dir: 'info', bytes: new Uint8Array(), len: 0, text: e.data }
      : { seq: e.seq, t: e.t, dir: e.dir, bytes: bytesOf(e.data), len: e.len })
  }
  return out.length > max ? out.slice(out.length - max) : out
}

const printable = (b: number) => (b >= 0x20 && b < 0x7f) || b === 0x09 || b === 0x0a || b === 0x0d

/** Mostly not text (binary protocols, waveform blocks): shown as hex in `auto`. */
export function isBinary(b: Uint8Array): boolean {
  if (!b.length) return false
  let n = 0
  for (const x of b) if (!printable(x)) n++
  return n / b.length > 0.1
}

export function hex(b: Uint8Array): string {
  return Array.from(b, (x) => x.toString(16).padStart(2, '0')).join(' ')
}

/** Text with control bytes made visible: `\r`, `\n`, `\t`, `\xNN`. */
export function escaped(b: Uint8Array): string {
  let s = ''
  for (const x of b) {
    if (x === 0x0d) s += '\\r'
    else if (x === 0x0a) s += '\\n'
    else if (x === 0x09) s += '\\t'
    else if (x === 0x5c) s += '\\\\'
    else if (x >= 0x20 && x < 0x7f) s += String.fromCharCode(x)
    else s += `\\x${x.toString(16).padStart(2, '0')}`
  }
  return s
}

/** How a line's bytes are shown (long binary payloads are shortened in `auto`). */
export function showBytes(line: WireLine, view: WireView): string {
  if (line.dir === 'info') return line.text ?? ''
  const binary = isBinary(line.bytes)
  let s: string
  if (view === 'hex' || (view === 'auto' && binary)) {
    const head = view === 'auto' && line.bytes.length > 64 ? line.bytes.slice(0, 64) : line.bytes
    // in `auto` a hex line says so: a status byte 0x11 must not read as the text "11"
    s = (view === 'auto' ? 'hex ' : '') + hex(head) + (head.length < line.bytes.length ? ' …' : '')
  } else {
    s = escaped(line.bytes)
  }
  return line.len > line.bytes.length ? `${s} … (${line.len} bytes)` : s
}

const ARROW: Record<WireDir, string> = { tx: '→', rx: '←', info: '•' }

export function clock(t: number): string {
  const d = new Date(t)
  const p = (n: number, w = 2) => String(n).padStart(w, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}.${p(d.getMilliseconds(), 3)}`
}

export function lineText(line: WireLine, view: WireView): string {
  return `${clock(line.t)} ${ARROW[line.dir]} ${showBytes(line, view)}`
}

export interface EvidenceMeta {
  instrument: string
  klass: string
  model?: string
  idn?: string
  support?: string
  app?: string
  link?: string
}

/** The console as plain text with a header - paste-ready for a bug report or forum post. */
export function evidence(meta: EvidenceMeta, lines: WireLine[], view: WireView): string {
  const head = [
    `BenchMesh instrument console - ${new Date().toISOString()}`,
    `Instrument: ${meta.instrument} (${meta.klass})${meta.model ? ` model ${meta.model}` : ''}${meta.support ? ` [${meta.support}]` : ''}`,
    meta.idn ? `Identify: ${meta.idn}` : null,
    meta.link ? `Link: ${meta.link}` : null,
    meta.app ? `App: ${meta.app}` : null,
    `Legend: → sent, ← received, • port event; bytes as text with \\r \\n \\xNN escapes${view === 'hex' ? ' (hex view)' : ''}`,
    '',
  ].filter((x) => x !== null)
  return [...head, ...lines.map((l) => lineText(l, view))].join('\n') + '\n'
}
