/**
 * A rectangular indicator LED, as tall as the capital letters of the text it stands before (its
 * bottom on their baseline: put it in a baseline-aligned row, or inline in the text) and twice as
 * long; lit green / amber / red, or dark. The tooltip says what it shows.
 */
export type LedState = 'ok' | 'warn' | 'alert' | 'off'

export function Led({ state, label, title, className }: { state: LedState; label: string; title: string; className?: string }) {
  return <i className={`bench-led${className ? ` ${className}` : ''}`} data-state={state} role="img" aria-label={label} title={title} />
}
