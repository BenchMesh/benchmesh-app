import React from 'react'
import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { SkinProvider, useSkin, loadSkin, SKIN_STORAGE_KEY, Skin } from '../skin'
import { Readout, Selector } from '../controls'

function Switch() {
  const { skin, setSkin } = useSkin()
  return (
    <Selector label="Skin" value={skin} onChange={(v) => setSkin(v as Skin)}
      options={[{ value: 'lcd', label: 'LCD' }, { value: 'led', label: 'LED' }, { value: 'glass', label: 'Glass' }]} />
  )
}

beforeEach(() => {
  localStorage.clear()
  delete document.documentElement.dataset.skin
})
afterEach(() => vi.restoreAllMocks())

describe('skin', () => {
  it('defaults to the reflective LCD and marks the root element', () => {
    expect(loadSkin()).toBe('lcd')
    render(<SkinProvider><Switch /></SkinProvider>)
    expect(document.documentElement.dataset.skin).toBe('lcd')
    expect(screen.getByRole('button', { name: 'LCD' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('switching sets data-skin on the root and persists the choice', () => {
    render(<SkinProvider><Switch /></SkinProvider>)
    fireEvent.click(screen.getByRole('button', { name: 'LED' }))
    expect(document.documentElement.dataset.skin).toBe('led')
    expect(localStorage.getItem(SKIN_STORAGE_KEY)).toBe('led')
    expect(screen.getByRole('button', { name: 'LED' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('restores a stored skin and ignores invalid values', () => {
    localStorage.setItem(SKIN_STORAGE_KEY, 'glass')
    expect(loadSkin()).toBe('glass')
    render(<SkinProvider><Switch /></SkinProvider>)
    expect(document.documentElement.dataset.skin).toBe('glass')
    localStorage.setItem(SKIN_STORAGE_KEY, 'neon')
    expect(loadSkin()).toBe('lcd')
  })

  it('falls back to the default and keeps working when storage throws', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('denied') })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('denied') })
    expect(loadSkin()).toBe('lcd')
    render(<SkinProvider><Switch /></SkinProvider>)
    fireEvent.click(screen.getByRole('button', { name: 'Glass' }))
    expect(document.documentElement.dataset.skin).toBe('glass')
  })
})

describe('Readout per skin', () => {
  it('draws seven segments on canvas outside the glass skin', () => {
    const { container } = render(<SkinProvider><Readout tag="VOUT" text=" 4.94" unit="V" tone="volt" /></SkinProvider>)
    expect(container.querySelector('canvas')).not.toBeNull()
    expect(container.querySelector('.kit-numtext')).toBeNull()
  })

  it('renders the number as text in the tone colour in the glass skin', () => {
    localStorage.setItem(SKIN_STORAGE_KEY, 'glass')
    const { container } = render(<SkinProvider><Readout tag="VOUT" text=" 4.94" unit="V" tone="volt" label="Output voltage" /></SkinProvider>)
    expect(container.querySelector('canvas')).toBeNull()
    const txt = screen.getByRole('img', { name: 'Output voltage 4.94' })
    expect(txt).toHaveTextContent('4.94')
    expect(txt.style.color).toBe('var(--kit-volt)')
    // the decimal point gets its own narrow slot
    expect(txt.querySelector('.kit-numtext__dp')).toHaveTextContent('.')
  })

  it('switches between canvas and text when the skin changes', () => {
    const { container } = render(<SkinProvider><Switch /><Readout tag="I" text="0.512" unit="A" /></SkinProvider>)
    expect(container.querySelector('canvas')).not.toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Glass' }))
    expect(container.querySelector('canvas')).toBeNull()
    expect(container.querySelector('.kit-numtext')).toHaveStyle({ color: 'var(--kit-seg-on)' })
  })
})
