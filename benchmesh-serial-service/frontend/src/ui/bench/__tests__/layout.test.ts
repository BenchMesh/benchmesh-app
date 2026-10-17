import { describe, it, expect } from 'vitest'
import { reconcile, move, setSize, hide, show, columnsFor, spanFor, panelsOf, EMPTY_LAYOUT } from '../layout'

const instruments = [
  { id: 'tenma', classes: [{ class: 'PSU', channels: ['/instruments/PSU/tenma/1'] }] },
  { id: 'spm', classes: [{ class: 'DMM', channels: [] }, { class: 'PSU', channels: [] }] },
  { id: 'dge', classes: [{ class: 'AWG', channels: [] }] },
]

describe('panelsOf', () => {
  it('makes one panel per instrument class', () => {
    expect(panelsOf(instruments).map((p) => p.key)).toEqual(['tenma:PSU', 'spm:DMM', 'spm:PSU', 'dge:AWG'])
  })
})

describe('reconcile', () => {
  it('appends new panels with their class default size and drops removed ones', () => {
    const l = reconcile({ order: ['gone:PSU', 'dge:AWG'], size: { 'gone:PSU': 's', 'dge:AWG': 's' }, hidden: ['gone:PSU'] }, panelsOf(instruments))
    expect(l.order).toEqual(['dge:AWG', 'tenma:PSU', 'spm:DMM', 'spm:PSU'])
    expect(l.size).toEqual({ 'dge:AWG': 's', 'tenma:PSU': 'm', 'spm:DMM': 'm', 'spm:PSU': 'm' })
    expect(l.hidden).toEqual([])
  })
  it('is stable when nothing changed', () => {
    const l = reconcile(EMPTY_LAYOUT, panelsOf(instruments))
    expect(reconcile(l, panelsOf(instruments))).toBe(l)
  })
})

describe('arranging', () => {
  const l = reconcile(EMPTY_LAYOUT, panelsOf(instruments))
  it('moves a panel to a new position', () => {
    expect(move(l, 'dge:AWG', 0).order).toEqual(['dge:AWG', 'tenma:PSU', 'spm:DMM', 'spm:PSU'])
    expect(move(l, 'tenma:PSU', 99).order).toEqual(['spm:DMM', 'spm:PSU', 'dge:AWG', 'tenma:PSU'])
  })
  it('sizes, hides and shows panels', () => {
    expect(setSize(l, 'spm:DMM', 's').size['spm:DMM']).toBe('s')
    const h = hide(l, 'spm:DMM')
    expect(h.hidden).toEqual(['spm:DMM'])
    expect(show(h, 'spm:DMM').hidden).toEqual([])
  })
})

describe('responsive grid', () => {
  it('fits as many columns as the width allows', () => {
    expect(columnsFor(388)).toBe(1) // phone: everything stacks
    expect(columnsFor(788)).toBe(2) // tablet: medium panels take the full row
    expect(columnsFor(1334)).toBe(4) // laptop: two medium panels per row
    expect(columnsFor(1888)).toBe(6) // full HD: three medium panels per row
  })
  it('clamps spans to the available columns', () => {
    expect(spanFor('l', 5)).toBe(3)
    expect(spanFor('l', 2)).toBe(2)
    expect(spanFor('full', 5)).toBe(5)
    expect(spanFor('m', 1)).toBe(1)
  })
})
