/**
 * The recordings chart: ECharts (tree-shaken core: line series, grid, tooltip, data zoom) on a
 * canvas. Loaded lazily (`LazyChart`), so the bench's initial bundle does not carry it.
 * The option comes from `chartModel.buildOption`; this component only hosts the instance.
 */
import React, { useEffect, useLayoutEffect, useRef } from 'react'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { DataZoomComponent, GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'

echarts.use([LineChart, GridComponent, TooltipComponent, DataZoomComponent, CanvasRenderer])

// jsdom (unit tests) has no canvas
const canDraw = typeof navigator === 'undefined' || !/jsdom/i.test(navigator.userAgent)

export interface RecordingChartProps {
  option: Record<string, any>
  label: string
  /** The user zoomed or panned (history view): the visible time range. */
  onZoom?: (from: number, to: number) => void
}

export default function RecordingChart({ option, label, onZoom }: RecordingChartProps) {
  const el = useRef<HTMLDivElement>(null)
  const chart = useRef<echarts.ECharts | null>(null)
  const zoomRef = useRef(onZoom)
  useLayoutEffect(() => { zoomRef.current = onZoom })

  useEffect(() => {
    if (!el.current || !canDraw) return
    const c = echarts.init(el.current, undefined, { renderer: 'canvas' })
    chart.current = c
    c.on('datazoom', () => {
      const dz = (c.getOption() as any)?.dataZoom?.[0]
      if (dz && dz.startValue != null && dz.endValue != null) zoomRef.current?.(Number(dz.startValue), Number(dz.endValue))
    })
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(() => c.resize()) : null
    ro?.observe(el.current)
    return () => { ro?.disconnect(); c.dispose(); chart.current = null }
  }, [])

  useEffect(() => {
    chart.current?.setOption(option, { replaceMerge: ['series', 'yAxis', 'dataZoom'] })
  }, [option])

  return <div ref={el} className="rec-chart__canvas" role="img" aria-label={label} />
}
