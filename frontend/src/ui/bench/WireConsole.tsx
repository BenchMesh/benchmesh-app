/**
 * Instrument console: an optional pane at the bottom of an instrument panel that shows what
 * goes to and comes from the instrument (`GET /devices/{id}/wire`, polled while open), with
 * pause, clear, text / hex view, Copy (paste-ready evidence for troubleshooting) and Save
 * (everything the server kept, about the last 20 minutes or more, as a text file).
 */
import React, { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { apiBaseUrl } from '../../api/base'
import { Selector } from '../kit'
import { fetchVersion } from '../about/version'
import { saveFile, savedText } from '../files'
import { appendEntries, evidence, lineText, type EvidenceMeta, type WireEntry, type WireLine, type WireView } from './wire'

const POLL_MS = 500
/** Lines the console shows; Save writes everything the server kept. */
export const CONSOLE_LINES = 5000

type WireReply = { entries: WireEntry[]; next: number; dropped: boolean }

const wirePath = (id: string) => `${apiBaseUrl()}/devices/${encodeURIComponent(id)}/wire`

/** The console's lines while `enabled` (polling paused with `paused`). */
export function useWireLog(id: string, enabled: boolean, paused: boolean) {
  const [lines, setLines] = useState<WireLine[]>([])
  const after = useRef(0)
  useEffect(() => {
    if (!enabled || paused) return
    let live = true
    const tick = async () => {
      try {
        // the first request takes the newest entries only, later ones whatever is new
        const q = after.current ? `after=${after.current}` : `limit=${CONSOLE_LINES}`
        const r = await fetch(`${wirePath(id)}?${q}`)
        if (!r.ok || !live) return
        const v: WireReply = await r.json()
        if (!live) return
        after.current = v.next
        if (v.entries.length || v.dropped) {
          const gap: WireEntry[] = v.dropped ? [{ seq: 0, t: Date.now(), dir: 'info', data: `earlier entries are not shown (the console shows the last ${CONSOLE_LINES} lines; Save writes everything kept)`, len: 0 }] : []
          setLines((old) => appendEntries(old, [...gap, ...v.entries], CONSOLE_LINES))
        }
      } catch { /* server unreachable: the panel shows offline */ }
    }
    void tick()
    const t = setInterval(tick, POLL_MS)
    return () => { live = false; clearInterval(t) }
  }, [id, enabled, paused])
  const clear = async () => {
    setLines([])
    try { await fetch(wirePath(id), { method: 'DELETE' }) } catch { /* local clear is enough */ }
  }
  return { lines, clear }
}

/** Everything the server kept for `id`, as console lines. */
export async function fullLog(id: string): Promise<WireLine[]> {
  const r = await fetch(`${wirePath(id)}?after=0`)
  if (!r.ok) throw new Error(`HTTP ${r.status}`)
  const v: WireReply = await r.json()
  return appendEntries([], v.entries, Number.POSITIVE_INFINITY)
}

/** A file name such as `benchmesh-console-rigol-dho804-2026-10-21T00-12-03.txt`. */
export function logFileName(instrument: string, at: Date): string {
  const stamp = at.toISOString().slice(0, 19).replace(/:/g, '-')
  return `benchmesh-console-${instrument.replace(/[^\w.-]+/g, '_')}-${stamp}.txt`
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // older webviews: a hidden textarea and execCommand
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand?.('copy') ?? false
    ta.remove()
    return ok
  }
}

const VIEWS: { value: WireView; label: string; title: string }[] = [
  { value: 'auto', label: 'Auto', title: 'Text, binary as hex' },
  { value: 'text', label: 'Text', title: 'Everything as text (\\r \\n \\xNN escapes)' },
  { value: 'hex', label: 'Hex', title: 'Everything as hex bytes' },
]

export function WireConsole({ meta }: { meta: EvidenceMeta }) {
  const [paused, setPaused] = useState(false)
  const [view, setView] = useState<WireView>('auto')
  const [copied, setCopied] = useState<string | null>(null)
  const { lines, clear } = useWireLog(meta.instrument, true, paused)
  const [app, setApp] = useState<string | undefined>(meta.app)
  const box = useRef<HTMLDivElement>(null)
  const stick = useRef(true)

  useEffect(() => {
    let live = true
    void fetchVersion().then((v) => live && v && setApp(v.display))
    return () => { live = false }
  }, [])
  // follow new lines unless the user scrolled up to read
  useLayoutEffect(() => {
    const el = box.current
    if (el && stick.current) el.scrollTop = el.scrollHeight
  }, [lines])

  const copy = async () => {
    const ok = await copyText(evidence({ ...meta, app }, lines, view))
    setCopied(ok ? `Copied ${lines.length} lines` : 'Copy failed - select the text instead')
    setTimeout(() => setCopied(null), 2500)
  }

  const save = async () => {
    try {
      const all = await fullLog(meta.instrument)
      const saved = await saveFile(logFileName(meta.instrument, new Date()), { text: evidence({ ...meta, app }, all, view) })
      if (!saved) return
      setCopied(`${all.length} lines - ${savedText(saved)}`)
    } catch (e) {
      setCopied(`Save failed: ${e instanceof Error ? e.message : e}`)
    }
    setTimeout(() => setCopied(null), 6000)
  }

  return (
    <section className="wire-console" aria-label="Instrument console" data-testid="wire-console">
      <div className="wire-console__bar">
        <span className="kit-label">Console</span>
        <Selector label="View" value={view} options={VIEWS} onChange={(v) => setView(v as WireView)} />
        <button type="button" className="bench-icon wire-console__btn" aria-pressed={paused} onClick={() => setPaused((p) => !p)}
          title={paused ? 'Resume' : 'Pause (keeps what is shown)'}>{paused ? 'Resume' : 'Pause'}</button>
        <button type="button" className="bench-icon wire-console__btn" onClick={() => void clear()} title="Clear the console">Clear</button>
        <button type="button" className="bench-icon wire-console__btn wire-console__copy" onClick={() => void copy()} disabled={!lines.length}
          title="Copy as text with instrument, model and app version (for a bug report or forum post)">Copy</button>
        <button type="button" className="bench-icon wire-console__btn" onClick={() => void save()}
          title="Save everything kept for this instrument (about the last 20 minutes or more) as a text file">Save</button>
        {copied && <span className="wire-console__note" role="status">{copied}</span>}
      </div>
      <div ref={box} className="wire-console__log" tabIndex={0} aria-live="off"
        onScroll={(e) => { const el = e.currentTarget; stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24 }}>
        {lines.length === 0
          ? <span className="wire-console__empty">{paused ? 'Paused.' : 'Waiting for traffic…'}</span>
          : lines.map((l) => (
            <div key={`${l.seq}-${l.t}`} className={`wire-console__line wire-console__line--${l.dir}`}>{lineText(l, view)}</div>
          ))}
      </div>
    </section>
  )
}
