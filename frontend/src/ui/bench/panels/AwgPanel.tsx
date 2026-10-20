import React, { useEffect, useState } from 'react'
import { ArmSwitch, DigitEditor, Selector, WavePreview, WaveGlyph, siNumber, siText, intDigitsFor } from '../../kit'
import { AwgProfile, NumberSpec } from '../profiles'
import { ChannelApi, useAction } from '../api'
import { Recordable } from '../../recording/signal'
import {
  PERIOD_UNITS, dbmOf, fromHighLow, highLow, loadOhms, periodUnitFor, reciprocal, roundTo, vppFromDbm,
  vppFromVrms, vrmsOf,
} from '../awgMath'

const ENTRY_KEY = 'benchmesh:awg-entry'
type Entry = { freq: 'FREQ' | 'PERIOD'; ampl: 'VPP' | 'VRMS' | 'DBM' | 'HILO' }

function loadEntry(): Entry {
  try {
    const e = JSON.parse(localStorage.getItem(ENTRY_KEY) ?? '{}')
    return { freq: e.freq === 'PERIOD' ? 'PERIOD' : 'FREQ', ampl: ['VRMS', 'DBM', 'HILO'].includes(e.ampl) ? e.ampl : 'VPP' }
  } catch { return { freq: 'FREQ', ampl: 'VPP' } }
}

const ENTRY_EVENT = 'benchmesh:awg-entry'

/** How frequency and amplitude are entered (the same on every generator and channel), kept per browser. */
function useEntry(): [Entry, (e: Partial<Entry>) => void] {
  const [entry, set] = useState(loadEntry)
  // every channel follows a change made in another one
  useEffect(() => {
    const follow = () => set(loadEntry())
    window.addEventListener(ENTRY_EVENT, follow)
    return () => window.removeEventListener(ENTRY_EVENT, follow)
  }, [])
  return [entry, (e) => {
    const next = { ...entry, ...e }
    try { localStorage.setItem(ENTRY_KEY, JSON.stringify(next)) } catch { /* private window */ }
    set(next)
    window.dispatchEvent(new Event(ENTRY_EVENT))
  }]
}

const CALC_TITLE = 'Calculated by BenchMesh from what the instrument reports'

/** Editor spec for a derived quantity: `decimals` fixed, integer digits from the largest value. */
function derivedSpec(min: number, max: number, decimals: number, unit: string): NumberSpec {
  const res = 10 ** -decimals
  return { min: min > 0 ? Math.max(min, res) : min, max, intDigits: intDigitsFor(Math.max(Math.abs(min), Math.abs(max))), decimals, unit }
}

/** Frequency in Hz, or the period in a chosen unit (sent as 1 / period). */
function FrequencyField({ freq, profile, api, online, entry, setEntry }: {
  freq: number | null; profile: AwgProfile; api: ChannelApi; online: boolean; entry: Entry; setEntry: (e: Partial<Entry>) => void
}) {
  const period = reciprocal(freq)
  const lim = profile.frequencyLimits
  if (!profile.frequency) return null
  const toggle = lim && (
    <Selector label="Frequency or period" value={entry.freq} onChange={(v) => setEntry({ freq: v as Entry['freq'] })}
      options={[{ value: 'FREQ', label: 'Hz', title: 'Enter the frequency' }, { value: 'PERIOD', label: 'T', title: 'Enter the period' }]} />
  )
  if (entry.freq !== 'PERIOD' || !lim) {
    return (
      <div className="bench-awg__field">
        <Recordable field="FREQ" unit="Hz" focusable={false}>
          <DigitEditor label="Frequency" value={freq} spec={profile.frequency} unit="Hz" group={3} disabled={!online}
            onCommit={(v) => api.set('frequency', v)} />
        </Recordable>
        {toggle}
      </div>
    )
  }
  return (
    <div className="bench-awg__field">
      <TimeEditor label="Period" seconds={period} min={1 / lim.max} max={1 / lim.min} disabled={!online}
        onCommit={(t) => api.set('frequency', Number((1 / t).toPrecision(9)))} />
      {toggle}
    </div>
  )
}

