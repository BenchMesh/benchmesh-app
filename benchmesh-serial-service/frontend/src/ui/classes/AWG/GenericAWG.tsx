import React, { useMemo, useState } from 'react'
import { useRequestLog, loggedFetch } from '../../RequestLogContext'
import { apiBaseUrl } from '../../../api/base'

// Generic AWG channel panel: live status from the registry (AWG.status_ch<N>:
// OUTPUT, SHAPE, FREQ, AMPL, OFFS, PHASE, SYMM) and an output on/off toggle.
export function GenericAWG({ channelPath, registry }: { channelPath?: string, registry?: any }) {
  const apiBase = apiBaseUrl()
  const { addLog } = useRequestLog()
  const { klass, deviceId, channel } = useMemo(() => parsePath(channelPath), [channelPath])
  const [busy, setBusy] = useState(false)
  const status = (deviceId && klass && channel && registry?.[deviceId]?.[klass]?.[`status_ch${channel}`]) || {}
  const outputOn = status.OUTPUT === 'ON'

  const setOutput = async (on: boolean) => {
    if (!klass || !deviceId || !channel || busy) return
    setBusy(true)
    try {
      await loggedFetch(`${apiBase}/instruments/${klass}/${deviceId}/${channel}/output/${on ? 'ON' : 'OFF'}`, {
        method: 'POST',
        instrument: deviceId,
        channel,
        action: on ? 'Output ON' : 'Output OFF',
        parameters: { output: on ? 'ON' : 'OFF' },
        addLog,
      })
    } catch (err) {
      console.debug('AWG output change failed', err)
    } finally {
      setBusy(false)
    }
  }

  const rows: [string, string][] = [
    ['Waveform', status.SHAPE ?? '—'],
    ['Frequency', si(status.FREQ, 'Hz')],
    ['Amplitude', si(status.AMPL, 'Vpp')],
    ['Offset', si(status.OFFS, 'V')],
    ['Phase', typeof status.PHASE === 'number' ? `${round(status.PHASE)}°` : '—'],
  ]
  if (status.SHAPE === 'RAMP') rows.push(['Symmetry', typeof status.SYMM === 'number' ? `${round(status.SYMM)} %` : '—'])

  return (
    <div className="awg-face">
      <div className="awg-main">
        <div className="awg-section">
          <div className="awg-section-title">Channel {channel ?? '—'}</div>
          {rows.map(([label, value]) => (
            <div className="awg-row" key={label}>
              <span className="awg-label">{label}</span>
              <span className="awg-value">{value}</span>
            </div>
          ))}
        </div>
      </div>
      <div className="awg-actions">
        <button
          type="button"
          className={`awg-output ${outputOn ? 'on' : 'off'}`}
          disabled={busy || !deviceId}
          aria-pressed={outputOn}
          title={`POST /instruments/${klass || 'AWG'}/${deviceId || '{id}'}/${channel || '1'}/output/${outputOn ? 'OFF' : 'ON'}`}
          onClick={() => setOutput(!outputOn)}
        >
          Output {status.OUTPUT ?? '—'}
        </button>
        <span className="psu-api" title={`GET /instruments/${klass || 'AWG'}/${deviceId || '{id}'}/${channel || '1'}/output`}>API</span>
      </div>
    </div>
  )
}

/** "1 kHz" from a BenchMesh SI value dict. */
function si(v: any, unit: string): string {
  const n = v?.si?.number
  if (n == null || n === '') return '—'
  return `${n} ${v.si.symbol ?? ''}${unit}`
}

function round(x: number): string {
  return String(Math.round(x * 100) / 100)
}

function parsePath(channelPath?: string): { klass?: string, deviceId?: string, channel?: string } {
  if (!channelPath) return {}
  const parts = channelPath.split('/').filter(Boolean)
  if (parts.length < 3) return {}
  return { klass: parts[1], deviceId: parts[2], channel: parts[3] }
}

export default GenericAWG
