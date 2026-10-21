/** About and feedback drawer: the running version and where to send feedback and issue reports. */
import React, { useState } from 'react'
import { PageLink, PAGE_URLS, SUPPORT_EMAIL } from './links'
import { useAppVersion } from './version'

export function AboutView() {
  const v = useAppVersion()
  const [copied, setCopied] = useState<'version' | 'email' | null>(null)
  const copy = async (what: 'version' | 'email', text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(what)
      setTimeout(() => setCopied(null), 2000)
    } catch { /* the text stays selectable */ }
  }
  return (
    <div className="bench-page" data-testid="about-view">
      <section className="bench-group" aria-label="Version">
        <span className="kit-label">Version</span>
        <div className="bench-group__row">
          <p className="bench-about__version" data-testid="app-version">{v ? v.display : 'unknown (the server does not answer)'}</p>
          {v && <button type="button" className="bench-btn" onClick={() => void copy('version', v.display)}>{copied === 'version' ? 'Copied' : 'Copy version'}</button>}
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
      <section className="bench-group" aria-label="Contact">
        <span className="kit-label">Contact</span>
        <div className="bench-group__row">
          <PageLink page="email" className="bench-about__email">{SUPPORT_EMAIL}</PageLink>
          <button type="button" className="bench-btn" onClick={() => void copy('email', SUPPORT_EMAIL)}>{copied === 'email' ? 'Copied' : 'Copy email'}</button>
        </div>
        <p className="bench-hint">Questions and anything that does not fit an issue report. Please include the version above.</p>
      </section>
    </div>
  )
}
