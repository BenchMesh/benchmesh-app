import { describe, it, expect, vi, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useConfirmation, CONFIRM_TIMEOUT_MS } from '../confirm'

afterEach(() => vi.useRealTimers())

describe('useConfirmation', () => {
  it('waits until the instrument reports the requested value', () => {
    const { result, rerender } = renderHook(({ reported }) => useConfirmation(reported), { initialProps: { reported: 'OFF' } })
    expect(result.current.state).toBe('idle')
    act(() => result.current.begin('ON'))
    expect(result.current.state).toBe('waiting')
    expect(result.current.target).toBe('ON')
    rerender({ reported: 'OFF' })
    expect(result.current.state).toBe('waiting')
    rerender({ reported: 'ON' })
    expect(result.current.state).toBe('idle')
    expect(result.current.target).toBeNull()
  })

  it('reports "unconfirmed" when the instrument does not follow in time', () => {
    vi.useFakeTimers()
    const { result } = renderHook(() => useConfirmation(5.0, (a, b) => Math.abs(a - b) < 1e-6))
    act(() => result.current.begin(5.5))
    act(() => { vi.advanceTimersByTime(CONFIRM_TIMEOUT_MS + 10) })
    expect(result.current.state).toBe('unconfirmed')
  })

  it('a new request or a failure resets it', () => {
    vi.useFakeTimers()
    const { result } = renderHook(() => useConfirmation('A'))
    act(() => result.current.begin('B'))
    act(() => { vi.advanceTimersByTime(CONFIRM_TIMEOUT_MS + 10) })
    act(() => result.current.begin('C'))
    expect(result.current.state).toBe('waiting')
    act(() => result.current.cancel())
    expect(result.current.state).toBe('idle')
  })

  it('a request for the current value is confirmed immediately', () => {
    const { result } = renderHook(() => useConfirmation('ON'))
    act(() => result.current.begin('ON'))
    expect(result.current.state).toBe('idle')
  })
})
