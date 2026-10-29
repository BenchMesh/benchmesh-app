/** A mocked API with one serial entry that has a `match` (Instruments form tests). */
import { vi } from 'vitest'

export const XDM = { id: 'x1', name: 'XDM', driver: 'owon_xdm', transport: 'serial', port: '/dev/ttyUSB2', baud: 115200, serial: '8N1',
  match: { usb: { vid: '1a86', pid: '7523' }, idn_serial: '24512010' } }

export function mockApi() {
  vi.spyOn(globalThis, 'fetch').mockImplementation((url) => {
    const u = String(url)
    const body = u.endsWith('/config') ? { devices: [XDM] } : u.includes('/drivers') ? {} : []
    return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } }))
  })
}
