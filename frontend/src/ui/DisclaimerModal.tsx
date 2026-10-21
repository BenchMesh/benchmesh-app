/**
 * DisclaimerModal - the start screen: a welcome (logo, thanks, the app is in development and
 * needs feedback, contact) above the safety information. Cannot be dismissed except by
 * accepting; optional "don't show again". Styled with kit tokens, so it follows every skin.
 */

import React, { useId, useState } from 'react'
import { BrandMark } from './kit/BrandMark'
import { PageLink, SUPPORT_EMAIL } from './about/links'
import './disclaimer.css'

interface DisclaimerModalProps {
  isOpen: boolean;
  onAccept: (dontShowAgain: boolean) => void;
}

const POINTS = [
  {
    title: 'Connect Instruments Before Starting',
    text: (
      <>
        Some instruments may not handle being connected while the software is actively communicating
        with them. <strong>Always connect all instruments to your computer and power them on before
        starting BenchMesh.</strong>
      </>
    ),
  },
  {
    title: 'Limited Testing Scope',
    text: (
      <>
        This software has been tested on a limited number of instrument models. While no damage or
        issues have been observed during testing, your specific instrument configuration may behave
        differently.
      </>
    ),
  },
  {
    title: 'No Warranty or Liability',
    text: (
      <>
        BenchMesh is provided "as-is" without any warranty. The creators and maintainers assume no
        responsibility for any damage, data loss, or issues that may occur from using this software.
      </>
    ),
  },
]

export function DisclaimerModal({ isOpen, onAccept }: DisclaimerModalProps) {
  const [dontShowAgain, setDontShowAgain] = useState(false)
  const uid = useId()

  if (!isOpen) return null

  return (
    <div className="bm-start">
      <div className="bm-start__card" role="dialog" aria-modal="true" aria-labelledby={`${uid}-welcome`}>
        <div className="bm-start__body">
          <section className="bm-start__welcome" aria-labelledby={`${uid}-welcome`}>
            <BrandMark size={72} className="bm-start__logo" />
            <span className="bm-start__name">BenchMesh</span>
            <h1 id={`${uid}-welcome`} className="bm-start__thanks">Thank you for trying BenchMesh</h1>
            <p className="bm-start__text">
              BenchMesh is still in development. It already drives the instruments on our bench, but making
              it work with all possible instruments needs your feedback: tell us what works, what does not
              and which instruments you use.
            </p>
            <p className="bm-start__contact">
              <span className="kit-label">Contact</span>
              <PageLink page="email" className="bm-start__email">{SUPPORT_EMAIL}</PageLink>
            </p>
          </section>

          <section className="bm-start__safety" aria-labelledby={`${uid}-safety`}>
            <h2 id={`${uid}-safety`} className="bm-start__heading">
              <svg className="bm-start__warn" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path d="M12 3 2 21h20L12 3Z" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
                <path d="M12 10v5M12 17.5v.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
              </svg>
              Important Safety Information
            </h2>
            <p className="bm-start__lead">Before using BenchMesh, please read and acknowledge the following:</p>
            <ol className="bm-start__points">
              {POINTS.map((p, i) => (
                <li key={p.title} className="bm-start__point">
                  <span className="bm-start__number" aria-hidden="true">{i + 1}</span>
                  <div>
                    <h3 className="bm-start__point-title">{p.title}</h3>
                    <p className="bm-start__point-text">{p.text}</p>
                  </div>
                </li>
              ))}
            </ol>
          </section>
        </div>

        <div className="bm-start__footer">
          <label className="bm-start__check">
            <input type="checkbox" checked={dontShowAgain} onChange={(e) => setDontShowAgain(e.target.checked)} />
            <span>Don't show this message again</span>
          </label>
          <button type="button" className="bench-btn bench-btn--primary bm-start__accept" onClick={() => onAccept(dontShowAgain)}>
            I Agree and Continue
          </button>
        </div>
      </div>
    </div>
  )
}
