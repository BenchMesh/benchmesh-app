/**
 * Context menu at the pointer (or at an element for the keyboard), clamped to the viewport.
 * Keyboard: ↑/↓ (wrapping), Home/End, Enter/Space choose, Escape or Tab close. Closes on a
 * pointer press outside, on scroll and on resize; focus returns to where it was.
 * Checkable items (`checked` set) are `menuitemcheckbox`; `confirm` shows the kit's
 * confirmation pattern on the item while a change is in flight.
 */
import React, { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { ConfirmState } from './confirm'
import { NOT_CONFIRMED } from './confirm'

export interface MenuItem {
  key: string
  label: React.ReactNode
  /** Accessible name when `label` is not plain text. */
  name?: string
  /** Set for a checkable item. */
  checked?: boolean
  disabled?: boolean
  /** Secondary text after the label (e.g. "paused"). */
  hint?: string
  confirm?: ConfirmState
  error?: string | null
  /** Draw a separator above this item. */
  separator?: boolean
  /** Keep the menu open after choosing (checkable items whose change is confirmed in place). */
  keepOpen?: boolean
  onSelect: () => void
}

const MARGIN = 4

/** Top-left corner that keeps a `w`×`h` box at (x, y) inside a `vw`×`vh` viewport. */
export function clampToViewport(x: number, y: number, w: number, h: number, vw: number, vh: number) {
  return {
    left: Math.max(MARGIN, Math.min(x, vw - w - MARGIN)),
    top: Math.max(MARGIN, Math.min(y, vh - h - MARGIN)),
  }
}

export function ContextMenu({ x, y, items, label, onClose }: {
  x: number; y: number; items: MenuItem[]; label: string; onClose: () => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState({ left: x, top: y })
  const opener = useRef<Element | null>(null)
  const closeRef = useRef(onClose)
  useLayoutEffect(() => { closeRef.current = onClose })

  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const r = el.getBoundingClientRect()
    setPos(clampToViewport(x, y, r.width, r.height, window.innerWidth, window.innerHeight))
  }, [x, y, items.length])

  // focus the first item; give focus back on close
  useEffect(() => {
    opener.current = document.activeElement
    ref.current?.querySelector<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])')?.focus()
    return () => {
      const o = opener.current as HTMLElement | null
      if (o && typeof o.focus === 'function' && document.contains(o)) o.focus()
    }
  }, [])

  useEffect(() => {
    const inside = (t: EventTarget | null) => t instanceof Node && !!ref.current?.contains(t)
    const onDown = (e: PointerEvent | MouseEvent) => { if (!inside(e.target)) closeRef.current() }
    // A scroll that happened just before opening still delivers its event afterwards (events
    // are dispatched with the next frame): only a scroll after opening closes the menu.
    const opened = performance.now()
    const page = { x: window.scrollX, y: window.scrollY }
    const onScroll = (e: Event) => {
      if (inside(e.target)) return
      const isPage = e.target === document || e.target === document.documentElement || e.target === document.body
      if (isPage ? window.scrollX === page.x && window.scrollY === page.y : performance.now() - opened < 150) return
      closeRef.current()
    }
    const onResize = () => closeRef.current()
    document.addEventListener('pointerdown', onDown, true)
    document.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onResize)
    window.addEventListener('blur', onResize)
    return () => {
      document.removeEventListener('pointerdown', onDown, true)
      document.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onResize)
      window.removeEventListener('blur', onResize)
    }
  }, [])

  const choose = (it: MenuItem) => {
    if (it.disabled) return
    it.onSelect()
    if (!it.keepOpen) onClose()
  }

  const onKeyDown = (e: React.KeyboardEvent) => {
    const els = Array.from(ref.current?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])') ?? [])
    const i = els.indexOf(document.activeElement as HTMLElement)
    const focusAt = (j: number) => els[(j + els.length) % els.length]?.focus()
    switch (e.key) {
      case 'ArrowDown': e.preventDefault(); focusAt(i + 1); break
      case 'ArrowUp': e.preventDefault(); focusAt(i < 0 ? -1 : i - 1); break
      case 'Home': e.preventDefault(); focusAt(0); break
      case 'End': e.preventDefault(); focusAt(-1); break
      case 'Escape': e.preventDefault(); e.stopPropagation(); onClose(); break
      case 'Tab': e.preventDefault(); onClose(); break
      case 'Enter':
      case ' ': {
        e.preventDefault()
        const key = (document.activeElement as HTMLElement | null)?.dataset.key
        const it = items.find((x) => x.key === key)
        if (it) choose(it)
        break
      }
    }
  }

  return createPortal(
    <div ref={ref} className="kit-menu" role="menu" aria-label={label} style={{ left: pos.left, top: pos.top }}
      onKeyDown={onKeyDown} onContextMenu={(e) => e.preventDefault()}>
      {items.map((it) => {
        const checkable = it.checked !== undefined
        return (
          <React.Fragment key={it.key}>
            {it.separator && <div className="kit-menu__sep" role="separator" />}
            <div role={checkable ? 'menuitemcheckbox' : 'menuitem'} tabIndex={-1} data-key={it.key}
              className="kit-menu__item" aria-checked={checkable ? it.checked : undefined}
              aria-disabled={it.disabled || undefined} aria-label={it.name}
              aria-busy={it.confirm === 'waiting' || undefined} data-confirm={it.confirm && it.confirm !== 'idle' ? it.confirm : undefined}
              title={it.error ?? (it.confirm === 'unconfirmed' ? NOT_CONFIRMED : undefined)}
              onClick={() => choose(it)} onMouseEnter={(e) => e.currentTarget.focus()}>
              <span className="kit-menu__check" aria-hidden="true">{checkable ? (it.checked ? '✓' : '') : ''}</span>
              <span className="kit-menu__label">{it.label}</span>
              {it.hint && <span className="kit-menu__hint">{it.hint}</span>}
              {it.error && <span className="kit-error kit-menu__err" role="alert">{it.error}</span>}
            </div>
          </React.Fragment>
        )
      })}
    </div>,
    document.body,
  )
}

/** Opens a context menu on right-click, the context-menu key or Shift+F10. */
export function useContextMenu() {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null)
  const props = {
    onContextMenu: (e: React.MouseEvent) => {
      e.preventDefault()
      e.stopPropagation()
      // keyboard-generated contextmenu events carry no pointer position
      if (e.clientX === 0 && e.clientY === 0) {
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
        setAt({ x: r.left, y: r.bottom })
      } else setAt({ x: e.clientX, y: e.clientY })
    },
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
        e.preventDefault()
        e.stopPropagation()
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
        setAt({ x: r.left, y: r.bottom })
      }
    },
  }
  return { at, props, close: () => setAt(null) }
}
