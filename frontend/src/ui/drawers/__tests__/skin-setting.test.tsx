import React from 'react'
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { SkinProvider, SKIN_STORAGE_KEY } from '../../kit'
import { SkinSetting } from '../SkinSetting'

beforeEach(() => localStorage.clear())

describe('SkinSetting (Instruments - Miscellaneous)', () => {
  it('offers the three skins by name and switches and persists the choice', () => {
    render(<SkinProvider><SkinSetting /></SkinProvider>)
    const group = screen.getByRole('group', { name: 'Skin' })
    expect([...group.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Reflective LCD', 'LED segment', 'Cockpit glass'])
    expect(screen.getByRole('button', { name: 'Reflective LCD' })).toHaveAttribute('aria-pressed', 'true')
    fireEvent.click(screen.getByRole('button', { name: 'Cockpit glass' }))
    expect(document.documentElement.dataset.skin).toBe('glass')
    expect(localStorage.getItem(SKIN_STORAGE_KEY)).toBe('glass')
    expect(screen.getByRole('button', { name: 'Cockpit glass' })).toHaveAttribute('aria-pressed', 'true')
  })
})
