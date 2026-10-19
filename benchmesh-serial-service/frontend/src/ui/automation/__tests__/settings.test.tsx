import React from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { automationClient } from '../client'
import { SettingsTab } from '../SettingsTab'
import { routeFetch } from './fakes'

afterEach(() => vi.restoreAllMocks())

const KEY = 'sk-ant-api03-SECRETSECRETSECRET-1234'

function fakeServer() {
  const state = { provider: 'anthropic', model: 'claude-x', base_url: '', keys: { anthropic: null as string | null, openai: null, openrouter: null, custom: null } as Record<string, string | null> }
  let mcp = 'read'
  const calls = routeFetch({
    'GET /automation/assistant': () => ({ body: state }),
    'PUT /automation/assistant': (b) => {
      if (b.provider) state.provider = b.provider
      if (b.model !== undefined) state.model = b.model
      if (b.base_url !== undefined) state.base_url = b.base_url
      if (b.api_key !== undefined) state.keys[state.provider] = b.api_key ? `sk-…${b.api_key.slice(-4)}` : null
      return { body: state }
    },
    'GET /automation/assistant/models': () => ({ body: [{ id: 'claude-a', name: 'Claude A' }, { id: 'claude-b', name: 'Claude B' }] }),
    'GET /automation/mcp': () => ({ body: { access: mcp, endpoint: 'http://127.0.0.1:57701/mcp', command: '/opt/benchmesh/benchmesh', args: ['--mcp'] } }),
    'PUT /automation/mcp': (b) => { mcp = b.access; return { body: { access: mcp } } },
  })
  return { state, calls }
}

describe('automation settings', () => {
  it('saves a key and then shows only the hint, never the key', async () => {
    const { calls } = fakeServer()
    const { container } = render(<SettingsTab client={automationClient} />)
    const input = await screen.findByLabelText('API key for Anthropic')
    expect(input).toHaveAttribute('type', 'password')
    fireEvent.change(input, { target: { value: KEY } })
    fireEvent.click(screen.getByRole('button', { name: 'Save key' }))
    expect(await screen.findByText('API key saved')).toBeTruthy()
    expect(calls.find((c) => c.method === 'PUT')!.body).toEqual({ provider: 'anthropic', api_key: KEY })
    expect(input).toHaveValue('')
    expect(screen.getByTestId('key-hint').textContent).toMatch(/sk-…1234/)
    expect(container.innerHTML).not.toContain('SECRET')

    fireEvent.click(within(screen.getByTestId('key-hint')).getByRole('button', { name: 'Remove' }))
    await screen.findByText('API key removed')
    expect(calls.filter((c) => c.method === 'PUT').pop()!.body).toEqual({ provider: 'anthropic', api_key: '' })
    expect(screen.getByTestId('key-hint').textContent).toBe('No key stored.')
  })

  it('loads the model list, picks a model and saves it; free text works too', async () => {
    const { calls } = fakeServer()
    render(<SettingsTab client={automationClient} />)
    const model = await screen.findByLabelText('Model')
    expect(model).toHaveValue('claude-x')
    fireEvent.click(screen.getByRole('button', { name: 'Load models' }))
    const list = await screen.findByLabelText('Models of the provider')
    expect(within(list).getByText('Claude B (claude-b)')).toBeTruthy()
    fireEvent.change(list, { target: { value: 'claude-b' } })
    expect(model).toHaveValue('claude-b')
    fireEvent.submit(model.closest('form')!)
    await screen.findByText('Model saved')
    expect(calls.filter((c) => c.method === 'PUT').pop()!.body).toEqual({ provider: 'anthropic', model: 'claude-b' })
    fireEvent.change(model, { target: { value: 'my-own-model' } })
    fireEvent.submit(model.closest('form')!)
    await waitFor(() => expect(calls.filter((c) => c.method === 'PUT').pop()!.body.model).toBe('my-own-model'))
  })

  it('switches the provider and saves a base URL', async () => {
    const { calls } = fakeServer()
    render(<SettingsTab client={automationClient} />)
    fireEvent.change(await screen.findByLabelText('Provider'), { target: { value: 'custom' } })
    expect(await screen.findByLabelText('API key for Custom endpoint')).toBeTruthy()
    const base = screen.getByLabelText('Base URL (required)')
    fireEvent.change(base, { target: { value: 'http://localhost:11434/v1' } })
    fireEvent.submit(base.closest('form')!)
    await screen.findByText('Base URL saved')
    expect(calls.filter((c) => c.method === 'PUT').map((c) => c.body)).toEqual([{ provider: 'custom' }, { base_url: 'http://localhost:11434/v1' }])
  })

  it('sets MCP access and shows the connection snippets', async () => {
    const { calls } = fakeServer()
    render(<SettingsTab client={automationClient} />)
    const read = await screen.findByRole('radio', { name: /Read/ })
    expect(read).toBeChecked()
    fireEvent.click(screen.getByRole('radio', { name: /Control/ }))
    await waitFor(() => expect(screen.getByRole('radio', { name: /Control/ })).toBeChecked())
    expect(calls.find((c) => c.path === '/automation/mcp' && c.method === 'PUT')!.body).toEqual({ access: 'control' })
    expect(screen.getByText('claude mcp add --transport http benchmesh http://127.0.0.1:57701/mcp')).toBeTruthy()
    expect(screen.getByText(/"command": "\/opt\/benchmesh\/benchmesh"/)).toBeTruthy()
    expect(screen.getByText(/"url": "http:\/\/127.0.0.1:57701\/mcp"/)).toBeTruthy()
    const writeText = vi.fn(() => Promise.resolve())
    Object.assign(navigator, { clipboard: { writeText } })
    fireEvent.click(screen.getByRole('button', { name: 'Copy: Claude Code' }))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('claude mcp add --transport http benchmesh http://127.0.0.1:57701/mcp'))
  })
})
