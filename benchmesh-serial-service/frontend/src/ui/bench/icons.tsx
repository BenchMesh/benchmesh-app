/** Small line icons for the panel window controls (stroke = text colour). */
import React from 'react'

const Svg = ({ children }: { children: React.ReactNode }) => (
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    {children}
  </svg>
)

/** Fold to the header line. */
export const FoldIcon = () => <Svg><path d="M3 10l5-5 5 5" /></Svg>
/** Unfold. */
export const UnfoldIcon = () => <Svg><path d="M3 6l5 5 5-5" /></Svg>
/** Open in its own window. */
export const DetachIcon = () => <Svg><path d="M9 3h4v4M13 3L7.5 8.5M11 9.5V13H3V5h3.5" /></Svg>
/** Keep on top (pin). */
export const PinIcon = () => <Svg><path d="M6 2.5h4M7 2.5v4L4.5 9h7L9 6.5v-4M8 9v4.5" /></Svg>
/** Instrument console (wire log). */
export const ConsoleIcon = () => <Svg><path d="M2.5 3.5h11v9h-11zM5 7l2 1.5L5 10M8.5 10.5H11" /></Svg>
/** Close the window (back to the bench). */
export const CloseIcon = () => <Svg><path d="M4 4l8 8M12 4l-8 8" /></Svg>
