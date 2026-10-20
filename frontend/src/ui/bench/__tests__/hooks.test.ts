import { describe, it, expect, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { useHistory, useSetpoint, useChannelStatus, ChannelApi } from '../api'
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

describe('useSetpoint', () => {
  const parse = (v: unknown) => (typeof v === 'number' ? v : null)

  it('reads the setpoint only once the instrument is online, and again after every reconnect', async () => {
    let reply = 5
    const api: ChannelApi = { get: vi.fn(async () => reply), set: vi.fn(async () => {}) }
    const { result, rerender } = renderHook(({ online }) => useSetpoint(api, 'voltage', parse, online), { initialProps: { online: false } })
    expect(api.get).not.toHaveBeenCalled()
    expect(result.current[0]).toBeNull()

    rerender({ online: true })
    await waitFor(() => expect(result.current[0]).toBe(5))

    reply = 7 // changed on the instrument's front panel while it was disconnected
    rerender({ online: false })
    expect(result.current[0]).toBe(5) // last value kept while offline
    rerender({ online: true })
    await waitFor(() => expect(result.current[0]).toBe(7))
    expect(api.get).toHaveBeenCalledTimes(2)
  })
})

describe('useChannelStatus', () => {
  it('keeps the same status object until the channel is updated again', () => {
    const reg = (at: number, v: string) => ({ d: { DMM: { status_ch1: { MEAS: v }, updated_ch1: at } } })
    const { result, rerender } = renderHook(({ r }) => useChannelStatus(r, 'd', 'DMM', '1'), { initialProps: { r: reg(1000, '1') } })
    const first = result.current
    // the registry is re-sent with the same reading: same object (not a new sample)
    rerender({ r: reg(1000, '1') })
    expect(result.current).toBe(first)
    // a new reading with the same value: a new object
    rerender({ r: reg(1500, '1') })
    expect(result.current).not.toBe(first)
    expect(result.current).toEqual({ MEAS: '1' })
  })

  it('without update times (older servers, mocks) every message is passed on', () => {
    const { result, rerender } = renderHook(({ r }) => useChannelStatus(r, 'd', 'DMM', '1'), { initialProps: { r: { d: { DMM: { status_ch1: { MEAS: '1' } } } } } })
    const first = result.current
    rerender({ r: { d: { DMM: { status_ch1: { MEAS: '1' } } } } })
    expect(result.current).not.toBe(first)
  })
})
