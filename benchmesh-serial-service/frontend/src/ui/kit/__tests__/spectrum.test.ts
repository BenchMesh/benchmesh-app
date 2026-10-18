import { describe, it, expect } from 'vitest'
import { freqAt, freqFraction, levelDivs, spectrumPoints, peakIndex, maxHold, freqText, dbText } from '../spectrum'

describe('spectrum arithmetic', () => {
  it('frequency of a sweep point and screen fraction of a frequency', () => {
    expect(freqAt(0, 5, 100, 500)).toBe(100)
    expect(freqAt(4, 5, 100, 500)).toBe(500)
    expect(freqAt(2, 5, 100, 500)).toBe(300)
    expect(freqFraction(300, 100, 500)).toBe(0.5)
    expect(freqFraction(50, 100, 500)).toBeNull()
    expect(freqFraction(100, 100, 100)).toBeNull() // zero span
  })
  it('levels to screen: the top line is the reference level', () => {
    expect(levelDivs(-10, -10, 10)).toBe(0)
    expect(levelDivs(-60, -10, 10)).toBe(5)
    expect(spectrumPoints([-10, -110], -10, 10, 200, 100, 10)).toEqual([[0, 0], [200, 100]])
  })
  it('peak and max hold', () => {
    expect(peakIndex([-80, -40, -40, -90])).toBe(1)
    expect(peakIndex([])).toBe(-1)
    expect(maxHold([-50, -60], [-55, -40])).toEqual([-50, -40])
    expect(maxHold([-50], [-55, -40])).toEqual([-55, -40])
  })
  it('labels', () => {
    expect(freqText(433920000, 7)).toBe('433.9200 MHz')
    expect(freqText(0)).toBe('0 Hz')
    expect(dbText(-42.314)).toBe('−42.31 dBm')
    expect(dbText(null)).toBe('—')
  })
})
