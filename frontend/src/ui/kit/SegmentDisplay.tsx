import React from 'react'
import { useCanvas, token, glows, toneVar, Tone } from './useCanvas'
import { useSkin } from './skin'

// Lit segments per character (a top, b upper right, c lower right, d bottom, e lower left, f upper left, g middle).
const SEGMENTS: Record<string, string> = {
  '0': 'abcdef', '1': 'bc', '2': 'abged', '3': 'abgcd', '4': 'fgbc', '5': 'afgcd', '6': 'afgedc', '7': 'abc',
  '8': 'abcdefg', '9': 'abcdfg', '-': 'g', ' ': '', O: 'abcdef', L: 'fed', E: 'afged', r: 'eg', o: 'cdeg',
  P: 'abefg', H: 'bcefg', C: 'afed', A: 'abcefg', F: 'afeg', n: 'ceg', u: 'cde', t: 'fged', d: 'bcdeg',
}

/** Characters with the decimal point folded into the preceding cell, as on a real display. */
export function segmentCells(text: string): { ch: string; dp: boolean }[] {
  const out: { ch: string; dp: boolean }[] = []
  for (const ch of text) {
    if (ch === '.' && out.length) out[out.length - 1].dp = true
    else out.push({ ch, dp: false })
  }
  return out
}

type Pt = [number, number]

function drawSegments(ctx: CanvasRenderingContext2D, w: number, h: number, text: string, on: string, off: string, glow: boolean) {
  const cells = segmentCells(text)
  if (!cells.length) return
  const H = h * 0.9
  const W = H * 0.52
  const t = W * 0.17
  const adv = W + t * 1.25
  const g = t * 0.14
  const total = cells.length * adv
  const scale = Math.min(1, (w - 4) / total)
  ctx.save()
  ctx.translate(w - total * scale, (h - H * scale) / 2)
  ctx.scale(scale, scale)
  ctx.transform(1, 0, -0.09, 1, H * 0.09, 0) // slight italic, like LCD glass
  const x0 = t / 2, x1 = W - t / 2, y0 = t / 2, ym = H / 2, y1 = H - t / 2
  const hseg = (x: number, y: number, L: number): Pt[] =>
    [[x, y], [x + t / 2, y - t / 2], [x + L - t / 2, y - t / 2], [x + L, y], [x + L - t / 2, y + t / 2], [x + t / 2, y + t / 2]]
  const vseg = (x: number, y: number, L: number): Pt[] =>
    [[x, y], [x + t / 2, y + t / 2], [x + t / 2, y + L - t / 2], [x, y + L], [x - t / 2, y + L - t / 2], [x - t / 2, y + t / 2]]
  const P: Record<string, Pt[]> = {
    a: hseg(x0 + g, y0, x1 - x0 - 2 * g), g: hseg(x0 + g, ym, x1 - x0 - 2 * g), d: hseg(x0 + g, y1, x1 - x0 - 2 * g),
    f: vseg(x0, y0 + g, ym - y0 - 2 * g), b: vseg(x1, y0 + g, ym - y0 - 2 * g),
    e: vseg(x0, ym + g, y1 - ym - 2 * g), c: vseg(x1, ym + g, y1 - ym - 2 * g),
  }
  const poly = (pts: Pt[]) => {
    ctx.beginPath()
    pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)))
    ctx.closePath()
    ctx.fill()
  }
  cells.forEach((c, i) => {
    ctx.save()
    ctx.translate(i * adv, 0)
    ctx.fillStyle = off
    ctx.shadowBlur = 0
    Object.values(P).forEach(poly)
    ctx.fillRect(W + t * 0.3, H - t, t, t)
    ctx.fillStyle = on
    if (glow) { ctx.shadowColor = on; ctx.shadowBlur = t * 1.4 }
    for (const s of SEGMENTS[c.ch] ?? '') poly(P[s])
    if (c.dp) ctx.fillRect(W + t * 0.3, H - t, t, t)
    ctx.restore()
  })
  ctx.restore()
}

/** Rough advance of the text readout in em: digits are one mono cell, the point a narrow slot. */
function textWidthEm(text: string) {
  let w = 0
  for (const ch of text) w += ch === '.' ? 0.28 : 0.6
  return Math.max(1, w)
}

/**
 * A fixed-width numeric display in the active skin: seven-segment canvas (right-aligned,
 * unlit ghost segments visible, glowing in LED), or B612 Mono text in the Glass skin.
 * `tone` picks the per-quantity colour token (one ink in LCD).
 */
export function SegmentDisplay({ text, label, tone }: { text: string; label?: string; tone?: Tone }) {
  const { skin } = useSkin()
  const name = label ? `${label} ${text.trim()}` : text.trim()
  if (skin === 'glass') {
    return (
      <div className="kit-numtext" role="img" aria-label={name}
        style={{ color: `var(${toneVar(tone)})`, '--kit-numtext-w': textWidthEm(text) } as React.CSSProperties}>
        <span aria-hidden="true">
          {[...text].map((ch, i) => ch === '.' ? <span key={i} className="kit-numtext__dp">.</span> : ch)}
        </span>
      </div>
    )
  }
  return <SegmentCanvas text={text} name={name} tone={tone} />
}

function SegmentCanvas({ text, name, tone }: { text: string; name: string; tone?: Tone }) {
  const ref = useCanvas((ctx, w, h, el) => {
    drawSegments(ctx, w, h, text, token(el, toneVar(tone)), token(el, '--kit-seg-off'), glows(el))
  }, [text, tone])
  return <canvas ref={ref} className="kit-canvas" role="img" aria-label={name} />
}
