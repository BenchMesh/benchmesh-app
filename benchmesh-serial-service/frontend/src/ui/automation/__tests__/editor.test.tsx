import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { AutomationDrawer, resetDraft } from '../AutomationDrawer'
import { RunProvider } from '../RunProvider'
import { flowDoc, routeFetch } from './fakes'

afterEach(() => { vi.restoreAllMocks(); resetDraft() })

const renderDrawer = () => render(<RunProvider><AutomationDrawer /></RunProvider>)

const step = (path: string) => screen.getByRole('group', { name: new RegExp(`^Step ${path.replace(/[[\]]/g, '\\$&')}:`) })

describe('flow editor', () => {
  it('loads a saved flow, edits a set step through the pickers and saves it', async () => {
    const calls = routeFetch({
      'GET /automation/flows': () => ({ body: [{ id: 'ramp', name: 'Ramp', description: '', updated_at: null }] }),
      'GET /automation/flows/ramp': () => ({ body: flowDoc() }),
      'PUT /automation/flows/ramp': (b) => ({ body: { id: 'ramp', flow: b, issues: [] } }),
    })
    renderDrawer()
    fireEvent.click(await screen.findByRole('button', { name: 'Ramp' }))
    const card = await waitFor(() => step('steps[0]'))
    expect(screen.getByLabelText('Parameter 1 name')).toHaveValue('target')

    // switch the setter and go from expression to a literal value
    const w = within(card)
    expect(w.getByLabelText(/Value expression/)).toHaveValue('target')
    fireEvent.change(w.getByLabelText('Setting'), { target: { value: 'output' } })
    expect(within(step('steps[0]')).getByText('⚡ energizes')).toBeTruthy()
    fireEvent.change(within(step('steps[0]')).getByLabelText('Setting'), { target: { value: 'current' } })
    expect(within(step('steps[0]')).getByText('0 – 15 A')).toBeTruthy()
    fireEvent.click(within(step('steps[0]')).getByRole('button', { name: 'Value' }))
    fireEvent.change(within(step('steps[0]')).getByLabelText('Value (A)'), { target: { value: '1.25' } })
    expect(screen.getByTestId('unsaved')).toBeTruthy()

    const save = screen.getByRole('button', { name: 'Save' })
    await waitFor(() => expect(save).not.toBeDisabled())
    fireEvent.click(save)
    await screen.findByText('Saved')
    const put = calls.find((c) => c.method === 'PUT')!
    expect(put.path).toBe('/automation/flows/ramp')
    expect(put.body.steps[0]).toEqual({ do: 'set', device: 'psu', class: 'PSU', channel: 1, op: 'current', value: 1.25 })
    expect(screen.queryByTestId('unsaved')).toBeNull()
  })

  it('moves the instrument to another device and offers its channels', async () => {
    routeFetch({})
    renderDrawer()
    fireEvent.change(await screen.findByLabelText('Add step at the start of steps'), { target: { value: 'set' } })
    const card = step('steps[0]')
    await waitFor(() => expect(within(card).getByLabelText('Instrument')).toHaveValue('psu'))
    fireEvent.change(within(card).getByLabelText('Instrument'), { target: { value: 'awg' } })
    const c = within(step('steps[0]'))
    expect(c.getByLabelText('Class')).toHaveValue('AWG')
    expect(c.getByLabelText('Setting')).toHaveValue('frequency')
    fireEvent.change(c.getByLabelText('Channel'), { target: { value: '2' } })
    fireEvent.click(screen.getByRole('button', { name: 'JSON' }))
    const doc = JSON.parse((screen.getByLabelText('Flow document (JSON)') as HTMLTextAreaElement).value)
    expect(doc.steps[0]).toMatchObject({ device: 'awg', class: 'AWG', channel: 2, op: 'frequency' })
  })

  it('shows validation issues at the step of their path; errors block Save and Run', async () => {
    routeFetch({
      'GET /automation/flows': () => ({ body: [{ id: 'ramp', name: 'Ramp', description: '', updated_at: null }] }),
      'GET /automation/flows/ramp': () => ({ body: flowDoc({ steps: [...flowDoc().steps!, { do: 'if', condition: 'v > target', then: [{ do: 'log', message: 'hi {v}' }] }] }) }),
      'POST /automation/validate': () => ({ body: { ok: false, energizes: [], issues: [
        { path: 'steps[2].then[0]', level: 'error', message: 'unknown variable q' },
        { path: 'steps[1]', level: 'warning', message: 'device psu is offline' },
        { path: 'params[0]', level: 'error', message: 'bad parameter' },
      ] } }),
    })
    renderDrawer()
    fireEvent.click(await screen.findByRole('button', { name: 'Ramp' }))
    expect(await within(await waitFor(() => step('steps[2].then[0]'))).findByText(/unknown variable q/)).toBeTruthy()
    expect(within(step('steps[1]')).getByText(/device psu is offline/)).toBeTruthy()
    expect(within(step('steps[2]')).queryByText(/unknown variable q/)).toBeTruthy() // nested card is inside its parent
    expect(step('steps[2].then[0]').dataset.level).toBe('error')
    expect(step('steps[1]').dataset.level).toBe('warning')
    expect(step('steps[0]').dataset.level).toBeUndefined()
    expect(screen.getByText(/bad parameter/)).toBeTruthy()
    expect(screen.getByTestId('issue-summary').textContent).toBe('2 errors, 1 warning')
    // edit so that Save would be possible if not for the errors
    fireEvent.change(screen.getByLabelText('Parameter 1 value'), { target: { value: '6' } })
    await waitFor(() => expect(screen.getByTestId('issue-summary').textContent).toBe('2 errors, 1 warning'))
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
    expect(screen.getByRole('button', { name: /Run/ })).toBeDisabled()
    expect(screen.getByText('Fix 2 errors first')).toBeTruthy()
  })

  it('keeps the JSON view in sync and shows parse errors', async () => {
    routeFetch({})
    renderDrawer()
    fireEvent.click(await screen.findByRole('button', { name: 'JSON' }))
    const area = screen.getByLabelText('Flow document (JSON)')
    fireEvent.change(area, { target: { value: '{ "schema": ' } })
    expect(screen.getByRole('alert').textContent).toMatch(/^JSON:/)
    fireEvent.change(area, { target: { value: JSON.stringify(flowDoc({ name: 'From JSON' })) } })
    expect(screen.queryByText(/^JSON:/)).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Flow' }))
    expect(screen.getByLabelText('Flow name')).toHaveValue('From JSON')
    expect(step('cleanup[0]')).toBeTruthy()
  })

  it('shows 422 issues from saving a new flow', async () => {
    const calls = routeFetch({
      'POST /automation/flows': () => ({ status: 422, body: { detail: 'the flow has errors', issues: [{ path: 'steps[0]', level: 'error', message: 'server says no' }] } }),
    })
    renderDrawer()
    fireEvent.change(await screen.findByLabelText('Add step at the start of steps'), { target: { value: 'log' } })
    const save = screen.getByRole('button', { name: 'Save' })
    await waitFor(() => expect(save).not.toBeDisabled())
    fireEvent.click(save)
    expect(await screen.findByText('the flow has errors')).toBeTruthy()
    expect(within(step('steps[0]')).getByText(/server says no/)).toBeTruthy()
    expect(calls.some((c) => c.method === 'POST' && c.path === '/automation/flows')).toBe(true)
  })
})
