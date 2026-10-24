import { describe, expect, it } from 'vitest'
import { extensions } from '..'

describe('UI extensions of the open-source build', () => {
  it('adds nothing', () => {
    expect(extensions.root).toEqual([])
    expect(extensions.systemGroups).toEqual([])
    expect(extensions.recordingViews).toEqual([])
    expect(extensions.titleBadges).toEqual([])
    expect(extensions.panelActions).toEqual([])
  })
})
