import React from 'react'
import { describe, it, expect, afterEach } from 'vitest'
import { render, fireEvent, cleanup } from '@testing-library/react'
import { suppressNativeContextMenu } from '../nativeMenu'

describe('suppressNativeContextMenu', () => {
  let undo: (() => void) | undefined
  afterEach(() => { undo?.(); undo = undefined; cleanup() })

  it('blocks the native menu everywhere but still lets app handlers run', () => {
    undo = suppressNativeContextMenu()
    let opened = 0
    const { getByText } = render(<div><span>plain</span><button onContextMenu={() => opened++}>reading</button></div>)
    expect(fireEvent.contextMenu(getByText('plain'))).toBe(false)
    expect(fireEvent.contextMenu(getByText('reading'))).toBe(false)
    expect(opened).toBe(1)
  })

  it('can be undone', () => {
    suppressNativeContextMenu()()
    const { getByText } = render(<span>plain</span>)
    expect(fireEvent.contextMenu(getByText('plain'))).toBe(true)
  })
})
