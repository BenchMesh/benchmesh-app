import React from 'react'
import { useCanvas, token, glows, toneVar, Tone } from './useCanvas'

/** Segmented bar for "how much of the range": `fraction` 0..1 with scale marks underneath. */
export function Bargraph({ fraction, marks, label, cells = 40, tone }: {
  fraction: number; marks: string[]; label: string; cells?: number; tone?: Tone
}) {
  const f = Number.isFinite(fraction) ? Math.min(1, Math.max(0, fraction)) : 0
  const ref = useCanvas((ctx, w, h, el) => {
    const gap = 2
    const bw = (w - gap * (cells - 1)) / cells
    const bh = h - 12
    const lit = Math.round(f * cells)
    const on = token(el, toneVar(tone))
    const off = token(el, '--kit-seg-off')
    const glow = glows(el)
    for (let i = 0; i < cells; i++) {
      ctx.fillStyle = i < lit ? on : off
      if (glow && i < lit) { ctx.shadowColor = on; ctx.shadowBlur = 6 } else ctx.shadowBlur = 0
      ctx.fillRect(i * (bw + gap), 0, bw, bh)
    }
    ctx.shadowBlur = 0
    ctx.fillStyle = token(el, '--kit-glass-ink-3')
    ctx.font = `10px ${token(el, '--kit-font-num')}`
    marks.forEach((m, i) => {
      ctx.textAlign = i === 0 ? 'left' : i === marks.length - 1 ? 'right' : 'center'
      ctx.fillText(m, (i / Math.max(1, marks.length - 1)) * w, h - 1)
    })
  }, [f, marks.join('|'), cells, tone])
  return (
    <canvas ref={ref} className="kit-canvas kit-bar" role="meter" aria-label={label}
      aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(f * 100)} />
  )
}
