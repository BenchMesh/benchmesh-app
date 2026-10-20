import React from 'react'
import { useCanvas, token, glows } from './useCanvas'
import { spectrumPoints, levelDivs } from './spectrum'

interface Props {
  /** Trace levels (dBm), START..STOP; null while there is none. */
  trace: number[] | null
  /** Client-side max-hold trace, drawn as a ghost behind the live trace. */
  ghost?: number[] | null
  /** Top graticule line (dBm) and dB/div. */
  refLevel: number
  scale: number
  hdivs?: number
  vdivs?: number
  /** Marker 1: x as a fraction of the width, level in dBm. */
  marker?: { x: number; level: number } | null
  /** Click on the screen: x as a fraction of the width (place the marker). */
  onPick?: (x: number) => void
  label: string
}

const M = { l: 34, r: 8, t: 8, b: 8 } // margins: level labels on the left

/**
 * Spectrum analyzer screen: dB graticule (`hdivs` × `vdivs`) with the reference level on the top
 * line and level labels on the left, the trace in the skin's first channel colour, an optional
 * max-hold ghost, and marker 1 as a diamond on the trace.
 */
export function SpectrumScreen({ trace, ghost, refLevel, scale, hdivs = 10, vdivs = 10, marker, onPick, label }: Props) {
  const ref = useCanvas((ctx, w, h, el) => {
    const W = w - M.l - M.r
    const H = h - M.t - M.b
    const grid = token(el, '--kit-seg-off')
    const ink = token(el, '--kit-glass-ink-2')
    const glow = glows(el)
    ctx.save()
    ctx.translate(M.l, M.t)
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
    // level labels: every other division (all when there are few)
    ctx.fillStyle = ink
    ctx.font = `400 9px ${token(el, '--kit-font-num')}`
    ctx.textAlign = 'right'
    ctx.textBaseline = 'middle'
    const every = vdivs > 6 ? 2 : 1
    for (let j = 0; j <= vdivs; j += every) {
      const v = refLevel - j * scale
      ctx.fillText(`${Number(v.toFixed(1))}`, -4, (j * H) / vdivs)
    }
    ctx.save()
    ctx.beginPath(); ctx.rect(0, 0, W, H); ctx.clip()
    const draw = (levels: number[], color: string, alpha: number) => {
      ctx.globalAlpha = alpha
      ctx.strokeStyle = color
      ctx.lineWidth = 1.5
      if (glow) { ctx.shadowColor = color; ctx.shadowBlur = 6 }
      ctx.beginPath()
      spectrumPoints(levels, refLevel, scale, W, H, vdivs).forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)))
      ctx.stroke()
      ctx.shadowBlur = 0
      ctx.globalAlpha = 1
    }
    if (ghost?.length) draw(ghost, token(el, '--kit-ch2'), 0.45)
    if (trace?.length) draw(trace, token(el, '--kit-ch1'), 1)
    ctx.restore()
    if (marker) {
      const x = marker.x * W
      const y = Math.min(H, Math.max(0, levelDivs(marker.level, refLevel, scale) * (H / vdivs)))
      ctx.fillStyle = token(el, '--kit-accent')
      ctx.beginPath(); ctx.moveTo(x, y - 7); ctx.lineTo(x + 5, y - 2); ctx.lineTo(x, y + 3); ctx.lineTo(x - 5, y - 2); ctx.closePath(); ctx.fill()
      ctx.font = `700 9px ${token(el, '--kit-font-num')}`
      ctx.textAlign = 'center'
      ctx.textBaseline = 'bottom'
      ctx.fillText('1', x, y - 8)
    }
    ctx.restore()
  }, [trace, ghost, refLevel, scale, hdivs, vdivs, marker?.x, marker?.level])
  return (
    <canvas ref={ref} className="kit-canvas kit-spectrum" role="img" aria-label={label}
      onClick={onPick ? (e) => {
        const r = e.currentTarget.getBoundingClientRect()
        const W = r.width - M.l - M.r
        if (W <= 0) return
        const x = (e.clientX - r.left - M.l) / W
        if (x >= 0 && x <= 1) onPick(x)
      } : undefined} />
  )
}
