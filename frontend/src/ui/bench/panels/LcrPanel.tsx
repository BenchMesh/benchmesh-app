import React, { useEffect, useState } from 'react'
import { Readout, Annunciators, Selector, MicroSwitch, ArmSwitch, DigitEditor, ActionButton, prefixed, siNumber, siText } from '../../kit'
import { nearly } from '../../kit/confirm'
import { LcrFunction, LcrProfile, LcrSetting, lcrFunction } from '../profiles'
import { ChannelApi, useAction, useHistory, useSetpoint } from '../api'
import { useStats } from './DmmPanel'
import { Recordable } from '../../recording/signal'
import './lcr.css'

/** Unitless (D, Q) and angle readings: fixed significant digits, no SI prefix. */
export function plainText(x: number | null, counts: number, overload = false): string {
  if (x == null || !Number.isFinite(x)) return overload ? '  OL  ' : ' ' + '-'.repeat(counts)
  const ax = Math.abs(x)
  const intDigits = Math.max(1, Math.floor(Math.log10(ax || 1)) + 1)
  const decimals = Math.min(6, Math.max(0, counts - intDigits))
  return (x < 0 ? '-' : ' ') + ax.toFixed(decimals)
}

/** Values the UI derives from the pair (display only, never sent): Q = 1/D, ESR of Cs-D, Rs of Ls-Q. */
export function derived(fn: LcrFunction | null, primary: number | null, secondary: number | null, freq: number | null): { label: string; text: string }[] {
  if (!fn || secondary == null || !Number.isFinite(secondary) || secondary === 0) return []
  const out: { label: string; text: string }[] = []
  const w = freq ? 2 * Math.PI * freq : null
  if (fn.secondary === 'D') {
    out.push({ label: 'Q', text: plainText(1 / secondary, 5).trim() })
    if (fn.primary === 'Cs' && w && primary) out.push({ label: 'ESR', text: siText(secondary / (w * primary), 'Ω') })
  } else if (fn.secondary === 'Q') {
    out.push({ label: 'D', text: plainText(1 / secondary, 5).trim() })
    if (fn.primary === 'Ls' && w && primary) out.push({ label: 'Rs', text: siText((w * primary) / secondary, 'Ω') })
  }
  return out
}

/** Readings with a primary value seen so far: advances on every status update that carries one. */
function useReadingCount(status: Record<string, any>): number {
  const ok = siNumber(status.PRIMARY) != null
  const [s, setS] = useState({ status, n: 0 })
  if (s.status !== status) {
    const n = s.n + (ok ? 1 : 0)
    setS({ status, n })
    return n
  }
  return s.n
}

const CORRECTION: Record<string, { label: string; prompt: string }> = {
  OPEN: { label: 'Open', prompt: 'Open the test clips: nothing connected, clips apart as in a measurement.' },
  SHORT: { label: 'Short', prompt: 'Short the test clips together (or use the shorting bar).' },
  AUTO: { label: 'Correct', prompt: 'Open or short the test clips; the meter decides which correction to run.' },
}
const SPEED_LABEL: Record<string, string> = { FAST: 'Fast', MED: 'Med', SLOW: 'Slow' }

const unique = <T,>(xs: T[]) => xs.filter((x, i) => xs.indexOf(x) === i)

/** Best function for a primary key / secondary / circuit choice, keeping what still fits. */
export function pickFunction(functions: LcrFunction[], current: LcrFunction | null, want: { group?: string; secondary?: string | null; circuit?: string | null }): LcrFunction | null {
  const group = want.group ?? current?.group
  const inGroup = functions.filter((f) => f.group === group)
  if (!inGroup.length) return null
  const secondary = want.secondary !== undefined ? want.secondary : current?.secondary
  const circuit = want.circuit !== undefined ? want.circuit : current?.circuit
  const bySec = inGroup.filter((f) => f.secondary === secondary)
  const pool = bySec.length ? bySec : inGroup
  return pool.find((f) => f.circuit === circuit) ?? pool[0]
}