/**
 * A time (period, pulse width, edges, burst period) in a unit picked from its range: ns to s,
 * 3 + 3 digits; the unit follows the value unless one is chosen.
 */
export function TimeEditor({ label, seconds, min, max, disabled, onCommit }: {
  label: string; seconds: number | null; min: number; max: number; disabled?: boolean; onCommit: (s: number) => Promise<void> | void
}) {
  const [unitPick, setUnitPick] = useState<string | null>(null)
  const tMax = Math.min(max, 1e6)
  // units in which the range has something to enter
  const units = PERIOD_UNITS.filter((u) => tMax >= u.scale && min < u.scale * 1000)
  const u = units.find((x) => x.unit === unitPick) ?? units.find((x) => x.unit === periodUnitFor(seconds).unit) ?? units[0] ?? PERIOD_UNITS[0]
  const spec = derivedSpec(min / u.scale, Math.min(tMax / u.scale, 999.999), 3, u.unit)
  return (
    <div className="bench-awg__field">
      <DigitEditor label={label} value={roundTo(seconds == null ? null : seconds / u.scale, spec.decimals)} spec={spec} unit={u.unit}
        disabled={disabled} onCommit={(v) => onCommit(Number((v * u.scale).toPrecision(9)))} />
      {units.length > 1 && (
        <Selector variant="chips" label={`${label} unit`} value={u.unit} onChange={(v) => setUnitPick(v)}
          options={units.map((x) => ({ value: x.unit, label: x.unit }))} />
      )}
    </div>
  )
}

/** Amplitude as Vpp (native), Vrms or dBm (for waveforms with a fixed crest factor), or high / low levels. */
function AmplitudeField({ ampl, offs, shape, ohms, profile, api, online, entry, setEntry }: {
  ampl: number | null; offs: number | null; shape: string; ohms: number | null; profile: AwgProfile; api: ChannelApi
  online: boolean; entry: Entry; setEntry: (e: Partial<Entry>) => void
}) {
  const a = profile.amplitude
  if (!a) return null
  const rmsOk = vrmsOf(1, shape) != null
  const dbmOk = rmsOk && ohms != null && Number.isFinite(ohms)
  const hiloOk = !!profile.offset && profile.levelLimit != null
  const options = [
    { value: 'VPP', label: 'Vpp', title: 'Peak-to-peak amplitude' },
    { value: 'VRMS', label: 'Vrms', disabled: !rmsOk, title: rmsOk ? 'RMS amplitude' : `No fixed RMS ratio for ${shape || 'this waveform'}` },
    { value: 'DBM', label: 'dBm', disabled: !dbmOk, title: dbmOk ? `Power into the ${ohms} Ω load` : 'Needs a sine, square or ramp and a load in ohms (not High-Z)' },
    { value: 'HILO', label: 'H/L', disabled: !hiloOk, title: 'High and low level' },
  ]
  const view = options.find((o) => o.value === entry.ampl && !o.disabled)?.value ?? 'VPP'
  const toggle = (
    <Selector label="Amplitude unit" value={view} onChange={(v) => setEntry({ ampl: v as Entry['ampl'] })} options={options} />
  )
  const setAmpl = async (vpp: number | null) => {
    if (vpp == null || vpp < a.min - 1e-12 || vpp > a.max + 1e-12) throw new Error(`Outside ${a.min} to ${a.max} Vpp`)
    await api.set('amplitude', Number(vpp.toPrecision(6)))
  }

  let editors: React.ReactNode
  if (view === 'VRMS') {
    const k = vppFromVrms(1, shape)!
    const spec = derivedSpec(a.min / k, a.max / k, 4, 'Vrms')
    editors = <DigitEditor label="Amplitude" value={roundTo(vrmsOf(ampl, shape), spec.decimals)} spec={spec} unit="Vrms"
      disabled={!online} onCommit={(v) => setAmpl(vppFromVrms(v, shape))} />
  } else if (view === 'DBM') {
    const spec = derivedSpec(Math.ceil(dbmOf(a.min, shape, ohms)! * 100) / 100, Math.floor(dbmOf(a.max, shape, ohms)! * 100) / 100, 2, 'dBm')
    editors = <DigitEditor label="Amplitude" value={roundTo(dbmOf(ampl, shape, ohms), spec.decimals)} spec={spec} unit="dBm"
      disabled={!online} onCommit={(v) => setAmpl(vppFromDbm(v, shape, ohms))} />
  } else if (view === 'HILO') {
    const lim = profile.levelLimit!
    const spec = derivedSpec(-lim, lim, profile.offset!.decimals, 'V')
    const hl = highLow(ampl, offs)
    const apply = async (high: number, low: number) => {
      if (high <= low) throw new Error('High must be above low')
      const t = fromHighLow(high, low)
      // shrink first, so the pair never passes through a combination beyond the limits
      const amplFirst = ampl != null && t.amplitude < ampl
      if (amplFirst) await setAmpl(t.amplitude)
      await api.set('offset', Number(t.offset.toPrecision(6)))
      if (!amplFirst) await setAmpl(t.amplitude)
    }
    editors = (
      <>
        <DigitEditor label="High level" value={roundTo(hl?.high ?? null, spec.decimals)} spec={spec} unit="V" disabled={!online || !hl}
          onCommit={(v) => apply(v, hl!.low)} />
        <DigitEditor label="Low level" value={roundTo(hl?.low ?? null, spec.decimals)} spec={spec} unit="V" disabled={!online || !hl}
          onCommit={(v) => apply(hl!.high, v)} />
      </>
    )
  } else {
    editors = (
      <Recordable field="AMPL" unit={a.unit} focusable={false}>
        <DigitEditor label="Amplitude" value={ampl} spec={a} unit={a.unit} disabled={!online} onCommit={(v) => api.set('amplitude', v)} />
      </Recordable>
    )
  }
  return <div className="bench-awg__field">{editors}{toggle}</div>
}

