import React, { useEffect, useRef, useState } from 'react'
import { Readout, Annunciators, DigitEditor, DigitEditorHandle, ArmSwitch, MicroSwitch, Selector, Knob, fixedText, siNumber, intDigitsFor } from '../../kit'
import { EllProfile } from '../profiles'
import { ChannelApi, useAction, useSetpoint } from '../api'
import { Recordable } from '../../recording/signal'

const parseNum = (v: unknown) => siNumber(v)

const onOff = (v: unknown): boolean | null => (v === 'ON' || v === true ? true : v === 'OFF' || v === false ? false : null)

/**
 * The short-circuit state: the `SHORT` status where the definition reports it, else read when
 * the load comes online and after every switch.
 */
function useShort(api: ChannelApi, enabled: boolean, reported: unknown) {
  const [read, setRead] = useState<boolean | null>(null)
  useEffect(() => {
    if (!enabled || reported !== undefined) return
    let cancelled = false
    api.get('short').then((v) => { if (!cancelled) setRead(onOff(v)) }).catch(() => {})
    return () => { cancelled = true }
  }, [api, enabled, reported])
  const set = async (on: boolean) => {
    await api.set('short', on ? 'ON' : 'OFF')
    if (reported === undefined) api.get('short').then((v) => setRead(onOff(v))).catch(() => {})
  }
  return [reported !== undefined ? onOff(reported) : read, set] as const
}

/** Electronic load: V/I/P on the glass, mode selector, setpoint (knob + digits), input, short. */
export function EllPanel({ status, profile, api, online }: {
  status: Record<string, any>; profile: EllProfile; api: ChannelApi; online: boolean
}) {
  const mode = typeof status.MODE === 'string' ? status.MODE : profile.modes[0]?.key
  const m = profile.modes.find((x) => x.key === mode)
  const [setpoint, commit] = useSetpoint(api, m?.op ?? null, parseNum, online)
  const [short, setShort] = useShort(api, profile.short && online, status.SHORT)
  const [error, run] = useAction()
  const [sense, setSense] = useState(false)
  const editor = useRef<DigitEditorHandle>(null)
  const [knob, setKnob] = useState<number | null>(null)

  const remote = status.REMOTE === 'ON'
  const inputOn = status.INPUT === 'ON'
  const locked = profile.remoteRequired && !remote
  const canControl = online && !locked
  const vin = siNumber(status.VIN)
  const iin = siNumber(status.IIN)
  const pin = siNumber(status.PIN)
  const vMax = profile.modes.find((x) => x.key === 'VOLT')?.spec?.max ?? 150
  const iMax = profile.modes.find((x) => x.key === 'CURR')?.spec?.max ?? 15
  const pMax = profile.modes.find((x) => x.key === 'POW')?.spec?.max ?? 150

  return (
    <div className="bench-ell">
      <div className="kit-glass">
        <div className="bench-ell__trio">
          <Recordable field="VIN" unit="V"><Readout size="small" tag="V" text={fixedText(vin, intDigitsFor(vMax), 2)} unit="V" tone="volt" label="Input voltage" /></Recordable>
          <Recordable field="IIN" unit="A"><Readout size="small" tag="I" text={fixedText(iin, intDigitsFor(iMax), 3)} unit="A" tone="amp" label="Input current" /></Recordable>
          <Recordable field="PIN" unit="W"><Readout size="small" tag="P" text={fixedText(pin, intDigitsFor(pMax), 2)} unit="W" tone="watt" label="Input power" /></Recordable>
        </div>
        <Annunciators items={[
          { label: 'REM', on: remote },
          { label: 'INPUT', on: inputOn },
          { label: m?.short ?? String(mode ?? '-'), on: true, title: m?.name },
          { label: 'OVP', on: status.OVP === true, alert: true },
          { label: 'OCP', on: status.OCP === true, alert: true },
          { label: 'OPP', on: status.OPP === true, alert: true },
          ...(profile.short ? [{ label: 'SHORT', on: short === true, alert: true }] : []),
        ]} />
      </div>
      {locked && <span className="bench-hint">Switch Remote on to control the load from here.</span>}
      <Selector label="Load mode" disabled={!canControl} value={mode ?? null}
        options={profile.modes.map((x) => ({ value: x.key, label: x.short, title: x.name }))}
        onChange={(k) => run(api.set('mode', k))} />
      {m && !m.spec && (
        <span className="bench-hint">{m.name}: its parameters are set on the instrument's front panel.</span>
      )}
      {m?.spec && (
        <div className="bench-ell__set">
          <Knob label={`${m.short} setpoint`} min={m.spec.min} max={m.spec.max} disabled={!canControl}
            value={knob ?? setpoint ?? m.spec.min}
            onChange={(v) => { setKnob(v); editor.current?.setDraft(v) }} />
          <DigitEditor key={m.key} ref={editor} label={`${m.short} setpoint`} value={setpoint} spec={m.spec} unit={m.spec.unit}
            disabled={!canControl} onCommit={async (v) => { await commit(v); setKnob(null) }} />
        </div>
      )}
      <div className="kit-row">
        {profile.input && <ArmSwitch label="Input" on={inputOn} disabled={!canControl} onChange={(on) => api.set('input', on ? 'ON' : 'OFF')} />}
        {profile.short && <ArmSwitch label="Short" on={short === true} disabled={!canControl} onChange={setShort} />}
        {profile.remote && <MicroSwitch label="Remote" checked={remote} disabled={!online} onChange={(on) => api.set('remote', on ? 'ON' : 'OFF')} />}
        {profile.sense && (
          <MicroSwitch label="Sense" checked={sense} disabled={!canControl} title="Remote voltage sense (the load does not report this setting)"
            onChange={async (on) => { await api.set('remote_compensation', on ? 'ON' : 'OFF'); setSense(on) }} />
        )}
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
