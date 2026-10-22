// UI soak test: opens the bench in Playwright
// (Chromium: WebKit needs system packages) and reports, every SAMPLE_S seconds, event-loop lag, registry messages, DOM size and the
// web process memory, to find work or state that grows the longer the UI stays open.
// usage: node scripts/soak/ui-soak.mjs <url> <minutes> [sample_s]
import { chromium } from '../../frontend/node_modules/playwright/index.mjs'

const [url = 'http://127.0.0.1:57691/ui/', minutes = '20', sampleS = '30'] = process.argv.slice(2)
const browser = await chromium.launch({ args: ['--enable-precise-memory-info'] })
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } })
await page.addInitScript(() => {
  localStorage.setItem('benchmesh:disclaimer-accepted', 'true')
  const s = (window.__soak = { maxLag: 0, lagSum: 0, lagN: 0, msgs: 0, msgMs: 0 })
  let last = performance.now()
  setInterval(() => {
    const now = performance.now(); const lag = now - last - 100; last = now
    s.maxLag = Math.max(s.maxLag, lag); s.lagSum += Math.max(0, lag); s.lagN++
  }, 100)
  const Native = window.WebSocket
  window.WebSocket = class extends Native {
    constructor(...a) {
      super(...a)
      this.addEventListener('message', () => {
        s.msgs++
        const t0 = performance.now()
        // the render React schedules for this message runs before the next macrotask
        setTimeout(() => { s.msgMs += performance.now() - t0 }, 0)
      })
    }
  }
})
page.on('pageerror', (e) => console.log('pageerror', e.message))
page.on('console', (m) => { if (m.type() === 'error') console.log('console.error', m.text().slice(0, 200)) })
await page.goto(url)
const t0 = Date.now()
console.log('min  maxLag  avgLag  msgs/s  ms/msg  dom   heapMB')
while (Date.now() - t0 < Number(minutes) * 60_000) {
  await page.waitForTimeout(Number(sampleS) * 1000)
  const s = await page.evaluate(() => { const x = { ...window.__soak, dom: document.getElementsByTagName('*').length, heap: Math.round((performance.memory?.usedJSHeapSize ?? 0) / 1048576) }; Object.assign(window.__soak, { maxLag: 0, lagSum: 0, lagN: 0, msgs: 0, msgMs: 0 }); return x })
  const m = ((Date.now() - t0) / 60000).toFixed(1)
  console.log(`${m.padStart(4)} ${s.maxLag.toFixed(0).padStart(6)} ${(s.lagSum / Math.max(1, s.lagN)).toFixed(1).padStart(7)} ${(s.msgs / Number(sampleS)).toFixed(1).padStart(7)} ${(s.msgMs / Math.max(1, s.msgs)).toFixed(1).padStart(7)} ${String(s.dom).padStart(5)} ${String(s.heap).padStart(7)}`)
}
await browser.close()