/** What the output does besides the carrier, for the preview. */
function modeCaption(mode: string | null, st: Record<string, any>): string | undefined {
  if (mode === 'MOD' && typeof st.MOD_TYPE === 'string') {
    const f = siNumber(st.MOD_FREQ)
    return `${st.MOD_TYPE}${f != null ? ` ${siText(f, 'Hz', 4)}` : ''}`
  }
  if (mode === 'SWEEP') {
    const a = siNumber(st.SWEEP_START), b = siNumber(st.SWEEP_STOP)
    return `SWEEP${a != null && b != null ? ` ${siText(a, 'Hz', 4)} → ${siText(b, 'Hz', 4)}` : ''}`
  }
  if (mode === 'BURST') {
    return `BURST${st.BURST_MODE === 'INF' ? ' ∞' : typeof st.BURST_CYCLES === 'number' ? ` ${st.BURST_CYCLES} cyc` : ''}`
  }
  return undefined
}

const MODE_TITLES: Record<string, string> = {
  CW: 'Continuous output', MOD: 'Modulation', SWEEP: 'Frequency sweep', BURST: 'Burst',
}
const TRIG_LABELS: Record<string, string> = { INT: 'Internal', MAN: 'Manual', EXT: 'External' }
const TRIG_TITLES: Record<string, string> = {
  INT: 'Repeats on its own (sweep time / burst period)',
  MAN: 'Starts on the trigger key of the instrument',
  EXT: 'Starts on the trigger input',
}

