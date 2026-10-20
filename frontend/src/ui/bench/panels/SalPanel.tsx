import React, { useMemo, useState } from 'react'
import {
  Annunciators, Selector, MicroSwitch, ArmSwitch, DigitEditor, Stepper, ActionButton, SpectrumScreen,
  siNumber, siText, step125, freqFraction, freqText, dbText, maxHold, peakIndex,
} from '../../kit'
import { NOT_CONFIRMED, nearly, useConfirmation } from '../../kit/confirm'
import { SalProfile } from '../profiles'
import { ChannelApi, useAction } from '../api'
import { Recordable } from '../../recording/signal'
import './sal.css'

const SWEEP_LABEL: Record<string, string> = { CONT: 'Cont', SINGLE: 'Single', HOLD: 'Hold', RESTART: 'Restart' }
const TRACE_LABEL: Record<string, string> = { WRITE: 'Clear/write', MAXHOLD: 'Max hold', MINHOLD: 'Min hold', AVERAGE: 'Average', VIEW: 'View' }
const PEAK_LABEL: Record<string, string> = { PEAK: 'Peak', NEXT: 'Next peak', LEFT: 'Peak ◀', RIGHT: 'Peak ▶', MKR_CF: 'Mkr→CF' }

/** Next value of a sorted list in direction `dir` from `value` (nearest step when off-list). */
export function stepList(list: number[], value: number | null, dir: number): number | null {
  if (!list.length) return null
  if (value == null) return list[0]
  const eps = Math.abs(value) * 1e-6
  return dir > 0 ? list.find((v) => v > value + eps) ?? list[list.length - 1] : [...list].reverse().find((v) => v < value - eps) ?? list[0]
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

/** Client-side max hold of the trace: one update per status message, restarted by `key`. */
function useMaxHold(trace: number[] | null, on: boolean, key: unknown): number[] | null {
  const [s, setS] = useState<{ trace: number[] | null; key: unknown; held: number[] | null }>({ trace, key, held: null })
  if (s.trace !== trace || s.key !== key) {
    const held = on && trace ? maxHold(s.key !== key ? null : s.held, trace) : null
    setS({ trace, key, held })
    return held
  }
  return on ? s.held : null
}

type Group = 'FREQ' | 'BW' | 'AMPT' | 'SWEEP' | 'TRACE' | 'MKR' | 'TG'

/**
 * Spectrum analyzer: screen (dB graticule, trace, max-hold ghost, marker 1) with the marker
 * readout and the annotation bar, and the soft-key groups FREQ / BW / AMPT / SWEEP / TRACE /
 * MKR / TG (only the controls the definition offers).
 */
export function SalPanel({ status, profile, api, online }: {
  status: Record<string, any>; profile: SalProfile; api: ChannelApi; online: boolean
}) {
  const [error, run] = useAction()
  const [group, setGroup] = useState<Group>('FREQ')
  const [ghostOn, setGhostOn] = useState(false)
  const center = siNumber(status.CENTER)
  const span = siNumber(status.SPAN)
  const start = siNumber(status.START)
  const stop = siNumber(status.STOP)
  const rbw = siNumber(status.RBW)
  const vbw = siNumber(status.VBW)
  const ref = siNumber(status.REF_LEVEL)
  const scale = siNumber(status.SCALE) ?? 10
  const atten = siNumber(status.ATTEN)
  const swt = siNumber(status.SWEEP_TIME)
  const unit = typeof status.TRACE_UNIT === 'string' ? status.TRACE_UNIT : 'dBm'
  const rawTrace = status.TRACE
  const trace = useMemo<number[] | null>(() => (Array.isArray(rawTrace) ? rawTrace.filter((x: unknown): x is number => typeof x === 'number') : null), [rawTrace])
  const ghost = useMaxHold(trace, ghostOn, `${start}:${stop}`)
  const mkOn = status.MARKER !== 'OFF'
  const mkFreq = siNumber(status.MARKER_FREQ)
  const mkLevel = siNumber(status.MARKER_LEVEL)
  const mkX = mkOn ? freqFraction(mkFreq, start, stop) : null
  const mode = typeof status.SWEEP_MODE === 'string' ? status.SWEEP_MODE : null
  const peak = peakIndex(trace)
  const peakLevel = peak >= 0 ? trace![peak] : null

  // placing the marker by a click waits until the instrument reports it near the clicked frequency
  const pointSpacing = start != null && stop != null && trace && trace.length > 1 ? (stop - start) / (trace.length - 1) : 0
  const place = useConfirmation<number | null>(mkFreq, (a, b) => a != null && b != null && Math.abs(a - b) <= Math.max(pointSpacing, Math.abs(b) * 1e-6))
  const pick = profile.markerFreq && online && start != null && stop != null && stop > start
    ? (x: number) => {
      const f = Math.round(start + x * (stop - start))
      void run(api.set('marker_freq', f), () => place.begin(f))
    } : undefined

  const sendTo = (op: string, v: number) => api.set(op, v).then(() => v)
  const markerState = { freq: mkFreq, level: mkLevel, peakLevel, center }

  const groups: Group[] = ['FREQ', 'BW', 'AMPT', 'SWEEP',
    ...(profile.traceModes.length || profile.detectors.length ? ['TRACE' as Group] : []),
    'MKR', ...(profile.tg ? ['TG' as Group] : [])]

  const summary = `Spectrum ${freqText(start)} to ${freqText(stop)}, reference ${dbText(ref, unit, 1)}, ${scale} dB/div`
  return (
    <div className="bench-sal">
      <div className="kit-glass bench-sal__screen">
        <div className="bench-sal__top">
          <Annunciators items={[
            { label: mode ?? 'CONT', on: mode != null, title: 'Sweep mode' },
            { label: 'SWEEPING', on: status.SWEEPING === true },
            { label: status.TRACE_MODE ?? 'WRITE', on: status.TRACE_MODE != null, title: 'Trace mode' },
            { label: 'PA', on: status.PREAMP === 'ON', title: 'Preamplifier' },
            { label: 'TG', on: status.TG_OUTPUT === 'ON', alert: true, title: 'Generator output' },
          ]} />
          {mkOn && (mkFreq != null || mkLevel != null) && (
            <span className="bench-sal__mkr" data-confirm={place.state} aria-busy={place.state === 'waiting'}
              title={place.state === 'unconfirmed' ? NOT_CONFIRMED : 'Marker 1'}>
              <b>MKR1</b>
              <Recordable inline field="MARKER_FREQ" unit="Hz" tag="Marker freq">{freqText(mkFreq, 7)}</Recordable>
              <Recordable inline field="MARKER_LEVEL" unit={unit} tag="Marker level">{dbText(mkLevel, unit)}</Recordable>
            </span>
          )}
        </div>
        <SpectrumScreen trace={trace} ghost={ghost} refLevel={ref ?? 0} scale={scale} hdivs={profile.hdivs} vdivs={profile.vdivs}
          marker={mkX != null && mkLevel != null ? { x: mkX, level: mkLevel } : null} onPick={pick} label={summary} />
        <div className="bench-sal__ann" aria-label="Annotations">
          <span><b>CENTER</b>{freqText(center, 9)}</span>
          <span><b>SPAN</b>{freqText(span)}</span>
          <span><b>RBW</b>{siText(rbw, 'Hz', 3)}{status.RBW_AUTO === 'OFF' ? ' #' : ''}</span>
          {status.VBW !== undefined && <span><b>VBW</b>{siText(vbw, 'Hz', 3)}</span>}
          {status.SWEEP_TIME !== undefined && <span><b>SWT</b>{siText(swt, 's', 3)}</span>}
          <span><b>REF</b>{dbText(ref, unit, 1)}</span>
          {status.ATTEN !== undefined && <span><b>ATT</b>{atten != null ? `${atten} dB` : '-'}</span>}
          {typeof status.DETECTOR === 'string' && <span><b>DET</b>{status.DETECTOR}</span>}
          {status.POINTS != null && <span><b>PTS</b>{status.POINTS}</span>}
        </div>
      </div>

      <div className="bench-sal__keys">
        <div className="kit-seg bench-sal__groups" role="tablist" aria-label="Menu">
          {groups.map((g) => (
            <button key={g} type="button" role="tab" aria-selected={group === g} aria-pressed={group === g} onClick={() => setGroup(g)}>{g}</button>
          ))}
        </div>

        {group === 'FREQ' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Frequency">
            {profile.center && <DigitEditor label="Center" value={center} spec={profile.center} unit="Hz" group={3} disabled={!online} onCommit={(v) => api.set('center', v)} />}
            {profile.span && <DigitEditor label="Span" value={span} spec={profile.span} unit="Hz" group={3} disabled={!online} onCommit={(v) => api.set('span', v)} />}
            {profile.start && <DigitEditor label="Start" value={start} spec={profile.start} unit="Hz" group={3} disabled={!online} onCommit={(v) => api.set('start', v)} />}
            {profile.stop && <DigitEditor label="Stop" value={stop} spec={profile.stop} unit="Hz" group={3} disabled={!online} onCommit={(v) => api.set('stop', v)} />}
            <div className="kit-row">
              {profile.fullSpan && (
                <ActionButton label="Full span" disabled={!online} reported={{ start, stop }}
                  done={(now, before) => profile.freqRange
                    ? nearly(now.start, profile.freqRange.min) && nearly(now.stop, profile.freqRange.max)
                    : now.start !== before.start || now.stop !== before.stop}
                  onPress={() => api.set('full_span', 1)} />
              )}
              {profile.span && profile.span.min <= 0 && (
                <ActionButton label="Zero span" disabled={!online} reported={span} done={(now) => now === 0} onPress={() => api.set('span', 0)} />
              )}
            </div>
          </div>
        )}

        {group === 'BW' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Bandwidth">
            <div className="kit-row">
              {profile.rbw.length > 0 && (
                <Stepper label="RBW" disabled={!online || rbw == null} text={siText(rbw, 'Hz', 3)} value={rbw} format={(v) => siText(v, 'Hz', 3)}
                  onStep={(d) => sendTo('rbw', stepList(profile.rbw, rbw, d)!)} />
              )}
              {profile.rbwAuto && <MicroSwitch label="Auto RBW" checked={status.RBW_AUTO === 'ON'} disabled={!online} onChange={(on) => api.set('rbw_auto', on ? 'ON' : 'OFF')} />}
            </div>
            <div className="kit-row">
              {profile.vbw.length > 0 && (
                <Stepper label="VBW" disabled={!online || vbw == null} text={siText(vbw, 'Hz', 3)} value={vbw} format={(v) => siText(v, 'Hz', 3)}
                  onStep={(d) => sendTo('vbw', stepList(profile.vbw, vbw, d)!)} />
              )}
              {profile.vbwAuto && <MicroSwitch label="Auto VBW" checked={status.VBW_AUTO === 'ON'} disabled={!online} onChange={(on) => api.set('vbw_auto', on ? 'ON' : 'OFF')} />}
            </div>
            {!profile.rbw.length && !profile.vbw.length && <span className="bench-hint">No bandwidth settings on this instrument.</span>}
          </div>
        )}

        {group === 'AMPT' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Amplitude">
            <div className="kit-row">
              {profile.refLevel && (
                <Stepper label="Ref level" disabled={!online || ref == null} text={dbText(ref, unit, 1)} value={ref} format={(v) => dbText(v, unit, 1)}
                  title="One division per step"
                  onStep={(d) => sendTo('ref_level', clamp(Number(((ref ?? 0) + d * scale).toFixed(2)), profile.refLevel!.min, profile.refLevel!.max))} />
              )}
              {profile.attenuation && (
                <Stepper label="Attenuation" disabled={!online || atten == null} text={atten != null ? `${atten} dB` : '-'} value={atten} format={(v) => `${v} dB`}
                  onStep={(d) => sendTo('attenuation', clamp((atten ?? 0) + d * profile.attenuation!.step, profile.attenuation!.min, profile.attenuation!.max))} />
              )}
            </div>
            {profile.scales.length > 0 && (
              <div className="kit-field">
                <span className="kit-label">Scale</span>
                <Selector variant="chips" label="Scale" disabled={!online} value={profile.scales.find((s) => s === scale)?.toString() ?? null}
                  options={profile.scales.map((s) => ({ value: String(s), label: `${s} dB/div` }))} onChange={(v) => run(api.set('scale', Number(v)))} />
              </div>
            )}
            <div className="kit-row">
              {profile.attenAuto && <MicroSwitch label="Auto att" checked={status.ATTEN_AUTO === 'ON'} disabled={!online} onChange={(on) => api.set('atten_auto', on ? 'ON' : 'OFF')} />}
              {profile.preamp && <MicroSwitch label="Preamp" checked={status.PREAMP === 'ON'} disabled={!online} onChange={(on) => api.set('preamp', on ? 'ON' : 'OFF')} />}
            </div>
          </div>
        )}

        {group === 'SWEEP' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Sweep">
            <div className="kit-row">
              {profile.sweeps.filter((s) => s !== 'RESTART').length > 0 && (
                <Selector label="Sweep mode" disabled={!online} value={mode}
                  options={profile.sweeps.filter((s) => s !== 'RESTART').map((s) => ({ value: s, label: SWEEP_LABEL[s] ?? s }))}
                  onChange={(v) => run(api.set('sweep', v))} />
              )}
              {profile.sweeps.includes('RESTART') && (
                <ActionButton label="Restart" title="Restart the sweep" disabled={!online} reported={status.TRACE} done={(now, before) => now !== before}
                  onPress={() => api.set('sweep', 'RESTART')} />
              )}
            </div>
            {profile.sweepTime && (
              <Stepper label="Sweep time" disabled={!online || swt == null} text={siText(swt, 's', 3)} value={swt} format={(v) => siText(v, 's', 3)}
                onStep={(d) => sendTo('sweep_time', step125(swt!, d, profile.sweepTime!.min, profile.sweepTime!.max))} />
            )}
          </div>
        )}

        {group === 'TRACE' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Trace">
            {profile.traceModes.length > 0 && (
              <Selector variant="chips" label="Trace mode" disabled={!online} value={status.TRACE_MODE ?? null}
                options={profile.traceModes.map((m) => ({ value: m, label: TRACE_LABEL[m] ?? m }))} onChange={(v) => run(api.set('trace_mode', v))} />
            )}
            {profile.detectors.length > 0 && (
              <div className="kit-field">
                <span className="kit-label">Detector</span>
                <Selector variant="chips" label="Detector" disabled={!online} value={status.DETECTOR ?? null}
                  options={profile.detectors.map((m) => ({ value: m, label: m }))} onChange={(v) => run(api.set('detector', v))} />
              </div>
            )}
          </div>
        )}

        {group === 'MKR' && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Marker">
            <div className="kit-row">
              {profile.marker && <MicroSwitch label="Marker 1" checked={mkOn && status.MARKER === 'ON'} disabled={!online} onChange={(on) => api.set('marker', on ? 'ON' : 'OFF')} />}
              <MicroSwitch label="Max hold (screen)" checked={ghostOn} title="Kept on this screen only; the instrument is not changed" onChange={(on) => setGhostOn(on)} />
            </div>
            {profile.markerFreq && <DigitEditor label="Marker freq" value={mkFreq} spec={profile.markerFreq} unit="Hz" group={3} disabled={!online} onCommit={(v) => api.set('marker_freq', v)} />}
            <div className="kit-row">
              {profile.peaks.filter((p) => p !== 'MKR_CF').map((p) => (
                <ActionButton key={p} label={PEAK_LABEL[p] ?? p} disabled={!online} reported={markerState}
                  done={(now, before) => now.freq !== before.freq || (p === 'PEAK' && now.level != null && now.peakLevel != null && Math.abs(now.level - now.peakLevel) <= 0.5)}
                  onPress={() => api.set('peak_search', p)} />
              ))}
              {(profile.peaks.includes('MKR_CF') || (profile.center && mkFreq != null)) && (
                <ActionButton label={PEAK_LABEL.MKR_CF} title="Marker frequency to center" disabled={!online || mkFreq == null} reported={markerState}
                  done={(now) => now.center != null && now.freq != null && Math.abs(now.center - now.freq) <= Math.max(pointSpacing, 1)}
                  onPress={() => (profile.peaks.includes('MKR_CF') ? api.set('peak_search', 'MKR_CF') : api.set('center', mkFreq!))} />
              )}
            </div>
            {profile.markerFreq && <span className="bench-hint">Click the screen to place the marker.</span>}
          </div>
        )}

        {group === 'TG' && profile.tg && (
          <div className="bench-sal__group" role="tabpanel" aria-label="Generator">
            <ArmSwitch label="Generator output" on={status.TG_OUTPUT === 'ON'} disabled={!online} onChange={(on) => api.set('tg_output', on ? 'ON' : 'OFF')} />
            {profile.tg.level && (
              <DigitEditor label="Generator level" value={siNumber(status.TG_LEVEL)} spec={profile.tg.level} unit="dBm" disabled={!online} onCommit={(v) => api.set('tg_level', v)} />
            )}
          </div>
        )}
        {error && <span className="kit-error" role="alert">{error}</span>}
      </div>
    </div>
  )
}
