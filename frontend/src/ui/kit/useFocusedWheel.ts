import { RefObject, useEffect, useLayoutEffect, useRef } from 'react'

/**
 * Scroll-wheel adjustment for a control, only while it has focus (so scrolling the page
 * past a control never changes it). Native non-passive listener: React's onWheel is
 * passive and cannot stop the page scrolling while the control is being adjusted.
 * `onStep` gets +1 for wheel up, -1 for wheel down.
 */
export function useFocusedWheel(ref: RefObject<HTMLElement | null>, onStep: (dir: number, e: WheelEvent) => void) {
  const cb = useRef(onStep)
  useLayoutEffect(() => { cb.current = onStep })
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const handler = (e: WheelEvent) => {
      if (!el.contains(document.activeElement) || e.deltaY === 0) return
      e.preventDefault()
      cb.current(e.deltaY < 0 ? 1 : -1, e)
    }
    el.addEventListener('wheel', handler, { passive: false })
    return () => el.removeEventListener('wheel', handler)
  }, [ref])
}
