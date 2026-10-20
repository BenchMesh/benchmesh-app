import React from 'react'
import { useCanvas, token, glows, toneVar, Tone } from './useCanvas'

/** Recent history under a readout: filled line, endpoint dot, no axes. */
export function Sparkline({ data, capacity = 150, tone }: { data: number[]; capacity?: number; tone?: Tone }) {
  const ref = useCanvas((ctx, w, h, el) => {
    if (data.length < 2) return
    const ink = token(el, toneVar(tone))
    let lo = Math.min(...data)
    let hi = Math.max(...data)
    if (hi - lo < 1e-12) { lo -= 1; hi += 1 }
    const X = (i: number) => ((capacity - data.length + i) / (capacity - 1)) * w
    const Y = (v: number) => h - 2 - ((v - lo) / (hi - lo)) * (h - 4)
    ctx.globalAlpha = 0.12
    ctx.fillStyle = ink
    ctx.beginPath()
    ctx.moveTo(X(0), h)
    data.forEach((v, i) => ctx.lineTo(X(i), Y(v)))
    ctx.lineTo(X(data.length - 1), h)
    ctx.closePath()
    ctx.fill()
    ctx.globalAlpha = 0.85
    ctx.strokeStyle = ink
    ctx.lineWidth = 1.3
    if (glows(el)) { ctx.shadowColor = ink; ctx.shadowBlur = 4 }
    ctx.beginPath()
    data.forEach((v, i) => (i ? ctx.lineTo(X(i), Y(v)) : ctx.moveTo(X(i), Y(v))))
    ctx.stroke()
    ctx.globalAlpha = 1
    ctx.beginPath()
    ctx.arc(X(data.length - 1) - 2, Y(data[data.length - 1]), 2.4, 0, 7)
    ctx.fill()
  }, [data, capacity, tone])
  return <canvas ref={ref} className="kit-canvas kit-readout__spark" aria-hidden="true" />
}
