import React, { useState } from 'react'
import { describe, it, expect, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { ContextMenu, MenuItem, clampToViewport, useContextMenu } from '../ContextMenu'

function Host({ onPick = vi.fn() }: { onPick?: (k: string) => void }) {
  const menu = useContextMenu()
  const [checked, setChecked] = useState(false)
  const items: MenuItem[] = [
    { key: 'a', label: 'Alpha', checked, keepOpen: true, onSelect: () => { setChecked((c) => !c); onPick('a') } },
    { key: 'b', label: 'Beta', disabled: true, onSelect: () => onPick('b') },
    { key: 'c', label: 'Gamma', separator: true, onSelect: () => onPick('c') },
  ]
  return (
    <div>
      <div tabIndex={0} {...menu.props}>Target</div>
      <button type="button">Outside</button>
      {menu.at && <ContextMenu x={menu.at.x} y={menu.at.y} label="Test menu" items={items} onClose={menu.close} />}
    </div>
  )
}

describe('ContextMenu', () => {
  it('opens on right-click at the pointer and focuses the first item', () => {
    render(<Host />)
    fireEvent.contextMenu(screen.getByText('Target'), { clientX: 40, clientY: 50 })
    const menu = screen.getByRole('menu', { name: 'Test menu' })
    expect(menu.style.left).toBe('40px')
    expect(screen.getByRole('menuitemcheckbox', { name: 'Alpha' })).toHaveFocus()
    expect(screen.getByRole('menuitemcheckbox', { name: 'Alpha' })).toHaveAttribute('aria-checked', 'false')
  })

  it('opens from the keyboard (context-menu key, Shift+F10)', () => {
    render(<Host />)
    fireEvent.keyDown(screen.getByText('Target'), { key: 'ContextMenu' })
    expect(screen.getByRole('menu')).toBeInTheDocument()
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' })
    expect(screen.queryByRole('menu')).toBeNull()
    fireEvent.keyDown(screen.getByText('Target'), { key: 'F10', shiftKey: true })
    expect(screen.getByRole('menu')).toBeInTheDocument()
  })

  it('arrows skip disabled items and wrap; Enter chooses; checkable items toggle and stay open', () => {
    const onPick = vi.fn()
    render(<Host onPick={onPick} />)
    fireEvent.contextMenu(screen.getByText('Target'), { clientX: 10, clientY: 10 })
    const menu = screen.getByRole('menu')
    fireEvent.keyDown(menu, { key: 'ArrowDown' })
    expect(screen.getByRole('menuitem', { name: 'Gamma' })).toHaveFocus()
    fireEvent.keyDown(menu, { key: 'ArrowDown' })
    expect(screen.getByRole('menuitemcheckbox', { name: 'Alpha' })).toHaveFocus()
    fireEvent.keyDown(menu, { key: 'ArrowUp' })
    expect(screen.getByRole('menuitem', { name: 'Gamma' })).toHaveFocus()
    fireEvent.keyDown(menu, { key: 'ArrowUp' })
    fireEvent.keyDown(menu, { key: 'Enter' })
    expect(onPick).toHaveBeenCalledWith('a')
    expect(screen.getByRole('menuitemcheckbox', { name: 'Alpha' })).toHaveAttribute('aria-checked', 'true')
    fireEvent.keyDown(menu, { key: 'End' })
    fireEvent.keyDown(menu, { key: 'Enter' })
    expect(onPick).toHaveBeenCalledWith('c')
    expect(screen.queryByRole('menu')).toBeNull() // plain items close the menu
  })

  it('a disabled item does nothing; a press outside closes', () => {
    const onPick = vi.fn()
    render(<Host onPick={onPick} />)
    fireEvent.contextMenu(screen.getByText('Target'), { clientX: 10, clientY: 10 })
    fireEvent.click(screen.getByRole('menuitem', { name: 'Beta' }))
    expect(onPick).not.toHaveBeenCalled()
    fireEvent.pointerDown(screen.getByRole('button', { name: 'Outside' }))
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('closes on scroll and returns focus to the target', () => {
    render(<Host />)
    const target = screen.getByText('Target')
    target.focus()
    fireEvent.keyDown(target, { key: 'ContextMenu' })
    // a late event of a scroll from before opening (page did not move): stays open
    fireEvent.scroll(document)
    expect(screen.getByRole('menu')).toBeInTheDocument()
    const y = Object.getOwnPropertyDescriptor(window, 'scrollY')
    Object.defineProperty(window, 'scrollY', { value: 50, configurable: true })
    try {
      fireEvent.scroll(document)
    } finally {
      if (y) Object.defineProperty(window, 'scrollY', y)
    }
    expect(screen.queryByRole('menu')).toBeNull()
    expect(target).toHaveFocus()
  })

  it('closes when a scroll container scrolls after opening', () => {
    render(<Host />)
    fireEvent.contextMenu(screen.getByText('Target'), { clientX: 10, clientY: 10 })
    const now = performance.now()
    const spy = vi.spyOn(performance, 'now').mockReturnValue(now + 1000)
    fireEvent.scroll(screen.getByRole('button', { name: 'Outside' }))
    spy.mockRestore()
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('shows a request in flight on the item', () => {
    render(<ContextMenu x={0} y={0} label="m" onClose={() => {}}
      items={[{ key: 'x', label: 'X', checked: true, confirm: 'waiting', onSelect: () => {} }, { key: 'y', label: 'Y', checked: false, error: 'Nope', onSelect: () => {} }]} />)
    expect(screen.getByRole('menuitemcheckbox', { name: 'X' })).toHaveAttribute('data-confirm', 'waiting')
    expect(screen.getByRole('alert')).toHaveTextContent('Nope')
  })

  it('is clamped to the viewport', () => {
    expect(clampToViewport(990, 750, 200, 100, 1000, 800)).toEqual({ left: 796, top: 696 })
    expect(clampToViewport(-20, 10, 200, 100, 1000, 800)).toEqual({ left: 4, top: 10 })
    expect(clampToViewport(10, 10, 500, 100, 300, 800).left).toBe(4)
  })
})
