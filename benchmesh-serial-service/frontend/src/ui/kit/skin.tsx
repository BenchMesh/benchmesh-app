/**
 * Visual skin of the instrument kit. The skin is a `data-skin` attribute on the root element;
 * `kit.css` maps it to token values. Reflective LCD is the default (no attribute needed).
 * Canvas components read tokens at draw time, so they repaint when the skin changes
 * (`useCanvas` includes the skin in its redraw dependencies).
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'

export type Skin = 'lcd' | 'led' | 'glass'

export const SKINS: { value: Skin; label: string; title: string }[] = [
  { value: 'lcd', label: 'LCD', title: 'Reflective LCD: dark segments on grey-green glass' },
  { value: 'led', label: 'LED', title: 'LED segment: glowing digits on a dark rack' },
  { value: 'glass', label: 'Glass', title: 'Cockpit glass: calm dark panels, B612 numbers' },
]

export const SKIN_STORAGE_KEY = 'benchmesh:skin'
export const DEFAULT_SKIN: Skin = 'lcd'

const isSkin = (v: unknown): v is Skin => v === 'lcd' || v === 'led' || v === 'glass'

/** The persisted skin, or the default when nothing valid is stored or storage is unavailable. */
export function loadSkin(): Skin {
  try {
    const v = localStorage.getItem(SKIN_STORAGE_KEY)
    return isSkin(v) ? v : DEFAULT_SKIN
  } catch {
    return DEFAULT_SKIN
  }
}

export function saveSkin(skin: Skin) {
  try { localStorage.setItem(SKIN_STORAGE_KEY, skin) } catch { /* storage unavailable: skin lasts this session */ }
}

/** Sets the attribute the token blocks in `kit.css` key on. */
export function applySkin(skin: Skin) {
  document.documentElement.dataset.skin = skin
}

interface SkinState { skin: Skin; setSkin: (s: Skin) => void }

const SkinContext = createContext<SkinState>({ skin: DEFAULT_SKIN, setSkin: () => {} })

export function SkinProvider({ children }: { children: React.ReactNode }) {
  const [skin, setState] = useState<Skin>(() => {
    const s = loadSkin()
    applySkin(s) // before children paint their canvases
    return s
  })
  // The attribute must change before the children's layout effects repaint with the new
  // tokens, and children's effects run before the provider's: apply it in the setter.
  const setSkin = useCallback((s: Skin) => {
    applySkin(s)
    saveSkin(s)
    setState(s)
  }, [])
  useEffect(() => { applySkin(skin) }, [skin])
  const value = useMemo(() => ({ skin, setSkin }), [skin, setSkin])
  return <SkinContext.Provider value={value}>{children}</SkinContext.Provider>
}

export function useSkin(): SkinState {
  return useContext(SkinContext)
}