/** Settings of the active mode: modulation, sweep or burst (the carrier stays above). */
function ModeSettings({ mode, status, profile, api, online, run }: {
  mode: string; status: Record<string, any>; profile: AwgProfile; api: ChannelApi; online: boolean
  run: (p: Promise<unknown>) => Promise<boolean>
}) {
  const choose = (op: string, values: string[], value: unknown, label: string, labels?: Record<string, React.ReactNode>, titles?: Record<string, string>) =>
    values.length > 0 && (
      <div className="kit-field">
        <span className="kit-label">{label}</span>
        <Selector label={label} disabled={!online} value={typeof value === 'string' ? value : null}
          options={values.map((v) => ({ value: v, label: labels?.[v] ?? v, title: titles?.[v] }))} onChange={(v) => run(api.set(op, v))} />
      </div>
    )
  const num = (label: string, op: string, key: string, spec: NumberSpec | null, group = 0) => spec && (
    <DigitEditor label={label} value={siNumber(status[key])} spec={spec} unit={spec.unit} group={group} disabled={!online}
      onCommit={(v) => api.set(op, v)} />
  )
  if (mode === 'MOD') {
    const type = typeof status.MOD_TYPE === 'string' ? status.MOD_TYPE : null
    return (
      <div className="bench-awg__mode kit-row" role="group" aria-label="Modulation settings">
        {choose('mod_type', profile.modTypes, type, 'Modulation')}
        {profile.modShapes.length > 0 && (
          <div className="kit-field">
            <span className="kit-label">Modulating wave</span>
            <Selector variant="chips" label="Modulating wave" disabled={!online} value={typeof status.MOD_SHAPE === 'string' ? status.MOD_SHAPE : null}
              options={profile.modShapes.map((v) => ({ value: v, label: <WaveGlyph shape={v} />, title: v }))} onChange={(v) => run(api.set('mod_shape', v))} />
          </div>
        )}
        {num('Modulating frequency', 'mod_frequency', 'MOD_FREQ', profile.modFrequency, 3)}
        {type === 'AM' && num('Depth', 'am_depth', 'AM_DEPTH', profile.amDepth)}
        {type === 'FM' && num('Deviation', 'fm_deviation', 'FM_DEV', profile.fmDeviation, 3)}
        {type === 'PM' && num('Deviation', 'pm_deviation', 'PM_DEV', profile.pmDeviation)}
      </div>
    )
  }
  if (mode === 'SWEEP') {
    return (
      <div className="bench-awg__mode kit-row" role="group" aria-label="Sweep settings">
        {num('Start', 'sweep_start', 'SWEEP_START', profile.sweepStart, 3)}
        {num('Stop', 'sweep_stop', 'SWEEP_STOP', profile.sweepStop, 3)}
        {profile.sweepTime && (
          <TimeEditor label="Sweep time" seconds={siNumber(status.SWEEP_TIME)} min={profile.sweepTime.min} max={profile.sweepTime.max}
            disabled={!online} onCommit={(t) => api.set('sweep_time', t)} />
        )}
        {choose('sweep_spacing', profile.sweepSpacings, status.SWEEP_SPACING, 'Spacing', { LIN: 'Linear', LOG: 'Log' })}
        {choose('sweep_trigger', profile.sweepTriggers, status.SWEEP_TRIG, 'Sweep trigger', TRIG_LABELS, TRIG_TITLES)}
        {profile.trigger && status.SWEEP_TRIG !== 'INT' && <TriggerButton api={api} online={online} run={run} />}
      </div>
    )
  }
  if (mode === 'BURST') {
    const bmode = typeof status.BURST_MODE === 'string' ? status.BURST_MODE : null
    return (
      <div className="bench-awg__mode kit-row" role="group" aria-label="Burst settings">
        {choose('burst_mode', profile.burstModes, bmode, 'Burst', { NCYC: 'N cycles', INF: 'Infinite', GATED: 'Gated' })}
        {bmode !== 'INF' && bmode !== 'GATED' && num('Cycles', 'burst_cycles', 'BURST_CYCLES', profile.burstCycles, 3)}
        {profile.burstPeriod && status.BURST_TRIG !== 'MAN' && status.BURST_TRIG !== 'EXT' && (
          <TimeEditor label="Burst period" seconds={siNumber(status.BURST_PERIOD)} min={profile.burstPeriod.min} max={profile.burstPeriod.max}
            disabled={!online} onCommit={(t) => api.set('burst_period', t)} />
        )}
        {choose('burst_trigger', profile.burstTriggers, status.BURST_TRIG, 'Burst trigger', TRIG_LABELS, TRIG_TITLES)}
        {profile.trigger && status.BURST_TRIG !== 'INT' && <TriggerButton api={api} online={online} run={run} />}
      </div>
    )
  }
  return null
}

