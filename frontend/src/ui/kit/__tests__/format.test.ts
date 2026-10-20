import { describe, it, expect } from 'vitest'
import { siNumber, fixedText, prefixed, decimalsFromFormat, intDigitsFor } from '../format'

describe('siNumber', () => {
  it('reads the base-unit value of a registry SI dict', () => {
    expect(siNumber({ val: '-0.00009789336', si: { number: '-97.8934', symbol: 'µ' } })).toBeCloseTo(-9.789336e-5)
    expect(siNumber({ val: '4.94' })).toBe(4.94)
  })
  it('accepts plain numbers and numeric strings, rejects the rest', () => {
    expect(siNumber(3)).toBe(3)
    expect(siNumber('1.5')).toBe(1.5)
    expect(siNumber(null)).toBeNull()
    expect(siNumber(undefined)).toBeNull()
    expect(siNumber({})).toBeNull()
    expect(siNumber('OL')).toBeNull()
  })
})

describe('fixedText', () => {
  it('pads to a fixed digit count so the readout never changes width', () => {
    expect(fixedText(4.94, 2, 2)).toBe(' 04.94')
    expect(fixedText(0.988, 1, 3)).toBe(' 0.988')
    expect(fixedText(-1.5, 2, 1)).toBe('-01.5')
    expect(fixedText(1000, 8, 0)).toBe(' 00001000')
  })
  it('shows dashes when there is no value', () => {
    expect(fixedText(null, 2, 2)).toBe(' --.--')
  })
})

describe('prefixed', () => {
  it('picks an SI prefix and keeps the significant digit count', () => {
    expect(prefixed(-9.789336e-5, 5)).toEqual({ text: '-97.893', prefix: 'µ' })
    expect(prefixed(3.3071, 5)).toEqual({ text: ' 3.3071', prefix: '' })
    expect(prefixed(4699.2, 5)).toEqual({ text: ' 4.6992', prefix: 'k' })
    expect(prefixed(0, 5)).toEqual({ text: ' 0.0000', prefix: '' })
  })
  it('renders overload as OL', () => {
    expect(prefixed(null, 5, true)).toEqual({ text: '  OL  ', prefix: '' })
    expect(prefixed(null, 5)).toEqual({ text: ' -----', prefix: '' })
  })
})

describe('definition formats', () => {
  it('takes decimals from printf formats, with a fallback for %g', () => {
    expect(decimalsFromFormat('%.2f', 3)).toBe(2)
    expect(decimalsFromFormat('%.3f', 1)).toBe(3)
    expect(decimalsFromFormat('%d', 2)).toBe(0)
    expect(decimalsFromFormat('%g', 3)).toBe(3)
    expect(decimalsFromFormat(undefined, 1)).toBe(1)
  })
  it('sizes the integer part from the maximum', () => {
    expect(intDigitsFor(30)).toBe(2)
    expect(intDigitsFor(5)).toBe(1)
    expect(intDigitsFor(70e6)).toBe(8)
    expect(intDigitsFor(1000)).toBe(4)
    expect(intDigitsFor(0.5)).toBe(1)
  })
})
