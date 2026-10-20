import { describe, it, expect } from 'vitest'
import { digitModel } from '../digitModel'

const volts = digitModel({ min: 0, max: 30, intDigits: 2, decimals: 2 })
const offset = digitModel({ min: -10, max: 10, intDigits: 2, decimals: 3 })

describe('digitModel', () => {
  it('splits a value into fixed digits', () => {
    expect(volts.digits(4.94)).toEqual(['0', '4', '9', '4'])
    expect(volts.total).toBe(4)
  })
  it('steps the selected digit and clamps to the range', () => {
    expect(volts.bump(4.94, 1, 1)).toBe(5.94)
    expect(volts.bump(4.94, 3, -1)).toBe(4.93)
    expect(volts.bump(29.5, 0, 1)).toBe(30)
    expect(volts.bump(0.05, 2, -1)).toBe(0)
  })
  it('avoids floating point drift', () => {
    expect(volts.bump(0.1, 3, 2)).toBe(0.12)
    expect(volts.bump(0.29, 3, 1)).toBe(0.3)
  })
  it('types a digit in place', () => {
    expect(volts.type(4.94, 0, '1')).toBe(14.94)
    expect(volts.type(4.94, 3, '0')).toBe(4.9)
    expect(volts.type(4.94, 0, '9')).toBe(30) // clamped
  })
  it('handles negative ranges', () => {
    expect(offset.digits(-1.5)).toEqual(['0', '1', '5', '0', '0'])
    expect(offset.bump(0, 1, -1)).toBe(-1)
    expect(offset.type(-1.5, 1, '2')).toBe(-2.5)
    expect(offset.negate(1.25)).toBe(-1.25)
    expect(volts.negate(1.25)).toBe(1.25) // min >= 0: no sign
  })
})
