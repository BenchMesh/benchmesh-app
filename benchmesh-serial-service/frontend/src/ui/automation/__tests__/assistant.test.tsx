import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { AutomationDrawer, resetDraft } from '../AutomationDrawer'
import { RunProvider } from '../RunProvider'
import { flowDoc, routeFetch } from './fakes'

afterEach(() => { vi.restoreAllMocks(); resetDraft() })

describe('assistant', () => {
  it('assembles a flow and loads it into the editor as unsaved, with the explanation and issues', async () => {
    const posts: any[] = []
    routeFetch({
      'POST /automation/assist': (b) => {
        posts.push(b)
        return { body: { flow: flowDoc({ name: 'Assembled ramp' }), explanation: 'Sets the PSU to target and reads VOUT.', issues: [{ path: 'steps[1]', level: 'warning', message: 'psu offline' }], attempts: 2 } }
      },
    })
    render(<RunProvider><AutomationDrawer /></RunProvider>)
    const panel = await screen.findByRole('region', { name: 'Assistant' })
    expect(await within(panel).findByText('claude-x')).toBeTruthy()
    expect(within(panel).getByText(/no API key yet/)).toBeTruthy()
    expect(within(panel).getByText(/on your own account and at your expense/)).toBeTruthy()
    fireEvent.change(within(panel).getByLabelText('Describe the automation…'), { target: { value: 'ramp the psu to 5 V' } })
    fireEvent.click(within(panel).getByRole('button', { name: 'Assemble' }))
    expect(await within(panel).findByText('Sets the PSU to target and reads VOUT.')).toBeTruthy()
    expect(posts[0]).toEqual({ instruction: 'ramp the psu to 5 V' }) // no flow yet
    expect(within(panel).getByText(/psu offline/)).toBeTruthy()
    expect(screen.getByLabelText('Flow name')).toHaveValue('Assembled ramp')
    expect(screen.getByTestId('unsaved')).toBeTruthy()
    // refining sends the current flow
    expect(within(panel).getByRole('button', { name: 'Change flow' })).toBeTruthy()
    fireEvent.change(within(panel).getByLabelText('What should change?'), { target: { value: 'also wait 1 s' } })
    fireEvent.click(within(panel).getByRole('button', { name: 'Change flow' }))
    await waitFor(() => expect(posts).toHaveLength(2))
    expect(posts[1].flow.name).toBe('Assembled ramp')
  })

  it('shows a provider error with a link to the settings', async () => {
    routeFetch({ 'POST /automation/assist': () => ({ status: 502, body: { detail: 'no API key for anthropic - add one in Automation → Assistant settings' } }) })
    render(<RunProvider><AutomationDrawer /></RunProvider>)
    const panel = await screen.findByRole('region', { name: 'Assistant' })
    fireEvent.change(within(panel).getByLabelText('Describe the automation…'), { target: { value: 'x' } })
    fireEvent.click(within(panel).getByRole('button', { name: 'Assemble' }))
    const alert = await within(panel).findByRole('alert')
    expect(alert.textContent).toMatch(/no API key for anthropic/)
    fireEvent.click(within(alert).getByRole('button', { name: 'open assistant settings' }))
    expect(await screen.findByRole('region', { name: 'Assistant settings' })).toBeTruthy()
  })
})
