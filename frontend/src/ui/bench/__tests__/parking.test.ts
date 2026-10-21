import { describe, expect, it } from 'vitest'
import { energizedOutputs, isParked, parkedState, runUsing } from '../parking'
import { Instrument } from '../../types'

const psu: Instrument = { id: 'psu', name: 'PSU', classes: [{ class: 'PSU', channels: ['/instruments/PSU/psu/1', '/instruments/PSU/psu/2'] }] }

describe('parking', () => {
  it('takes the parked state from the live registry once it lists the instrument', () => {
    expect(isParked({ ...psu, parked: true }, {})).toBe(true)
    expect(isParked({ ...psu, parked: true }, { psu: {} })).toBe(false) // unparked a moment ago
    expect(isParked(psu, { psu: { parked: { present: true, detail: '/dev/ttyUSB0' } } })).toBe(true)
    expect(isParked(psu, null)).toBe(false)
  })

  it('reports presence, null until checked', () => {
    expect(parkedState(psu, { psu: { parked: { present: false, detail: 'unplugged', of: 'port' } } })).toEqual({ present: false, detail: 'unplugged', of: 'port' })
    expect(parkedState({ ...psu, parked: true, health: { present: true, presence: 'COM5', presence_of: 'port' } }, {})).toEqual({ present: true, detail: 'COM5', of: 'port' })
    expect(parkedState({ ...psu, parked: true, health: { present: null, presence: null } }, {})).toEqual({ present: null, detail: '', of: 'instrument' })
  })

  it('lists the outputs that are on', () => {
    const registry = { psu: { IDN: 'x', PSU: { status_ch1: { OUTPUT: 'ON' }, status_ch2: { OUTPUT: 'OFF' }, updated_ch1: 1 } } }
    expect(energizedOutputs(psu, registry)).toEqual(['PSU ch1 OUTPUT'])
    const load: Instrument = { id: 'l', classes: [{ class: 'ELL', channels: [] }] }
    expect(energizedOutputs(load, { l: { ELL: { status_ch1: { INPUT: '1' } } } })).toEqual(['ELL ch1 INPUT'])
    expect(energizedOutputs(psu, {})).toEqual([])
  })

  it('names the running flow that uses the instrument', () => {
    expect(runUsing({ state: 'running', flow: { id: null, name: 'Sweep' }, devices: ['psu'], events: [] }, 'psu')).toBe('Sweep')
    expect(runUsing({ state: 'running', flow: { id: null, name: 'Sweep' }, devices: ['dmm'], events: [] }, 'psu')).toBeNull()
    expect(runUsing({ state: 'finished', flow: { id: null, name: 'Sweep' }, devices: ['psu'], events: [] }, 'psu')).toBeNull()
    expect(runUsing(undefined, 'psu')).toBeNull()
  })
})
