// Logo assets from the one geometry in the kit (frontend/src/ui/kit/brand.ts):
//   docs/static/logo/*.svg            mark on dark / on light, app-icon tile
//   frontend/public/favicon.svg       the tile
//   crates/benchmesh-app/icons/*      desktop app icons (PNG, .ico, .icns) via `cargo tauri icon`
// Needs Inkscape (SVG -> 1024 px PNG) and the Tauri CLI. Run from the repository root:
//   node scripts/logo/build.mjs
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { BRAND_ON_DARK, BRAND_ON_LIGHT, brandSvg } from '../../benchmesh-serial-service/frontend/src/ui/kit/brand.ts'

const root = resolve(import.meta.dirname, '../..')
const logo = join(root, 'docs/static/logo')
mkdirSync(logo, { recursive: true })

const files = {
  'benchmesh-mark-on-dark.svg': brandSvg(BRAND_ON_DARK),
  'benchmesh-mark-on-light.svg': brandSvg(BRAND_ON_LIGHT),
  'benchmesh-icon.svg': brandSvg(BRAND_ON_DARK, { tile: true }),
}
for (const [name, svg] of Object.entries(files)) writeFileSync(join(logo, name), svg + '\n')
copyFileSync(join(logo, 'benchmesh-icon.svg'), join(root, 'benchmesh-serial-service/frontend/public/favicon.svg'))

// app icons: a 1024 px PNG of the tile, then only the files tauri.conf.json bundles (+ icon.png)
const tmp = mkdtempSync(join(root, 'target/logo-'))
try {
  const png = join(tmp, 'icon-1024.png')
  execFileSync('inkscape', [join(logo, 'benchmesh-icon.svg'), '--export-type=png', `--export-filename=${png}`, '-w', '1024', '-h', '1024'], { stdio: 'ignore' })
  copyFileSync(png, join(logo, 'benchmesh-icon-1024.png'))
  execFileSync('cargo', ['tauri', 'icon', png, '-o', join(tmp, 'icons')], { cwd: join(root, 'crates/benchmesh-app'), stdio: 'ignore' })
  for (const f of ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.icns', 'icon.ico', 'icon.png']) {
    copyFileSync(join(tmp, 'icons', f), join(root, 'crates/benchmesh-app/icons', f))
  }
} finally {
  rmSync(tmp, { recursive: true, force: true })
}
console.log('logo: docs/static/logo, frontend/public/favicon.svg, crates/benchmesh-app/icons')
