/**
 * BenchMesh instrument kit: the visual building blocks every instrument panel is made of
 * (skins in `kit.css`: Reflective LCD by default, LED segment, Cockpit glass; see `skin.tsx`).
 * Rules of use:
 * - Readout: measured values only (fixed digit count, ghost segments).
 * - DigitEditor: setpoints; nothing is sent until Enter.
 * - ArmSwitch: anything that energises (operations with `energizes`): two presses to switch on.
 * - MicroSwitch: secondary, non-energising settings.
 * - Annunciators: state flags, always present, lit when active.
 * - RotaryDial / Selector: mutually exclusive functions, modes and ranges.
 * - Bargraph / Sparkline: fraction of range, recent history.
 */
import './kit.css'

export { Readout, Annunciators, MicroSwitch, Selector } from './controls'
export type { Annunciator, Choice } from './controls'
export { DigitEditor } from './DigitEditor'
export type { DigitEditorHandle } from './DigitEditor'
export { ArmSwitch } from './ArmSwitch'
export { RotaryDial } from './RotaryDial'
export type { DialOption } from './RotaryDial'
export { Knob } from './Knob'
export { Bargraph } from './Bargraph'
export { Sparkline } from './Sparkline'
export { SegmentDisplay } from './SegmentDisplay'
export type { Tone } from './useCanvas'
export { SkinProvider, useSkin, loadSkin, SKINS, SKIN_STORAGE_KEY, DEFAULT_SKIN } from './skin'
export type { Skin } from './skin'
export { WavePreview, WaveGlyph, WAVE_SHAPES } from './WavePreview'
export * from './format'
export { ScopeScreen } from './ScopeScreen'
export type { ScopeTrace } from './ScopeScreen'
export { Stepper } from './Stepper'
export { step125, screenDivs, tracePoints, siText } from './scope'
export { digitModel } from './digitModel'
export type { DigitSpec } from './digitModel'
