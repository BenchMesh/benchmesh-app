import { describe, expect, it } from 'vitest'
import { formatBytes, formatDuration, levelOf, percentOf, temperatureLevel } from '../format'

describe('system figures', () => {
  it('sizes in powers of 1024', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(4 * 1024 ** 3)).toBe('4.0 GB')
    expect(formatBytes(31.4 * 1024 ** 3)).toBe('31 GB')
    expect(formatBytes(null)).toBe('-')
    expect(formatBytes(-1)).toBe('-')
  })

  it('durations in their two largest units', () => {
    expect(formatDuration(59)).toBe('59 s')
    expect(formatDuration(150)).toBe('2 min')
    expect(formatDuration(3720)).toBe('1 h 2 min')
    expect(formatDuration(93784)).toBe('1 d 2 h')
    expect(formatDuration(undefined)).toBe('-')
  })

  it('levels: full and hot', () => {
    expect([levelOf(50), levelOf(75), levelOf(95)]).toEqual(['ok', 'warn', 'alert'])
    expect([temperatureLevel(55), temperatureLevel(72), temperatureLevel(82)]).toEqual(['ok', 'warn', 'alert'])
    expect(percentOf(1, 4)).toBe(25)
    expect(percentOf(5, 0)).toBe(0)
    expect(percentOf(9, 4)).toBe(100)
  })
})
