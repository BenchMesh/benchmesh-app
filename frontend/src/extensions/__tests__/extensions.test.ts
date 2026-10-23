import { describe, expect, it } from 'vitest'
import { extensions } from '..'

describe('UI extensions of the open-source build', () => {
  it('adds nothing', () => {
    expect(extensions.root).toEqual([])
    expect(extensions.systemGroups).toEqual([])
  })
})