function TriggerButton({ api, online, run }: { api: ChannelApi; online: boolean; run: (p: Promise<unknown>) => Promise<boolean> }) {
  return (
    <button type="button" className="bench-btn" disabled={!online} title="Fire one sweep or burst" onClick={() => run(api.set('trigger', 'ONCE'))}>
      Trigger
    </button>
  )
}

/** One generator channel: waveform, output, preview, frequency / amplitude / offset, phase, symmetry. */
export function AwgChannel({ ch, status, profile, api, online }: {
  ch: string; status: Record<string, any>; profile: AwgProfile; api: ChannelApi; online: boolean
}) {
  const [error, run] = useAction()
  const [entry, setEntry] = useEntry()
  const shape = typeof status.SHAPE === 'string' ? status.SHAPE.toUpperCase() : ''
  const freq = siNumber(status.FREQ)
  const ampl = siNumber(status.AMPL)
  const offs = siNumber(status.OFFS)
  const phase = siNumber(status.PHASE)
  const symm = siNumber(status.SYMM)
  const ohms = loadOhms(status.LOAD)
  const duty = siNumber(status.DUTY)
  const mode = typeof status.MODE === 'string' ? status.MODE : profile.modes.length ? null : 'CW'
  const on = status.OUTPUT === 'ON'
  const color = profile.channelColors[ch]

  // what the editors do not show, derived (labelled as calculated)
  const hl = highLow(ampl, offs)
  const derived = [
    entry.freq === 'PERIOD' ? (freq != null ? `f ${siText(freq, 'Hz', 6)}` : null) : (reciprocal(freq) != null ? `T ${siText(reciprocal(freq), 's', 6)}` : null),
    entry.ampl !== 'VPP' && ampl != null ? `${siText(ampl, 'Vpp', 4)}` : null,
    entry.ampl !== 'VRMS' && vrmsOf(ampl, shape) != null ? siText(vrmsOf(ampl, shape), 'Vrms', 4) : null,
    entry.ampl !== 'DBM' && dbmOf(ampl, shape, ohms) != null ? `${dbmOf(ampl, shape, ohms)!.toFixed(2)} dBm` : null,
    entry.ampl !== 'HILO' && hl ? `H ${hl.high.toFixed(3)} V  L ${hl.low.toFixed(3)} V` : null,
  ].filter(Boolean)

  return (
    <div className="bench-awg__chan">
      <div className="bench-awg__head">
        <span className="bench-awg__ch">
          {color && <i style={{ background: color }} aria-hidden="true" />}CH{ch}
        </span>
        {profile.shapes.length > 0 && (
          <Selector variant="chips" label={`Waveform CH${ch}`} disabled={!online} value={shape}
            options={profile.shapes.map((s) => ({ value: s, label: <WaveGlyph shape={s} />, title: s }))}
            onChange={(s) => run(api.set('shape', s))} />
        )}
        {profile.loads.length > 1 ? (
          <Selector variant="chips" label={`Load CH${ch}`} disabled={!online} value={typeof status.LOAD === 'string' ? status.LOAD : null}
            options={profile.loads.map((l) => ({ value: l, label: l === 'HIGHZ' ? 'HIGH-Z' : `${l} Ω`, title: 'Load the amplitude refers to' }))}
            onChange={(v) => run(api.set('load', v))} />
        ) : status.LOAD != null && (
          <span className="bench-awg__load" title="Load impedance set on the instrument">
            {ohms === Infinity ? 'HIGH-Z' : ohms != null ? `${ohms} Ω` : String(status.LOAD)}
          </span>
        )}
        {profile.output && <ArmSwitch label={`CH${ch}`} on={on} disabled={!online} onChange={(v) => api.set('output', v ? 'ON' : 'OFF')} />}
      </div>
      {profile.modes.length > 1 && (
        <Selector label={`Mode CH${ch}`} disabled={!online} value={mode}
          options={profile.modes.map((m) => ({ value: m, label: m, title: MODE_TITLES[m] }))}
          onChange={(m) => run(api.set('mode', m))} />
      )}
      <div className="kit-glass">
        <WavePreview shape={shape} amplitude={ampl ?? 0} offset={offs ?? 0} on={on} symmetry={symm ?? 100}
          duty={shape === 'PUL' ? duty ?? undefined : undefined} caption={modeCaption(mode, status)}
          maxAmplitude={profile.amplitude?.max ?? 20} maxOffset={profile.offset?.max ?? 10} />
      </div>
      {mode && mode !== 'CW' && <ModeSettings mode={mode} status={status} profile={profile} api={api} online={online} run={run} />}
      <div className="kit-row">
        <FrequencyField freq={freq} profile={profile} api={api} online={online} entry={entry} setEntry={setEntry} />
        <AmplitudeField ampl={ampl} offs={offs} shape={shape} ohms={ohms} profile={profile} api={api} online={online}
          entry={entry} setEntry={setEntry} />
        {profile.offset && entry.ampl !== 'HILO' && (
          <Recordable field="OFFS" unit={profile.offset.unit} focusable={false}>
            <DigitEditor label="Offset" value={offs} spec={profile.offset} unit={profile.offset.unit} disabled={!online}
              onCommit={(v) => api.set('offset', v)} />
          </Recordable>
        )}
        {profile.phase && (
          <DigitEditor label="Phase" value={phase} spec={profile.phase} unit="°" disabled={!online}
            onCommit={(v) => api.set('phase', v)} />
        )}
        {profile.symmetry && shape === 'RAMP' && (
          <DigitEditor label="Symmetry" value={symm} spec={profile.symmetry} unit="%" disabled={!online}
            onCommit={(v) => api.set('symmetry', v)} />
        )}
      </div>
      {shape === 'PUL' && (profile.duty || profile.pulseWidth || profile.pulseRise || profile.pulseFall) && (
        <div className="kit-row" role="group" aria-label="Pulse">
          {profile.duty && (
            <DigitEditor label="Duty" value={duty} spec={profile.duty} unit="%" disabled={!online} onCommit={(v) => api.set('duty', v)} />
          )}
          {profile.pulseWidth && (
            <TimeEditor label="Width" seconds={siNumber(status.WIDTH)} min={profile.pulseWidth.min} max={profile.pulseWidth.max}
              disabled={!online} onCommit={(t) => api.set('pulse_width', t)} />
          )}
          {profile.pulseRise && (
            <TimeEditor label="Rise" seconds={siNumber(status.RISE)} min={profile.pulseRise.min} max={profile.pulseRise.max}
              disabled={!online} onCommit={(t) => api.set('pulse_rise', t)} />
          )}
          {profile.pulseFall && (
            <TimeEditor label="Fall" seconds={siNumber(status.FALL)} min={profile.pulseFall.min} max={profile.pulseFall.max}
              disabled={!online} onCommit={(t) => api.set('pulse_fall', t)} />
          )}
        </div>
      )}
      {online && profile.amplitude && ampl == null && shape !== '' && (
        <span className="bench-awg__note">The amplitude is not reported in Vpp: the instrument may be set to show Vrms or dBm on its front panel.</span>
      )}
      {derived.length > 0 && (
        <div className="bench-awg__derived" title={CALC_TITLE}>
          <span className="bench-calc">calc</span>{derived.join('  ·  ')}
        </div>
      )}
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
