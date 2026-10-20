import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { BRAND_ON_DARK, BRAND_ON_LIGHT, BRAND_SEGMENTS, brandSvg } from '../brand'
import { BrandMark } from '../BrandMark'

const repo = join(__dirname, '../../../../../..')

describe('BenchMesh mark', () => {
  it('is a seven-segment b: five lit segments, two ghosts and a decimal point', () => {
    expect(BRAND_SEGMENTS.filter((s) => s.lit).map((s) => s.id)).toEqual(['c', 'd', 'e', 'f', 'g'])
    const svg = brandSvg(BRAND_ON_DARK)
    expect(svg.match(/<polygon/g)).toHaveLength(7)
    expect(svg.match(/opacity="0.13"/g)).toHaveLength(2)
    expect(svg).toContain(`fill="${BRAND_ON_DARK.dp}"`)
  })

  it('generated files match the geometry (run node scripts/logo/build.mjs after changing it)', () => {
    const file = (p: string) => readFileSync(join(repo, p), 'utf8').trim()
    expect(file('docs/static/logo/benchmesh-mark-on-dark.svg')).toBe(brandSvg(BRAND_ON_DARK))
    expect(file('docs/static/logo/benchmesh-mark-on-light.svg')).toBe(brandSvg(BRAND_ON_LIGHT))
    expect(file('docs/static/logo/benchmesh-icon.svg')).toBe(brandSvg(BRAND_ON_DARK, { tile: true }))
    expect(file('benchmesh-serial-service/frontend/public/favicon.svg')).toBe(brandSvg(BRAND_ON_DARK, { tile: true }))
  })

  it('follows the skin tokens unless colours are given', () => {
    const { container, rerender } = render(<BrandMark />)
    const lit = () => container.querySelector('polygon:last-of-type') as SVGElement
    expect(lit().style.fill).toContain('--kit-brand-seg')
    rerender(<BrandMark colours={BRAND_ON_LIGHT} />)
    expect(lit().style.fill).not.toContain('var(')
    expect(container.querySelectorAll('polygon')).toHaveLength(7)
  })
})
