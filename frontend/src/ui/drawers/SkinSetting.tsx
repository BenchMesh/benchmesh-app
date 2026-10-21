import React from 'react'
import { Selector, useSkin, SKINS, type Skin } from '../kit'

/** Instruments - Miscellaneous: the kit skin (persisted per browser, shared by every window). */
export function SkinSetting() {
  const { skin, setSkin } = useSkin()
  return (
    <div className="settings-view__section">
      <h3 className="settings-view__section-title">Appearance</h3>
      <div className="settings-view__setting">
        <span className="settings-view__label">
          Skin
          <span className="settings-view__label-hint">How the instrument panels look</span>
        </span>
        <Selector label="Skin" value={skin} onChange={(v) => setSkin(v as Skin)}
          options={SKINS.map((s) => ({ value: s.value, label: s.label, title: s.title }))} />
      </div>
    </div>
  )
}
