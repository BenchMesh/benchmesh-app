/**
 * The BenchMesh mark, "Readout": a seven-segment b with its unlit segments faintly visible and an
 * amber decimal point. One geometry for everything: `BrandMark` (app top bar, website), the SVG
 * files, the favicon and the app icons (`scripts/logo/build.mjs`).
 */

export type BrandColours = { seg: string; dp: string; ghost: number }

/** For dark backgrounds (the site, LED and glass skins, the app icon tile). */
export const BRAND_ON_DARK: BrandColours = { seg: '#5cf2a0', dp: '#ffb648', ghost: 0.13 }
/** For light backgrounds (the Reflective LCD skin, print, light pages). */
export const BRAND_ON_LIGHT: BrandColours = { seg: '#0f8a52', dp: '#c86f00', ghost: 0.14 }

const T = 6.6
const H = T / 2
const GAP = 1.5
const L = 19, R = 41, TOP = 11, MID = 32, BOT = 53

type Pt = [number, number]
const hs = (x1: number, x2: number, y: number): Pt[] => [[x1, y], [x1 + H, y - H], [x2 - H, y - H], [x2, y], [x2 - H, y + H], [x1 + H, y + H]]
const vs = (x: number, y1: number, y2: number): Pt[] => [[x, y1], [x + H, y1 + H], [x + H, y2 - H], [x, y2], [x - H, y2 - H], [x - H, y1 + H]]
const pts = (p: Pt[]) => p.map(([x, y]) => `${+x.toFixed(2)},${+y.toFixed(2)}`).join(' ')

/** The seven segments in a 64 x 64 box; `lit` ones form the b. */
export const BRAND_SEGMENTS: { id: string; points: string; lit: boolean }[] = [
  { id: 'a', p: hs(L + GAP, R - GAP, TOP), lit: false },
  { id: 'b', p: vs(R, TOP + GAP, MID - GAP), lit: false },
  { id: 'c', p: vs(R, MID + GAP, BOT - GAP), lit: true },
  { id: 'd', p: hs(L + GAP, R - GAP, BOT), lit: true },
  { id: 'e', p: vs(L, MID + GAP, BOT - GAP), lit: true },
  { id: 'f', p: vs(L, TOP + GAP, MID - GAP), lit: true },
  { id: 'g', p: hs(L + GAP, R - GAP, MID), lit: true },
].map(({ id, p, lit }) => ({ id, points: pts(p), lit }))

export const BRAND_DP = { cx: 49, cy: BOT, r: 3.5 }
/** Italic like the instrument readouts. */
export const BRAND_SLANT = 'translate(32 32) skewX(-8) translate(-32 -32)'

const TILE = { from: '#141b1e', to: '#0a0e10', rim: '#33434a' }

/** The mark (or the app-icon tile with the mark) as a standalone SVG document. */
export function brandSvg(c: BrandColours, opts: { tile?: boolean; size?: number } = {}): string {
  const segs = BRAND_SEGMENTS.map((s) => `<polygon points="${s.points}" fill="${c.seg}"${s.lit ? '' : ` opacity="${c.ghost}"`}/>`).join('')
  let body = `<g transform="${BRAND_SLANT}">${segs}<circle cx="${BRAND_DP.cx}" cy="${BRAND_DP.cy}" r="${BRAND_DP.r}" fill="${c.dp}"/></g>`
  if (opts.tile) {
    body = `<defs><linearGradient id="bm-tile" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${TILE.from}"/><stop offset="1" stop-color="${TILE.to}"/></linearGradient></defs>` +
      `<rect x="1" y="1" width="62" height="62" rx="14" fill="url(#bm-tile)" stroke="${TILE.rim}" stroke-width="1.2"/>` +
      `<g transform="translate(32 32) scale(.78) translate(-32 -32)">${body}</g>`
  }
  const size = opts.size ? ` width="${opts.size}" height="${opts.size}"` : ''
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"${size}>${body}</svg>`
}
