import { BRAND_DP, BRAND_ON_DARK, BRAND_SEGMENTS, BRAND_SLANT, type BrandColours } from './brand'

/**
 * The BenchMesh mark. Without `colours` it follows the skin (`--kit-brand-seg`, `--kit-brand-dp`,
 * `--kit-brand-ghost` in `kit.css`, on-dark colours where the kit's CSS is not loaded); pass
 * `colours` to fix them (e.g. the website, which is always dark).
 */
export function BrandMark({ size = 24, className, colours }: { size?: number; className?: string; colours?: BrandColours }) {
  const d = BRAND_ON_DARK
  const seg = colours?.seg ?? `var(--kit-brand-seg, ${d.seg})`
  const dp = colours?.dp ?? `var(--kit-brand-dp, ${d.dp})`
  const ghost = colours ? String(colours.ghost) : `var(--kit-brand-ghost, ${d.ghost})`
  return (
    <svg viewBox="0 0 64 64" width={size} height={size} className={className} aria-hidden="true" focusable="false">
      <g transform={BRAND_SLANT}>
        {BRAND_SEGMENTS.map((s) => (
          <polygon key={s.id} points={s.points} style={{ fill: seg, opacity: s.lit ? undefined : ghost }} />
        ))}
        <circle cx={BRAND_DP.cx} cy={BRAND_DP.cy} r={BRAND_DP.r} style={{ fill: dp }} />
      </g>
    </svg>
  )
}
