import React, { useState } from 'react'
import { Readout, Annunciators, DigitEditor, ArmSwitch, MicroSwitch, Bargraph, fixedText, siNumber, intDigitsFor } from '../../kit'
import { PsuProfile } from '../profiles'
import { ChannelApi, useHistory, useSetpoint } from '../api'

/** Annunciators shown when the instrument reports them (OUT and REM always). */
const PSU_FLAGS: { key: string; label: string; alert?: boolean }[] = [
  { key: 'CV', label: 'CV' }, { key: 'CC', label: 'CC' }, { key: 'OVP', label: 'OVP', alert: true },
  { key: 'OCP', label: 'OCP', alert: true }, { key: 'OTP', label: 'OTP', alert: true }, { key: 'FAIL', label: 'FAIL', alert: true },
]

/** One power-supply channel: readouts on the glass, setpoints and output beside it. */
export function PsuPanel({ status, profile, api, online }: {
  status: Record<string, any>; profile: PsuProfile; api: ChannelApi; online: boolean
}) {
  const vout = siNumber(status.VOUT)
  const iout = siNumber(status.IOUT)
  const pout = siNumber(status.POUT) ?? (vout != null && iout != null ? vout * iout : null)
  const vHist = useHistory(vout, status)
  const iHist = useHistory(iout, status)
  const [vset, commitV] = useSetpoint(api, profile.voltage ? 'voltage' : null, siNumber)
  const [iset, commitI] = useSetpoint(api, profile.current ? 'current' : null, siNumber)
  const [ovpLevel, commitOvp] = useSetpoint(api, profile.voltageLimit ? 'voltage_limit' : null, siNumber)
  const [ocpLevel, commitOcp] = useSetpoint(api, profile.currentLimit ? 'current_limit' : null, siNumber)
  const [protect, setProtect] = useState<Record<string, boolean>>({})

  const vDig = profile.voltage ?? { intDigits: 2, decimals: 2 }
  const iDig = profile.current ?? { intDigits: 1, decimals: 3 }
  const pDigits = intDigitsFor(profile.maxPower ?? (vDig.intDigits + iDig.intDigits > 2 ? 100 : 10))
  const outputOn = status.OUTPUT === 'ON'
  const remote = status.REMOTE === 'ON'
  const flags = PSU_FLAGS.filter((f) => f.key in status).map((f) => ({ label: f.label, on: status[f.key] === true, alert: f.alert }))

  return (
    <div className="bench-psu">
      <div className="kit-glass bench-psu__glass">
        <Readout tag="VOUT" text={fixedText(vout, vDig.intDigits, vDig.decimals)} unit="V" tone="volt" history={vHist} label="Output voltage" />
        <Readout tag="IOUT" text={fixedText(iout, iDig.intDigits, iDig.decimals)} unit="A" tone="amp" history={iHist} label="Output current" />
        <Readout tag="POUT" size="mid" text={fixedText(pout, pDigits, 2)} unit="W" tone="watt" label="Output power" />
        <Annunciators items={[...flags, { label: 'OUT', on: outputOn }, { label: 'REM', on: remote }]} />
      </div>
      <div className="bench-psu__controls">
        {profile.voltage && <DigitEditor label="Voltage setpoint" value={vset} spec={profile.voltage} unit="V" onCommit={commitV} disabled={!online} />}
        {profile.current && <DigitEditor label="Current limit" value={iset} spec={profile.current} unit="A" onCommit={commitI} disabled={!online} />}
        {profile.current && (
          <div className="kit-field">
            <span className="kit-label">Headroom to current limit</span>
            <Bargraph tone="amp" label="Output current as a fraction of the current limit"
              fraction={iset ? (iout ?? 0) / iset : 0} marks={['0', '50%', iset != null ? `${iset.toFixed(iDig.decimals)} A` : '—']} />
          </div>
        )}
        {(profile.voltageLimit || profile.currentLimit) && (
          <div className="kit-row">
            {profile.voltageLimit && <DigitEditor label="OVP level" value={ovpLevel} spec={profile.voltageLimit} unit="V" onCommit={commitOvp} disabled={!online} />}
            {profile.currentLimit && <DigitEditor label="OCP level" value={ocpLevel} spec={profile.currentLimit} unit="A" onCommit={commitOcp} disabled={!online} />}
          </div>
        )}
        <div className="kit-row">
          {profile.output && <ArmSwitch label="Output" on={outputOn} onChange={(on) => api.set('output', on ? 'ON' : 'OFF')} disabled={!online} />}
          {profile.protectionSwitches.map((op) => (
            <MicroSwitch key={op} label={op.toUpperCase()} checked={!!protect[op]} disabled={!online}
              title={`${op.toUpperCase()} protection (the instrument does not report this setting)`}
              onChange={async (on) => { await api.set(op, on ? 'ON' : 'OFF'); setProtect((p) => ({ ...p, [op]: on })) }} />
          ))}
          {profile.remote && <MicroSwitch label="Remote" checked={remote} disabled={!online} onChange={(on) => api.set('remote', on ? 'ON' : 'OFF')} />}
        </div>
      </div>
    </div>
  )
}
