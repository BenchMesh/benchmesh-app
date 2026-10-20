import { describe, it, expect } from 'vitest'
import {
  dbmOf, fromHighLow, highLow, loadOhms, peakToRms, periodUnitFor, reciprocal, roundTo, vppFromDbm, vppFromVrms, vrmsOf,
} from '../awgMath'

describe('generator conversions', () => {
  it('period and frequency', () => {
    expect(reciprocal(1000)).toBeCloseTo(1e-3)
    expect(reciprocal(0)).toBeNull()
    expect(reciprocal(null)).toBeNull()
  })

  it('Vrms only for waveforms with a fixed crest factor', () => {
    expect(vrmsOf(2 * Math.SQRT2, 'SIN')).toBeCloseTo(1)
    expect(vrmsOf(2, 'squ')).toBeCloseTo(1)
    expect(vrmsOf(2 * Math.sqrt(3), 'RAMP')).toBeCloseTo(1)
    for (const s of ['PUL', 'PRN', 'ARB', 'DC', '']) expect(peakToRms(s)).toBeNull()
    expect(vppFromVrms(1, 'SIN')).toBeCloseTo(2.828427)
    expect(vppFromVrms(1, 'PRN')).toBeNull()
  })

  it('dBm into a finite load only', () => {
    // 0 dBm into 50 ohm = 0.2236 Vrms = 0.6325 Vpp (sine)
    expect(dbmOf(0.632456, 'SIN', 50)).toBeCloseTo(0, 3)
    expect(vppFromDbm(0, 'SIN', 50)).toBeCloseTo(0.632456, 5)
    expect(vppFromDbm(10, 'SIN', 50)).toBeCloseTo(2, 3)
    expect(dbmOf(1, 'SIN', Infinity)).toBeNull()
    expect(dbmOf(1, 'SIN', null)).toBeNull()
    expect(vppFromDbm(0, 'SIN', Infinity)).toBeNull()
    expect(dbmOf(0, 'SIN', 50)).toBeNull()
  })

  it('reads the LOAD status forms of the definitions', () => {
    expect(loadOhms('50')).toBe(50)
    expect(loadOhms('HIGHZ')).toBe(Infinity)
    expect(loadOhms(9.9e37)).toBe(Infinity)
    expect(loadOhms(' 600 ')).toBe(600)
    expect(loadOhms(undefined)).toBeNull()
    expect(loadOhms('weird')).toBeNull()
  })

  it('high and low levels', () => {
    expect(highLow(2, 0.5)).toEqual({ high: 1.5, low: -0.5 })
    expect(highLow(null, 0)).toBeNull()
    expect(fromHighLow(3.3, 0)).toEqual({ amplitude: 3.3, offset: 1.65 })
  })

  it('period unit and rounding', () => {
    expect(periodUnitFor(2).unit).toBe('s')
    expect(periodUnitFor(1e-3).unit).toBe('ms')
    expect(periodUnitFor(2.5e-7).unit).toBe('ns')
    expect(periodUnitFor(null).unit).toBe('ms')
    expect(roundTo(3.0000031, 3)).toBe(3)
  })
})
