import { describe, it, expect } from 'vitest'
import { apiError } from '../apiError'

describe('apiError', () => {
  it('reads detail, else the status', async () => {
    expect(await apiError(new Response(JSON.stringify({ detail: 'duplicate id' }), { status: 409 }))).toBe('duplicate id')
    expect(await apiError(new Response('oops', { status: 500 }))).toBe('HTTP 500')
  })
})
