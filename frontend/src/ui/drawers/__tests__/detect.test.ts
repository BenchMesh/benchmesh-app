import { describe, it, expect } from 'vitest'
import {
  identitySummary, certaintyLabel, instrumentName, uniqueId, buildAddEntry, buildLockEntry, apiError, pathField,
  type DetectedPort,
} from '../detect'

const xdm: DetectedPort = {
  port: '/dev/ttyUSB2', kind: 'serial',
  usb: { vid: '1a86', pid: '7523', serial: null, revision: '0264', manufacturer: null, product: 'USB Serial', location: '1-1.3.2.2.4' },
  configured_as: null, locked: false,
  instrument: { driver: 'owon_xdm', model: 'XDM1241', vendor: 'OWON', family: 'XDM', idn: 'OWON,XDM1241,24512010,V4.3.0,3', classes: ['DMM'] },
  match: { usb: { vid: '1a86', pid: '7523', revision: '0264', location: '1-1.3.2.2.4' }, idn_serial: '24512010' },
  certainty: 'instrument', error: null,
}

describe('identitySummary', () => {
  it('prefers the instrument serial number', () => {
    expect(identitySummary(xdm.match)).toBe('serial 24512010')
  })
  it('falls back to the adapter USB serial', () => {
    expect(identitySummary({ usb: { vid: '0416', pid: '5011', serial: 'AL036DJT' } })).toBe('USB serial AL036DJT')
  })
  it('falls back to the adapter type and revision', () => {
    expect(identitySummary({ usb: { vid: '1a86', pid: '7523', revision: '0264' } })).toBe('USB 1a86:7523 rev 0264')
    expect(identitySummary({ usb: { vid: '1a86', pid: '7523' } })).toBe('USB 1a86:7523')
  })
  it('works on the nullable USB info of a detected port', () => {
    expect(identitySummary({ usb: xdm.usb })).toBe('USB 1a86:7523 rev 0264')
  })
  it('is empty without an identity', () => {
    expect(identitySummary(null)).toBe('')
    expect(identitySummary({ usb: null })).toBe('')
  })
})

describe('certaintyLabel', () => {
  it('names how the instrument is identified', () => {
    expect(certaintyLabel('instrument')).toEqual({ text: 'identified by serial number', warn: false })
    expect(certaintyLabel('adapter')).toEqual({ text: 'identified by USB serial', warn: false })
    expect(certaintyLabel('chip')).toEqual({ text: 'identified by adapter type', warn: false })
  })
  it('warns when only the USB port position tells it apart', () => {
    const l = certaintyLabel('position')!
    expect(l.warn).toBe(true)
    expect(l.text).toMatch(/USB port position only/)
  })
  it('has no label when nothing was identified', () => {
    expect(certaintyLabel(null)).toBeNull()
  })
})

describe('uniqueId', () => {
  it('slugs vendor and model', () => {
    expect(uniqueId('OWON', 'XDM1241', [])).toBe('owon-xdm1241')
    expect(uniqueId('TENMA', '72-2540', [])).toBe('tenma-72-2540')
    expect(uniqueId('Rohde & Schwarz', 'HMC 8043', [])).toBe('rohde-schwarz-hmc-80')
  })
  it('appends -2, -3… when the id is taken', () => {
    expect(uniqueId('OWON', 'XDM1241', ['owon-xdm1241'])).toBe('owon-xdm1241-2')
    expect(uniqueId('OWON', 'XDM1241', ['owon-xdm1241', 'owon-xdm1241-2'])).toBe('owon-xdm1241-3')
  })
  it('stays within 20 characters with a suffix', () => {
    const id = uniqueId('Rohde & Schwarz', 'HMC 8043', ['rohde-schwarz-hmc-80'])
    expect(id).toBe('rohde-schwarz-hmc-2')
    expect(id.length).toBeLessThanOrEqual(20)
  })
  it('never returns an empty id', () => {
    expect(uniqueId('', '???', [])).toBe('instrument')
  })
})

describe('add and lock payloads', () => {
  it('builds a new entry identified by match, with the port as a hint', () => {
    expect(buildAddEntry(xdm, ['owon-spm3103'])).toEqual({
      id: 'owon-xdm1241', name: 'OWON XDM1241', driver: 'owon_xdm', model: 'XDM1241', transport: 'serial',
      match: xdm.match, port: '/dev/ttyUSB2',
    })
  })
  it('gives a USBTMC entry its path as `device`', () => {
    const e = buildAddEntry({ ...xdm, kind: 'usbtmc', port: '/dev/usbtmc0' }, ['owon-xdm1241'])
    expect(e).toMatchObject({ id: 'owon-xdm1241-2', transport: 'usbtmc', device: '/dev/usbtmc0' })
    expect(e).not.toHaveProperty('port')
    expect(pathField('usbtmc')).toBe('device')
    expect(pathField(undefined)).toBe('port')
  })
  it('refuses a port where nothing answered', () => {
    expect(() => buildAddEntry({ ...xdm, instrument: null, match: null }, [])).toThrow()
  })
  it('lock keeps every field of the entry and adds match', () => {
    const entry = { id: 'owon-xdm1241', name: 'DMM', driver: 'owon_xdm', model: 'XDM1241', port: '/dev/ttyUSB2', baud: 115200, extra: 1, _uuid: 'x' }
    expect(buildLockEntry(entry, xdm.match!)).toEqual({
      id: 'owon-xdm1241', name: 'DMM', driver: 'owon_xdm', model: 'XDM1241', port: '/dev/ttyUSB2', baud: 115200, extra: 1, match: xdm.match,
    })
  })
  it('instrumentName falls back to the driver', () => {
    expect(instrumentName(xdm.instrument!)).toBe('OWON XDM1241')
    expect(instrumentName({ ...xdm.instrument!, vendor: '' })).toBe('owon_xdm XDM1241')
  })
})

describe('apiError', () => {
  it('reads detail, else the status', async () => {
    expect(await apiError(new Response(JSON.stringify({ detail: 'duplicate id' }), { status: 409 }))).toBe('duplicate id')
    expect(await apiError(new Response('oops', { status: 500 }))).toBe('HTTP 500')
  })
})
