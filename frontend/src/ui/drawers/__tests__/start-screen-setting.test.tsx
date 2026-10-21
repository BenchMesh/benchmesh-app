import React from 'react'
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { DISCLAIMER_KEY, disclaimerHidden } from '../../disclaimer'
import { StartScreenSetting } from '../StartScreenSetting'

beforeEach(() => localStorage.clear())

describe('StartScreenSetting (Instruments - Miscellaneous)', () => {
  it('brings back a start screen hidden with "don\'t show again"', () => {
    localStorage.setItem(DISCLAIMER_KEY, 'true')
    render(<StartScreenSetting />)
    const box = screen.getByRole('checkbox', { name: /Show at start/ })
    expect(box).not.toBeChecked()
    fireEvent.click(box)
    expect(box).toBeChecked()
    expect(localStorage.getItem(DISCLAIMER_KEY)).toBeNull()
    expect(disclaimerHidden()).toBe(false)
  })

  it('can hide it again', () => {
    render(<StartScreenSetting />)
    const box = screen.getByRole('checkbox', { name: /Show at start/ })
    expect(box).toBeChecked()
    fireEvent.click(box)
    expect(localStorage.getItem(DISCLAIMER_KEY)).toBe('true')
  })
})
