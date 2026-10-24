/** Recording views (`views.tsx`): the chart, or a view a distribution adds (`extensions.recordingViews`). */
import React from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import type { RecordingView, RecordingViewProps } from '../../../extensions/contract'
import { RecordingPlot, useRecordingView, VIEW_STORAGE_KEY, ViewSwitch } from '../views'
import { recording } from './fakes'

beforeEach(() => localStorage.clear())

const seen: RecordingViewProps[] = []
const lanes: RecordingView = {
  id: 'lanes', label: 'Lanes', title: 'One lane per reading',
  component: (p) => { seen.push(p); return <div role="img" aria-label="Lanes view">{p.series.length} series</div> },
}
const broken: RecordingView = { id: 'broken', label: 'Broken', component: () => { throw new Error('boom') } }

const data: RecordingViewProps = { recording: recording(), series: [], hidden: new Set(), live: true }

function Host({ views }: { views: RecordingView[] }) {
  const choice = useRecordingView(views)
  return (
    <>
      <ViewSwitch {...choice} />
      <RecordingPlot view={choice.view} chart={{ option: {}, label: 'The chart' }} data={data} />
    </>
  )
}

describe('recording views', () => {
  it('show only the chart, without a switch, when no view was added', async () => {
    render(<Host views={[]} />)
    expect(screen.queryByRole('group', { name: 'Recording view' })).toBeNull()
    expect(await screen.findByRole('img', { name: 'The chart' })).toBeInTheDocument()
  })

  it('switch between the chart and an added view, remembering the choice', async () => {
    const { unmount } = render(<Host views={[lanes]} />)
    const sw = screen.getByRole('group', { name: 'Recording view' })
    expect(screen.getByRole('button', { name: 'Chart' })).toHaveAttribute('aria-pressed', 'true')
    fireEvent.click(screen.getByRole('button', { name: 'Lanes' }))
    expect(sw).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'Lanes view' })).toBeInTheDocument()
    expect(seen.at(-1)).toMatchObject({ live: true, series: [] })
    expect(localStorage.getItem(VIEW_STORAGE_KEY)).toBe('lanes')
    unmount()
    render(<Host views={[lanes]} />)
    expect(screen.getByRole('img', { name: 'Lanes view' })).toBeInTheDocument()
  })

  it('show the chart when the remembered view is not in this build', async () => {
    localStorage.setItem(VIEW_STORAGE_KEY, 'gone')
    render(<Host views={[lanes]} />)
    expect(await screen.findByRole('img', { name: 'The chart' })).toBeInTheDocument()
  })

  it('fall back to the chart when a view fails', async () => {
    const quiet = vi.spyOn(console, 'error').mockImplementation(() => {})
    localStorage.setItem(VIEW_STORAGE_KEY, 'broken')
    render(<Host views={[broken]} />)
    expect(await screen.findByRole('img', { name: 'The chart' })).toBeInTheDocument()
    quiet.mockRestore()
  })
})