function SettingControl({ label, setting, value, unit, onSet, online }: {
  label: string; setting: LcrSetting; value: number | null; unit: string; onSet: (v: string | number) => Promise<unknown>; online: boolean
}) {
  const [error, run] = useAction()
  if (!setting) return null
  if (setting.kind === 'list') {
    const current = setting.values.find((o) => nearly(o.num, value))?.value ?? null
    return (
      <div className="kit-field">
        <span className="kit-label">{label}</span>
        <Selector variant="chips" label={label} disabled={!online} value={current}
          options={setting.values.map((o) => ({ value: o.value, label: siText(o.num, unit, 3) }))}
          onChange={(v) => run(onSet(v))} />
        {error && <span className="kit-error" role="alert">{error}</span>}
      </div>
    )
  }
  const preset = setting.presets.find((p) => nearly(p, value))
  return (
    <div className="kit-field">
      <DigitEditor label={label} value={value} spec={setting.spec} unit={unit} group={unit === 'Hz' ? 3 : 0} disabled={!online}
        onCommit={async (v) => { await onSet(v) }} />
      {setting.presets.length > 0 && (
        <Selector variant="chips" label={`${label} presets`} disabled={!online} value={preset != null ? String(preset) : null}
          options={setting.presets.map((p) => ({ value: String(p), label: siText(p, unit, 3) }))}
          onChange={(v) => run(onSet(Number(v)))} />
      )}
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}

/**
 * LCR meter: large primary and smaller secondary readout with the setup annunciators
 * (frequency, level, SER/PAR, AUTO, speed, bias, OL, BIN), statistics; function keys
 * L / C / R / Z / DCR → secondary → SER / PAR, frequency, level, range, speed, trigger,
 * open/short correction and DC bias.
 */
export function LcrPanel({ status, profile, api, online }: {
  status: Record<string, any>; profile: LcrProfile; api: ChannelApi; online: boolean
}) {
  const [error, run] = useAction()
  const funcKey = typeof status.FUNC === 'string' ? status.FUNC : null
  const fn = profile.functions.find((f) => f.key === funcKey) ?? (funcKey ? lcrFunction(funcKey) : null)
  const overload = status.OVERLOAD === true
  const raw = siNumber(status.PRIMARY)
  const sec = siNumber(status.SECONDARY)
  const freq = siNumber(status.FREQ)
  const [holdOn, setHoldOn] = useState(false)
  const [hold, setHold] = useState<{ p: number | null; s: number | null }>({ p: null, s: null })
  const [rel, setRel] = useState<number | null>(null)
  const live = holdOn ? hold.p : raw
  const shown = live != null && rel != null ? live - rel : live
  const secShown = holdOn ? hold.s : sec
  const [funcSeen, setFuncSeen] = useState(funcKey)
  if (funcSeen !== funcKey) { setFuncSeen(funcKey); setRel(null) }
  const stats = useStats(holdOn ? null : shown, status, funcKey)
  const history = useHistory(holdOn ? null : shown, status, 150, funcKey)
  const readings = useReadingCount(status)

  const [levelSet, commitLevel] = useSetpoint(api, profile.level?.kind === 'range' ? 'level' : null, siNumber, online)
  const [biasSet, commitBias] = useSetpoint(api, profile.bias ? 'bias' : null, siNumber, online)
  const level = siNumber(status.LEVEL) ?? levelSet
  const bias = siNumber(status.BIAS) ?? biasSet
  const [trigSrc, setTrigSrc] = useState<string | null>(null)
  useEffect(() => {
    if (!profile.triggerSources.length || !online) return
    api.get('trigger_source').then((v) => typeof v === 'string' && setTrigSrc(v.trim().toUpperCase())).catch(() => {})
  }, [api, profile.triggerSources.length, online])
  const [corrSwitch, setCorrSwitch] = useState<Record<string, boolean>>({})
  const [prompt, setPrompt] = useState<string | null>(null)
  // a bias setter that applies the bias at once: the first Enter arms, the second (same value) sends
  const [biasArmed, setBiasArmed] = useState<number | null>(null)
  const applyBias = async (v: number) => {
    if (profile.biasEnergizes && v !== 0 && biasArmed !== v) {
      setBiasArmed(v)
      throw new Error('Press Enter again to apply this DC bias to the part')
    }
    setBiasArmed(null)
    await commitBias(v)
  }

  const pu = fn?.units[0] ?? ''
  const su = fn?.units[1] ?? ''
  const main = prefixed(shown, profile.counts, overload)
  const secPrefixed = su === '' || su === '°' || su === 'rad'
  const secMain = secPrefixed ? { text: plainText(secShown, profile.counts, overload), prefix: '' } : prefixed(secShown, profile.counts, overload)
  const fmt = (x: number) => { const p = prefixed(x, profile.counts); return `${p.text.trim()} ${p.prefix}${pu}` }
  const range = typeof status.RANGE === 'string' ? status.RANGE : null
  const circuit = status.CIRCUIT === 'SER' || status.CIRCUIT === 'PAR' ? status.CIRCUIT : fn?.circuit ?? null

  // function keys: primary group → secondary → circuit, each resolving to one function key
  const setFunction = (f: LcrFunction | null) => (f ? run(api.set('function', f.key)) : Promise.resolve(false))
  const groups = unique(profile.functions.map((f) => f.group))
  const inGroup = profile.functions.filter((f) => f.group === fn?.group)
  const secondaries = unique(inGroup.map((f) => f.secondary).filter((s): s is string => s != null))
  const circuits = unique(inGroup.filter((f) => f.secondary === fn?.secondary).map((f) => f.circuit).filter((c): c is 'SER' | 'PAR' => c != null))

  const freqLabel = freq != null ? siText(freq, 'Hz', 3) : 'FREQ'
  const levelLabel = level != null ? siText(level, 'V', 3) : 'LEVEL'
  const speed = typeof status.SPEED === 'string' ? status.SPEED : null
  const bin = typeof status.BIN === 'string' ? status.BIN : null

  return (
    <div className="bench-lcr">
      <div className="kit-glass bench-lcr__glass">
        <Recordable field="PRIMARY" unit={pu} tag={fn?.primary}>
          <Readout tag={fn?.primary ?? '-'} text={main.text} unit={`${main.prefix}${pu}`} label={fn ? `Primary ${fn.primary}` : 'Primary'} tone="meas" history={history} />
        </Recordable>
        {fn?.secondary && (
          <Recordable field="SECONDARY" unit={su} tag={fn.secondary}>
            <Readout size="mid" tag={fn.secondary} text={secMain.text} unit={`${secMain.prefix}${su}`} label={`Secondary ${fn.secondary}`} />
          </Recordable>
        )}
        <div className="kit-stats" aria-label="Statistics">
          <span><b>MIN</b>{stats.n ? fmt(stats.min) : '-'}</span>
          <span><b>MAX</b>{stats.n ? fmt(stats.max) : '-'}</span>
          <span><b>AVG</b>{stats.n ? fmt(stats.sum / stats.n) : '-'}</span>
          <span><b>N</b>{stats.n}</span>
          {derived(fn, raw, sec, freq).map((d) => <span key={d.label}><b>{d.label}</b>{d.text}</span>)}
        </div>
        <Annunciators items={[
          { label: 'SER', on: circuit === 'SER' },
          { label: 'PAR', on: circuit === 'PAR' },
          { label: 'AUTO', on: range === 'AUTO', title: 'Auto range' },
          { label: freqLabel, on: freq != null, title: 'Test frequency' },
          { label: levelLabel, on: level != null, title: 'Test level' },
          ...(speed ? [{ label: speed, on: true, title: 'Measurement speed' }] : []),
          { label: 'BIAS', on: status.BIAS_ON === 'ON', alert: true, title: 'DC bias applied' },
          { label: 'OL', on: overload, alert: true },
          ...(bin ? [{ label: bin, on: true, title: 'Comparator' }] : []),
          { label: 'HOLD', on: holdOn },
          { label: 'REL Δ', on: rel != null },
          { label: 'RMT', on: status.REMOTE === 'ON', title: 'Remote' },
        ]} />
      </div>

      <div className="bench-lcr__controls">
        {profile.canFunction && groups.length > 0 && (
          <div className="kit-field">
            <span className="kit-label">Function</span>
            <Selector label="Primary parameter" disabled={!online} value={fn?.group ?? null}
              options={groups.map((g) => ({ value: g, label: g }))}
              onChange={(g) => setFunction(pickFunction(profile.functions, fn, { group: g }))} />
            {secondaries.length > 1 && (
              <Selector variant="chips" label="Secondary parameter" disabled={!online} value={fn?.secondary ?? null}
                options={secondaries.map((s) => ({ value: s, label: s }))}
                onChange={(s) => setFunction(pickFunction(profile.functions, fn, { secondary: s }))} />
            )}
            {circuits.length > 1 && (
              <Selector label="Equivalent circuit" disabled={!online} value={fn?.circuit ?? null}
                options={circuits.map((c) => ({ value: c, label: c, title: c === 'SER' ? 'Series' : 'Parallel' }))}
                onChange={(c) => setFunction(pickFunction(profile.functions, fn, { circuit: c }))} />
            )}
          </div>
        )}
        {profile.circuit && circuits.length <= 1 && fn?.circuit == null && (
          <Selector label="Equivalent circuit" disabled={!online} value={circuit}
            options={[{ value: 'SER', label: 'SER' }, { value: 'PAR', label: 'PAR' }]} onChange={(c) => run(api.set('circuit', c))} />
        )}
        <SettingControl label="Frequency" setting={profile.frequency} value={freq} unit="Hz" online={online} onSet={(v) => api.set('frequency', v)} />
        <SettingControl label="Level" setting={profile.level} value={level} unit="V" online={online}
          onSet={(v) => (typeof v === 'number' ? commitLevel(v) : api.set('level', v))} />
        <div className="kit-row">
          {profile.ranges.length > 0 && (
            <div className="kit-field">
              <span className="kit-label">Range</span>
              <Selector variant="chips" label="Range" disabled={!online} value={range}
                options={profile.ranges.map((r) => ({ value: r.value, label: r.display }))} onChange={(v) => run(api.set('range', v))} />
            </div>
          )}
          {profile.speeds.length > 0 && (
            <div className="kit-field">
              <span className="kit-label">Speed</span>
              <Selector label="Speed" disabled={!online} value={speed}
                options={profile.speeds.map((s) => ({ value: s.value, label: SPEED_LABEL[s.value] ?? s.value, title: s.display }))}
                onChange={(v) => run(api.set('speed', v))} />
            </div>
          )}
        </div>
        <div className="kit-row">
          <MicroSwitch label="Hold" checked={holdOn} onChange={(on) => { setHoldOn(on); setHold(on ? { p: raw, s: sec } : { p: null, s: null }) }} />
          <MicroSwitch label="Rel Δ" checked={rel != null} onChange={(on) => setRel(on ? raw : null)} />
          {profile.remote && <MicroSwitch label="Remote" checked={status.REMOTE === 'ON'} disabled={!online} onChange={(on) => api.set('remote', on ? 'ON' : 'OFF')} />}
        </div>
        {(profile.triggerSources.length > 0 || profile.trigger) && (
          <div className="kit-row">
            {profile.triggerSources.length > 0 && (
              <Selector label="Trigger source" disabled={!online} value={trigSrc}
                options={profile.triggerSources.map((s) => ({ value: s, label: s }))}
                onChange={(v) => run(api.set('trigger_source', v).then(() => api.get('trigger_source')).then((r) => typeof r === 'string' && setTrigSrc(r.trim().toUpperCase())))} />
            )}
            {profile.trigger && (
              <ActionButton label="Trigger" title="Measure once (bus / manual trigger source)" disabled={!online}
                reported={readings} done={(now, before) => now > before} onPress={() => api.set('trigger', 'ONCE')} />
            )}
          </div>
        )}
        {profile.corrections.length > 0 && (
          <div className="kit-field">
            <span className="kit-label">Correction</span>
            <div className="kit-row">
              {profile.corrections.map((c) => (
                <button key={c} type="button" className="bench-btn" aria-pressed={prompt === c} disabled={!online}
                  onClick={() => setPrompt(prompt === c ? null : c)}>{CORRECTION[c]?.label ?? c}</button>
              ))}
              {profile.correctionSwitches.map((op) => (
                <MicroSwitch key={op} label={op === 'open_correction' ? 'Use open' : 'Use short'} checked={!!corrSwitch[op]} disabled={!online}
                  title="Apply the stored correction (the instrument does not report this setting)"
                  onChange={async (on) => { await api.set(op, on ? 'ON' : 'OFF'); setCorrSwitch((s) => ({ ...s, [op]: on })) }} />
              ))}
            </div>
            {prompt && (
              <div className="bench-lcr__prompt" role="dialog" aria-label={`${CORRECTION[prompt]?.label ?? prompt} correction`}>
                <span>{CORRECTION[prompt]?.prompt ?? 'Prepare the fixture.'} The correction can take a while; readings pause meanwhile.</span>
                <div className="kit-row">
                  <ActionButton primary label="Run correction" disabled={!online} reported={readings}
                    done={(now, before) => now >= before + 2}
                    onPress={async () => { await api.set('correction', prompt); setPrompt(null) }} />
                  <button type="button" className="bench-btn" onClick={() => setPrompt(null)}>Cancel</button>
                </div>
              </div>
            )}
          </div>
        )}
        {(profile.bias || profile.biasOutput) && (
          <div className="kit-row">
            {profile.bias && <DigitEditor label="DC bias" value={bias} spec={profile.bias} unit="V" disabled={!online} onCommit={applyBias} />}
            {profile.biasOutput && <ArmSwitch label="Bias" on={status.BIAS_ON === 'ON'} disabled={!online} onChange={(on) => api.set('bias_output', on ? 'ON' : 'OFF')} />}
          </div>
        )}
        {error && <span className="kit-error" role="alert">{error}</span>}
      </div>
    </div>
  )
}
