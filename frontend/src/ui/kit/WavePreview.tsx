import React from 'react'
import { useCanvas, token, glows } from './useCanvas'

/** One period of each waveform; `s` is the ramp symmetry (rising part, 0..1). */
const SHAPES: Record<string, (p: number, s: number) => number> = {
  SIN: (p) => Math.sin(p * 2 * Math.PI),
  SQU: (p) => (p % 1 < 0.5 ? 1 : -1),
  RAMP: (p, s) => {
    const x = p % 1
    if (s <= 0) return 1 - 2 * x
    if (s >= 1) return 2 * x - 1
    return x < s ? 2 * (x / s) - 1 : 1 - 2 * ((x - s) / (1 - s))
  },
  PUL: (p) => (p % 1 < 0.2 ? 1 : -1),
  PRN: (p) => Math.sin(p * 37.1) * Math.cos(p * 11.7), // deterministic "noise"
}

export const WAVE_SHAPES = Object.keys(SHAPES)

/** Small SVG glyph for a waveform selector button. */
export function WaveGlyph({ shape }: { shape: string }) {
  const d: Record<string, string> = {
    SIN: 'M1 6 C 4 -2, 7 -2, 11 6 S 18 14, 21 6',
    SQU: 'M1 10 V2 H11 V10 H21 V2',
    RAMP: 'M1 10 L19 2 V10',
    PUL: 'M1 10 H5 V2 H8 V10 H21',
    PRN: 'M1 6 L3 2 L5 9 L7 4 L9 10 L11 3 L13 8 L15 2 L17 9 L19 5 L21 7',
  }
  return (
    <svg viewBox="0 0 22 12" width="22" height="12" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d={d[shape] ?? 'M1 6 H21'} />
    </svg>
  )
}

/**
 * Three cycles of the configured waveform on an oscilloscope-like grid: amplitude and offset
 * are drawn relative to the generator's limits, dimmed while the output is off.
 */
export function WavePreview({ shape, amplitude, offset, maxAmplitude, maxOffset, on, symmetry = 100 }: {
  shape: string; amplitude: number; offset: number; maxAmplitude: number; maxOffset: number; on: boolean
  /** Ramp symmetry in % (rising part of the period); 100 is a sawtooth. */
  symmetry?: number
}) {
  const ref = useCanvas((ctx, w, h, el) => {
    const ink = token(el, '--kit-seg-on')
    ctx.strokeStyle = token(el, '--kit-seg-off')
    ctx.lineWidth = 1
    for (let i = 0; i <= 10; i++) { ctx.beginPath(); ctx.moveTo((i * w) / 10, 0); ctx.lineTo((i * w) / 10, h); ctx.stroke() }
    for (let i = 0; i <= 4; i++) { ctx.beginPath(); ctx.moveTo(0, (i * h) / 4); ctx.lineTo(w, (i * h) / 4); ctx.stroke() }
    const f = SHAPES[shape] ?? SHAPES.SIN
    const sym = Math.min(1, Math.max(0, symmetry / 100))
    const half = h / 2 - 4
    const A = half * Math.min(1, Math.max(0.08, amplitude / Math.max(1e-9, maxAmplitude)))
    const off = -half * 0.5 * Math.max(-1, Math.min(1, offset / Math.max(1e-9, maxOffset)))
    ctx.globalAlpha = on ? 1 : 0.4
    ctx.strokeStyle = ink
    ctx.lineWidth = 1.8
    if (on && glows(el)) { ctx.shadowColor = ink; ctx.shadowBlur = 8 }
    ctx.beginPath()
    for (let x = 0; x <= w; x++) {
      const y = h / 2 + off - f((x / w) * 3, sym) * A
      if (x) ctx.lineTo(x, y)
      else ctx.moveTo(x, y)
    }
    ctx.stroke()
    ctx.shadowBlur = 0
    ctx.globalAlpha = 1
    ctx.fillStyle = token(el, '--kit-glass-ink-2')
    ctx.font = `10px ${token(el, '--kit-font-num')}`
    ctx.textAlign = 'right'
    ctx.fillText(on ? 'OUTPUT ON' : 'OUTPUT OFF', w - 6, 12)
  }, [shape, amplitude, offset, maxAmplitude, maxOffset, on, symmetry])
  return <canvas ref={ref} className="kit-canvas kit-wave" role="img" aria-label={`${shape} waveform preview`} />
}
