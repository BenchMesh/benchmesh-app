import { describe, it, expect } from 'vitest'
import { step125, screenDivs, siText, tracePoints } from '../scope'

describe('step125', () => {
  it('walks the 1-2-5 sequence across decades', () => {
    expect(step125(0.05, 1)).toBeCloseTo(0.1)
    expect(step125(0.05, -1)).toBeCloseTo(0.02)
    expect(step125(0.1, -1)).toBeCloseTo(0.05)
    expect(step125(2e-6, 1)).toBeCloseTo(5e-6)
    expect(step125(5e-6, 1)).toBeCloseTo(1e-5)
    expect(step125(1, -1)).toBeCloseTo(0.5)
  })
  it('snaps off-sequence values to the next step in the requested direction', () => {
    expect(step125(0.07, 1)).toBeCloseTo(0.1)
    expect(step125(0.07, -1)).toBeCloseTo(0.05)
    expect(step125(3, 1)).toBeCloseTo(5)
  })
  it('clamps to the range', () => {
    expect(step125(10, 1, 5e-4, 10)).toBe(10)
    expect(step125(5e-4, -1, 5e-4, 10)).toBe(5e-4)
  })
})

describe('screen mapping', () => {
  it('converts a sample code to divisions from the screen centre', () => {
    // DHO804 at 50 mV/div: y0 = 128, 1.7067 mV per code
    expect(screenDivs(128, 128, 0.0017067, 0, 0.05)).toBe(0)
    expect(screenDivs(128 + 117, 128, 0.0017067, 0, 0.05)).toBeCloseTo(4, 1) // top of an 8-div screen
    // a 20 mV offset moves the trace up by 0.4 div
    expect(screenDivs(128, 128, 0.0017067, 0.02, 0.05)).toBeCloseTo(0.4)
  })
  it('maps a trace to pixel points across the screen width', () => {
    const pts = tracePoints([128, 245, 11], { y0: 128, dy: 0.0017067, offset: 0, scale: 0.05 }, 100, 80, 8)
    expect(pts[0]).toEqual([0, 40])
    expect(pts[1][0]).toBe(50)
    expect(pts[1][1]).toBeCloseTo(0, 0)
    expect(pts[2]).toEqual([100, expect.closeTo(80, 0)])
  })
})

describe('siText', () => {
  it('formats compact engineering values', () => {
    expect(siText(0.05, 'V')).toBe('50 mV')
    expect(siText(2e-6, 's')).toBe('2 µs')
    expect(siText(312500000, 'Sa/s')).toBe('312.5 MSa/s')
    expect(siText(0.00272, 'V')).toBe('2.72 mV')
    expect(siText(-0.00071333, 'V')).toBe('-713.3 µV')
    expect(siText(0, 'V')).toBe('0 V')
    expect(siText(null, 'Hz')).toBe('-')
  })
})
