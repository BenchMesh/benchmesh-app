import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useHistory } from '../api'
import { useStats } from '../panels/DmmPanel'

describe('useHistory', () => {
  it('appends one sample per status update and keeps the last `capacity`', () => {
    const { result, rerender } = renderHook(({ v, t }) => useHistory(v, t, 3), { initialProps: { v: 1 as number | null, t: {} } })
    expect(result.current).toEqual([1])
    rerender({ v: 2, t: {} })
    rerender({ v: 3, t: {} })
    rerender({ v: 4, t: {} })
    expect(result.current).toEqual([2, 3, 4])
  })
  it('ignores re-renders without a new status and skips missing values', () => {
    const tick = {}
    const { result, rerender } = renderHook(({ v, t }) => useHistory(v, t), { initialProps: { v: 1 as number | null, t: tick } })
    rerender({ v: 1, t: tick })
    rerender({ v: null, t: {} })
    expect(result.current).toEqual([1])
  })
  it('restarts when the reset key changes', () => {
    const { result, rerender } = renderHook(({ v, t, k }) => useHistory(v, t, 150, k), { initialProps: { v: 1, t: {}, k: 'VOLT' } })
    rerender({ v: 2, t: {}, k: 'VOLT' })
    rerender({ v: 9, t: {}, k: 'RES' })
    expect(result.current).toEqual([9])
  })
})

describe('useStats', () => {
  it('tracks min/max/avg and restarts with the function', () => {
    const { result, rerender } = renderHook(({ v, t, k }) => useStats(v, t, k), { initialProps: { v: 2 as number | null, t: {}, k: 'VOLT' } })
    rerender({ v: 4, t: {}, k: 'VOLT' })
    rerender({ v: null, t: {}, k: 'VOLT' }) // held / no reading
    expect(result.current).toEqual({ min: 2, max: 4, sum: 6, n: 2 })
    rerender({ v: 10, t: {}, k: 'RES' })
    expect(result.current).toEqual({ min: 10, max: 10, sum: 10, n: 1 })
  })
})
