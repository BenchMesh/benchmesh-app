import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, within } from '@testing-library/react'
import { DisclaimerModal } from '../DisclaimerModal'

describe('DisclaimerModal', () => {
  it('renders nothing when closed', () => {
    const { container } = render(<DisclaimerModal isOpen={false} onAccept={() => {}} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('welcomes first: logo, thanks, in development, feedback contact', () => {
    render(<DisclaimerModal isOpen onAccept={() => {}} />)
    const dialog = screen.getByRole('dialog', { name: /Thank you for trying BenchMesh/ })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    const welcome = within(dialog).getByRole('region', { name: /Thank you/ })
    expect(welcome.querySelector('svg')).not.toBeNull()
    expect(welcome).toHaveTextContent(/still in development/)
    expect(welcome).toHaveTextContent(/feedback/)
    const mail = within(welcome).getByRole('link', { name: 'support@benchmesh.online' })
    expect(mail).toHaveAttribute('href', 'mailto:support@benchmesh.online')
    // the welcome comes before the safety information
    const safety = within(dialog).getByRole('heading', { name: /Important Safety Information/ })
    expect(welcome.compareDocumentPosition(safety) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('accepts with or without "don\'t show again"', () => {
    const onAccept = vi.fn()
    render(<DisclaimerModal isOpen onAccept={onAccept} />)
    fireEvent.click(screen.getByRole('button', { name: 'I Agree and Continue' }))
    expect(onAccept).toHaveBeenLastCalledWith(false)
    fireEvent.click(screen.getByRole('checkbox', { name: "Don't show this message again" }))
    fireEvent.click(screen.getByRole('button', { name: 'I Agree and Continue' }))
    expect(onAccept).toHaveBeenLastCalledWith(true)
  })
})
