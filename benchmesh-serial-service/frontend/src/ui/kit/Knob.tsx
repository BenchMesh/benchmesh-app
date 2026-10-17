import React, { useRef } from 'react'
import { useCanvas, token } from './useCanvas'
import { useFocusedWheel } from './useFocusedWheel'

/**
 * Coarse continuous adjustment (drag up/down, or scroll/arrows while focused). It only
 * proposes a value through `onChange`; the paired digit editor shows it as pending until
 * the user applies it, so turning the knob never sends anything to the instrument.
 */
export function Knob({ value, min, max, onChange, label, disabled }: {
  value: number; min: number; max: number; onChange: (v: number) => void; label: string; disabled?: boolean
}) {
  const span = Math.max(1e-9, max - min)
  const f = Math.min(1, Math.max(0, (value - min) / span))
  const drag = useRef<{ y: number; v: number } | null>(null)
  const ref = useCanvas((ctx, w, h, el) => {
    const cx = w / 2, cy = h / 2, R = Math.min(w, h) / 2 - 7
    const a0 = Math.PI * 0.75, a1 = Math.PI * 2.25, a = a0 + (a1 - a0) * f
    ctx.lineCap = 'round'
    ctx.lineWidth = 5
    ctx.strokeStyle = token(el, '--kit-ctl-hi')
    ctx.beginPath(); ctx.arc(cx, cy, R, a0, a1); ctx.stroke()
    ctx.strokeStyle = token(el, disabled ? '--kit-ink-3' : '--kit-accent')
    ctx.beginPath(); ctx.arc(cx, cy, R, a0, a); ctx.stroke()
    ctx.lineWidth = 1
    ctx.strokeStyle = token(el, '--kit-ink-3')
    for (let i = 0; i <= 10; i++) {
      const t = a0 + ((a1 - a0) * i) / 10
      ctx.beginPath()
      ctx.moveTo(cx + Math.cos(t) * (R - 9), cy + Math.sin(t) * (R - 9))
      ctx.lineTo(cx + Math.cos(t) * (R - 13), cy + Math.sin(t) * (R - 13))
      ctx.stroke()
    }
    const g = ctx.createRadialGradient(cx - R * 0.2, cy - R * 0.3, 2, cx, cy, R * 0.72)
    g.addColorStop(0, token(el, '--kit-ctl-hi'))
    g.addColorStop(1, token(el, '--kit-ctl'))
    ctx.fillStyle = g
    ctx.beginPath(); ctx.arc(cx, cy, R * 0.66, 0, 7); ctx.fill()
    ctx.strokeStyle = token(el, '--kit-panel-edge'); ctx.stroke()
    ctx.strokeStyle = token(el, '--kit-ink')
    ctx.lineWidth = 3
    ctx.beginPath()
    ctx.moveTo(cx + Math.cos(a) * R * 0.2, cy + Math.sin(a) * R * 0.2)
    ctx.lineTo(cx + Math.cos(a) * R * 0.56, cy + Math.sin(a) * R * 0.56)
    ctx.stroke()
  }, [f, disabled])

  const set = (v: number) => onChange(Math.min(max, Math.max(min, v)))
  useFocusedWheel(ref, (dir) => { if (!disabled) set(value + dir * span / 100) })
  return (
    <canvas
      ref={ref}
      className="kit-canvas kit-knob"
      tabIndex={disabled ? -1 : 0}
      role="slider"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-disabled={disabled || undefined}
      onPointerDown={(e) => {
        if (disabled) return
        e.currentTarget.focus()
        e.currentTarget.setPointerCapture?.(e.pointerId)
        drag.current = { y: e.clientY, v: value }
      }}
      onPointerMove={(e) => { if (drag.current) set(drag.current.v + ((drag.current.y - e.clientY) / 180) * span) }}
      onPointerUp={() => { drag.current = null }}
      onKeyDown={(e) => {
        const d = ['ArrowUp', 'ArrowRight'].includes(e.key) ? 1 : ['ArrowDown', 'ArrowLeft'].includes(e.key) ? -1 : 0
        if (d && !disabled) { e.preventDefault(); set(value + d * span / 100) }
      }}
    />
  )
}
