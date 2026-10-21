/** About and feedback drawer: the running version and where to send feedback and issue reports. */
import React, { useState } from 'react'
import { PageLink, PAGE_URLS } from './links'
import { useAppVersion } from './version'

export function AboutView() {
  const v = useAppVersion()
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    if (!v) return
    try {
      await navigator.clipboard.writeText(v.display)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch { /* the version stays selectable */ }
  }
  return (
    <div className="bench-page" data-testid="about-view">
      <section className="bench-group" aria-label="Version">
        <span className="kit-label">Version</span>
        <div className="bench-group__row">
          <p className="bench-about__version" data-testid="app-version">{v ? v.display : 'unknown (the server does not answer)'}</p>
          {v && <button type="button" className="bench-btn" onClick={() => void copy()}>{copied ? 'Copied' : 'Copy version'}</button>}
        </div>
        <p className="bench-hint">The desktop app looks for a newer release at every start and offers it. All releases: <PageLink page="releases">GitHub releases</PageLink>.</p>
      </section>
      <section className="bench-group" aria-label="Feedback">
        <span className="kit-label">Feedback and issue reports</span>
        <p className="bench-hint">For now, please send feedback, ideas and issue reports through GitHub issues (writing one needs a free GitHub account).</p>
        <div className="bench-group__row">
          <PageLink page="new-issue" className="bench-btn bench-btn--primary">Report an issue</PageLink>
          <PageLink page="issues" className="bench-btn">See open issues</PageLink>
        </div>
        <p className="bench-hint"><code>{PAGE_URLS.issues}</code></p>
        <p className="bench-hint">
          Please include the version above. For an instrument problem, open the panel's console (the console icon in the panel
          header), press Copy or Save and add the text to the issue: it carries the instrument, its model, its identify reply and
          what was sent and received.
        </p>
      </section>
    </div>
  )
}
