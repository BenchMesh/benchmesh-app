import React, { useState } from 'react'
import { disclaimerHidden, setDisclaimerHidden } from '../disclaimer'

/** Instruments - Miscellaneous: show the start screen (welcome and safety information) again. */
export function StartScreenSetting() {
  const [show, setShow] = useState(() => !disclaimerHidden())
  const change = (next: boolean) => {
    setDisclaimerHidden(!next)
    setShow(next)
  }
  return (
    <div className="settings-view__section">
      <h3 className="settings-view__section-title">Start screen</h3>
      <div className="settings-view__setting">
        <label htmlFor="start-screen" className="settings-view__label">
          Show at start
          <span className="settings-view__label-hint">The welcome and the safety information, each time the app starts</span>
        </label>
        <input id="start-screen" type="checkbox" className="settings-view__check" checked={show}
          onChange={(e) => change(e.target.checked)} />
      </div>
    </div>
  )
}
