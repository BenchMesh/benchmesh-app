import React from 'react'
import { ArmSwitch, DigitEditor, Selector, WavePreview, WaveGlyph, siNumber } from '../../kit'
import { AwgProfile } from '../profiles'
import { ChannelApi, useAction } from '../api'
import { Recordable } from '../../recording/signal'

/** One generator channel: waveform, output, preview, frequency / amplitude / offset. */
export function AwgChannel({ ch, status, profile, api, online }: {
  ch: string; status: Record<string, any>; profile: AwgProfile; api: ChannelApi; online: boolean
}) {
  const [error, run] = useAction()
  const shape = typeof status.SHAPE === 'string' ? status.SHAPE.toUpperCase() : ''
  const freq = siNumber(status.FREQ)
  const ampl = siNumber(status.AMPL)
  const offs = siNumber(status.OFFS)
  const on = status.OUTPUT === 'ON'
  const color = profile.channelColors[ch]

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
        {profile.output && <ArmSwitch label={`CH${ch}`} on={on} disabled={!online} onChange={(v) => api.set('output', v ? 'ON' : 'OFF')} />}
      </div>
      <div className="kit-glass">
        <WavePreview shape={shape} amplitude={ampl ?? 0} offset={offs ?? 0} on={on}
          maxAmplitude={profile.amplitude?.max ?? 20} maxOffset={profile.offset?.max ?? 10} />
      </div>
      <div className="kit-row">
        {profile.frequency && (
          <Recordable field="FREQ" unit="Hz" focusable={false}>
            <DigitEditor label="Frequency" value={freq} spec={profile.frequency} unit="Hz" group={3} disabled={!online}
              onCommit={(v) => api.set('frequency', v)} />
          </Recordable>
        )}
        {profile.amplitude && (
          <Recordable field="AMPL" unit={profile.amplitude.unit} focusable={false}>
            <DigitEditor label="Amplitude" value={ampl} spec={profile.amplitude} unit={profile.amplitude.unit} disabled={!online}
              onCommit={(v) => api.set('amplitude', v)} />
          </Recordable>
        )}
        {profile.offset && (
          <Recordable field="OFFS" unit={profile.offset.unit} focusable={false}>
            <DigitEditor label="Offset" value={offs} spec={profile.offset} unit={profile.offset.unit} disabled={!online}
              onCommit={(v) => api.set('offset', v)} />
          </Recordable>
        )}
      </div>
      {error && <span className="kit-error" role="alert">{error}</span>}
    </div>
  )
}
