import { describe, it, expect } from 'vitest'
import { NO_ANSWER, offlineReason } from '../offline'

describe('offlineReason', () => {
  it('says a silent port probably means the instrument is switched off', () => {
    expect(offlineReason({ status: 'dead', last_error: 'timeout (0 bytes received)' })).toBe(NO_ANSWER)
  })

  it('passes other errors through and says nothing without one', () => {
    expect(offlineReason({ last_error: 'not attached: no serial port matches usb 1a86:7523' })).toBe('not attached: no serial port matches usb 1a86:7523')
    expect(offlineReason({ status: 'unknown' })).toBeNull()
    expect(offlineReason(undefined)).toBeNull()
    expect(offlineReason({ last_error: '  ' })).toBeNull()
  })
})
