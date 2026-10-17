import React from 'react'
import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent, act, waitFor } from '@testing-library/react'
import { DigitEditor } from '../DigitEditor'
import { ArmSwitch } from '../ArmSwitch'

const spec = { min: 0, max: 30, intDigits: 2, decimals: 2 }

describe('DigitEditor', () => {
  it('steps the selected digit with the arrow keys and applies on Enter', async () => {
    const onCommit = vi.fn().mockResolvedValue(undefined)
    render(<DigitEditor label="Voltage setpoint" value={4.94} spec={spec} unit="V" onCommit={onCommit} />)
    const ed = screen.getByRole('spinbutton', { name: 'Voltage setpoint' })
    fireEvent.focus(ed)
    // the ones digit is selected first
    fireEvent.keyDown(ed, { key: 'ArrowUp' })
    expect(ed).toHaveAttribute('aria-valuenow', '5.94')
    expect(ed.className).toContain('kit-digits--pending')
    expect(onCommit).not.toHaveBeenCalled()
    fireEvent.keyDown(ed, { key: 'Enter' })
    await waitFor(() => expect(onCommit).toHaveBeenCalledWith(5.94))
    await waitFor(() => expect(ed.className).not.toContain('kit-digits--pending'))
  })

  it('selects a digit by clicking, types digits, and Escape reverts', () => {
    const onCommit = vi.fn()
    render(<DigitEditor label="V" value={4.94} spec={spec} unit="V" onCommit={onCommit} />)
    const ed = screen.getByRole('spinbutton')
    fireEvent.click(screen.getAllByTestId('digit')[0])
    fireEvent.keyDown(ed, { key: '1' })
    expect(ed).toHaveAttribute('aria-valuenow', '14.94')
    fireEvent.keyDown(ed, { key: 'Escape' })
    expect(ed).toHaveAttribute('aria-valuenow', '4.94')
    expect(onCommit).not.toHaveBeenCalled()
  })

  it('follows the instrument value while not being edited', () => {
    const { rerender } = render(<DigitEditor label="V" value={4.94} spec={spec} unit="V" onCommit={vi.fn()} />)
    rerender(<DigitEditor label="V" value={12.5} spec={spec} unit="V" onCommit={vi.fn()} />)
    expect(screen.getByRole('spinbutton')).toHaveAttribute('aria-valuenow', '12.5')
  })

  it('keeps the draft and shows the error when applying fails', async () => {
    const onCommit = vi.fn().mockRejectedValue(new Error('Device requires remote mode'))
    render(<DigitEditor label="V" value={4.94} spec={spec} unit="V" onCommit={onCommit} />)
    const ed = screen.getByRole('spinbutton')
    fireEvent.focus(ed)
    fireEvent.keyDown(ed, { key: 'ArrowDown' })
    fireEvent.keyDown(ed, { key: 'Enter' })
    expect(await screen.findByText('Device requires remote mode')).toBeInTheDocument()
    expect(ed).toHaveAttribute('aria-valuenow', '3.94')
    expect(ed.className).toContain('kit-digits--error')
  })

  it('does nothing while disabled', () => {
    render(<DigitEditor label="V" value={4.94} spec={spec} unit="V" onCommit={vi.fn()} disabled />)
    const ed = screen.getByRole('spinbutton')
    fireEvent.keyDown(ed, { key: 'ArrowUp' })
    expect(ed).toHaveAttribute('aria-valuenow', '4.94')
  })
})

describe('ArmSwitch', () => {
  afterEach(() => vi.useRealTimers())

  it('needs two presses to switch on, one to switch off', async () => {
    const onChange = vi.fn().mockResolvedValue(undefined)
    const { rerender } = render(<ArmSwitch label="Output" on={false} onChange={onChange} />)
    const btn = screen.getByRole('button', { name: /Output/ })
    fireEvent.click(btn)
    expect(btn).toHaveAttribute('data-state', 'armed')
    expect(onChange).not.toHaveBeenCalled()
    fireEvent.click(btn)
    await waitFor(() => expect(onChange).toHaveBeenCalledWith(true))
    rerender(<ArmSwitch label="Output" on={true} onChange={onChange} />)
    await waitFor(() => expect(btn).toHaveAttribute('data-state', 'on'))
    fireEvent.click(btn)
    await waitFor(() => expect(onChange).toHaveBeenLastCalledWith(false))
  })

  it('disarms after 3 seconds', () => {
    vi.useFakeTimers()
    render(<ArmSwitch label="Input" on={false} onChange={vi.fn()} />)
    const btn = screen.getByRole('button', { name: /Input/ })
    fireEvent.click(btn)
    expect(btn).toHaveAttribute('data-state', 'armed')
    act(() => { vi.advanceTimersByTime(3100) })
    expect(btn).toHaveAttribute('data-state', 'off')
  })
})
