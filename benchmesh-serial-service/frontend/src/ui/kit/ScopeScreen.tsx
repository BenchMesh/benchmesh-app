import React from 'react'
import { useCanvas, token, glows } from './useCanvas'
import { TraceScale, tracePoints } from './scope'

export interface ScopeTrace extends TraceScale {
  ch: string
  codes: number[]
}

interface Props {
  traces: ScopeTrace[]
  /** Channels shown (traces may be missing while a channel waits for data). */
  channels: { ch: string; offsetDivs: number }[]
  hdivs?: number
  vdivs?: number
  /** Trigger level in divisions from the centre, on the trigger source's scale. */
  trigger?: { ch: string; levelDivs: number } | null
  /** Trigger position in divisions from the centre (−offset / timebase). */
  triggerPosDivs?: number
  label: string
}

const ch = (n: string) => `--kit-ch${Math.min(4, Math.max(1, Number(n) || 1))}`

/**
 * Oscilloscope screen: graticule (`hdivs` × `vdivs`, minor ticks on the centre axes), one
 * trace per displayed channel in the skin's channel colours, ground markers on the left,
 * trigger level on the right and the trigger position on top.
 */
export function ScopeScreen({ traces, channels, hdivs = 10, vdivs = 8, trigger, triggerPosDivs = 0, label }: Props) {
  const ref = useCanvas((ctx, w, h, el) => {
    const m = 14 // margin for markers
    const W = w - 2 * m
    const H = h - 2 * m
    const grid = token(el, '--kit-seg-off')
    const ink = token(el, '--kit-glass-ink-2')
    const glow = glows(el)
    ctx.save()
    ctx.translate(m, m)
    // graticule
    ctx.strokeStyle = grid
    ctx.lineWidth = 1
    for (let i = 0; i <= hdivs; i++) {
      const x = Math.round((i * W) / hdivs) + 0.5
      ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke()
    }
    for (let j = 0; j <= vdivs; j++) {
      const y = Math.round((j * H) / vdivs) + 0.5
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke()
    }
    ctx.strokeStyle = ink
    ctx.globalAlpha = 0.35
    for (let i = 0; i <= hdivs * 5; i++) {
      const x = (i * W) / (hdivs * 5)
      ctx.beginPath(); ctx.moveTo(x, H / 2 - 3); ctx.lineTo(x, H / 2 + 3); ctx.stroke()
    }
    for (let j = 0; j <= vdivs * 5; j++) {
      const y = (j * H) / (vdivs * 5)
      ctx.beginPath(); ctx.moveTo(W / 2 - 3, y); ctx.lineTo(W / 2 + 3, y); ctx.stroke()
    }
    ctx.globalAlpha = 1
    // traces
    ctx.save()
    ctx.beginPath(); ctx.rect(0, 0, W, H); ctx.clip()
    for (const t of traces) {
      if (!t.codes.length || !t.scale) continue
      const color = token(el, ch(t.ch))
      ctx.strokeStyle = color
      ctx.lineWidth = 1.5
      if (glow) { ctx.shadowColor = color; ctx.shadowBlur = 6 }
      ctx.beginPath()
      tracePoints(t.codes, t, W, H, vdivs).forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)))
      ctx.stroke()
      ctx.shadowBlur = 0
    }
    ctx.restore()
    // markers: ground levels (left), trigger level (right), trigger position (top)
    ctx.font = `700 9px ${token(el, '--kit-font-num')}`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    const yOf = (divs: number) => Math.min(H, Math.max(0, H / 2 - divs * (H / vdivs)))
    for (const c of channels) {
      const y = yOf(c.offsetDivs)
      ctx.fillStyle = token(el, ch(c.ch))
      ctx.beginPath(); ctx.moveTo(-m, y - 6); ctx.lineTo(-3, y); ctx.lineTo(-m, y + 6); ctx.closePath(); ctx.fill()
      ctx.fillStyle = token(el, '--kit-glass-top')
      ctx.fillText(c.ch, -m + 4, y)
    }
    if (trigger) {
      const y = yOf(trigger.levelDivs)
      ctx.fillStyle = token(el, ch(trigger.ch))
      ctx.beginPath(); ctx.moveTo(W + m, y - 6); ctx.lineTo(W + 3, y); ctx.lineTo(W + m, y + 6); ctx.closePath(); ctx.fill()
      ctx.globalAlpha = 0.5
      ctx.setLineDash([3, 4])
      ctx.strokeStyle = token(el, ch(trigger.ch))
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke()
      ctx.setLineDash([])
      ctx.globalAlpha = 1
    }
    const tx = Math.min(W, Math.max(0, W / 2 + triggerPosDivs * (W / hdivs)))
    ctx.fillStyle = token(el, '--kit-accent')
    ctx.beginPath(); ctx.moveTo(tx - 6, -m); ctx.lineTo(tx + 6, -m); ctx.lineTo(tx, -3); ctx.closePath(); ctx.fill()
    ctx.restore()
  }, [traces, channels.map((c) => `${c.ch}:${c.offsetDivs}`).join('|'), hdivs, vdivs, trigger?.ch, trigger?.levelDivs, triggerPosDivs])
  return <canvas ref={ref} className="kit-canvas kit-scope" role="img" aria-label={label} />
}
