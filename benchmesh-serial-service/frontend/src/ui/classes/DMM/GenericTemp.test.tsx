import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/react'
import React from 'react'
import { GenericTemp } from './GenericTemp'
import { RequestLogProvider } from '../../RequestLogContext'

describe('GenericTemp', () => {
  it('uses the partial operation names temp_sensor / temp_scale (API adds the set_ prefix)', () => {
    const tempConfig = {
      Sensor: [{ value: 'KITS90', display: 'KITS90' }],
      Scale: [{ symbol: 'C', unit: '°C', display: 'Celsius' }],
    }
    const { container } = render(
      <RequestLogProvider>
        <GenericTemp mode="TEMP" channelPath="/instruments/DMM/dmm-1/1" tempConfig={tempConfig as any} klass="DMM" deviceId="dmm-1" channel="1" />
      </RequestLogProvider>
    )
    const titles = Array.from(container.querySelectorAll('[title]')).map((e) => e.getAttribute('title') ?? '')
    expect(titles.some((t) => t.includes('/dmm-1/1/temp_sensor/'))).toBe(true)
    expect(titles.some((t) => t.includes('/dmm-1/1/temp_scale/'))).toBe(true)
    expect(titles.some((t) => t.includes('set_temp'))).toBe(false)
  })
})
