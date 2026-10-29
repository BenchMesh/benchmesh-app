/**
 * A model's reported problem as a short mark ("Issue reported", "Fix available", "Fix coming") with
 * a card: hover or focus shows it, a press keeps it open (touch, remote browsers), Escape or a
 * press elsewhere closes it. The card is placed on the page (`fixed`), so a scrolling drawer or a
 * panel never cuts it off; it closes when the page scrolls.
 */
import { useEffect, useId, useRef, useState } from 'react'
import type { CommunityIssue } from '../types'
import { issueCard, issueLabel, issueTone } from './issue'

const CARD_W = 280
const EDGE = 8

export function IssueMark({ issue, testId }: { issue: CommunityIssue; testId?: string }) {
  const id = useId()
  const mark = useRef<HTMLButtonElement>(null)
  const [hover, setHover] = useState(false)
  const [pinned, setPinned] = useState(false)
  const [at, setAt] = useState<{ left: number; top: number } | null>(null)
  const open = hover || pinned

  useEffect(() => {
    if (!open) return
    const r = mark.current?.getBoundingClientRect()
    if (r) setAt({ left: Math.max(EDGE, Math.min(r.left, window.innerWidth - CARD_W - EDGE)), top: r.bottom + 6 })
    const close = () => { setHover(false); setPinned(false) }
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close() }
    const onDown = (e: PointerEvent) => { if (!mark.current?.contains(e.target as Node)) close() }
    window.addEventListener('keydown', onKey)
    window.addEventListener('pointerdown', onDown)
    window.addEventListener('scroll', close, true)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('pointerdown', onDown)
      window.removeEventListener('scroll', close, true)
    }
  }, [open])

  const card = issueCard(issue)
  return (
    <>
      <button ref={mark} type="button" className="bench-issue" data-tone={issueTone(issue)} data-testid={testId}
        aria-expanded={open} aria-describedby={open ? id : undefined}
        onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        onFocus={() => setHover(true)} onBlur={() => { setHover(false); setPinned(false) }}
        onClick={() => setPinned((p) => !p)}>
        {issueLabel(issue)}
      </button>
      {open && at && (
        <div id={id} role="tooltip" className="bench-issue__card" data-tone={issueTone(issue)} style={{ left: at.left, top: at.top, width: CARD_W }}>
          <strong>{card.heading}</strong>
          {card.lines.map((l) => <p key={l}>{l}</p>)}
        </div>
      )}
    </>
  )
}
