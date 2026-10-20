# PSU: common functional denominator and software-level value

Researched 2026-10-20 for BenchMesh (read-only research, no repo file changed).
Class contract: `devices/classes/PSU.yaml`. Panel: `frontend/src/ui/bench/panels/PsuPanel.tsx`,
profile `psuProfile()` in `frontend/src/ui/bench/profiles.ts`.

**Sources and abbreviations** (local manuals under `/home/marek/project/BenchMeshDOC/<family>/doc/`,
read with pdftotext; page numbers as printed unless "PDF p."):

| Tag | Family (column) | Source |
|---|---|---|
| KA | Korad KA/KD + Tenma 72-xxxx | `tenma_72/doc/Series+Protocol+V2.0+of+Remote+Control.pdf` pp.1-2 (TENMA), `TENMA_72-2540_User_Manual.pdf`, coverage `ai_context/migration/coverage/PSU.md` §2.1 (KA33 manual is vector-only) |
| RD | Riden RD6006/6012/6018 | no vendor doc; community register map `github.com/Baldanos/rd6006/blob/master/registers.md` (fetched 2026-10-20), coverage §2.2 |
| DPM | Juntek DPM86xx (Joy-IT) | `juntek_dpm86xx/doc/JT-DPM86XX_Communication-protocol_2025-07-22.pdf` PDF p.3-5 |
| SPE | OWON SPE/SP/SPM (PSU half) | `owon_sp_spe_sps/doc/SP_and_SPE_SPS_programming_manual.pdf` p.6-12, `SPE_Series_..._USER_MANUAL.pdf` §4.4 |
| PPS | Hantek PPS2116A/2320A | `hantek_pps/doc/hantek_power_supply_communication_protocol.pdf` pp.1-2 |
| DP8 | Rigol DP800 (DP700 = subset) | `rigol_dp800/doc/DP800_ProgrammingGuide_EN_2015.pdf` (PG800, chapter-page), `rigol_dp700/doc/DP700_ProgrammingGuide.pdf` |
| S33 | Siglent SPD3303X/X-E | `siglent_spd3303/doc/SPD3303X_QuickStart.pdf` (PDF pages) |
| S4K | Siglent SPD4000X | `siglent_spd4000/doc/SPD4000X_UserManual_ProgrammingGuide.pdf` |
| GPP | GW Instek GPP-x323 | `gwinstek_gpp/doc/GPP_Series_User_Manual.pdf` (PDF pages) |
| UDP | UNI-T UDP3305S | `unit_udp/doc/UDP3000S_Series_Programming_Manual_V1.1.pdf`; UDP6720 from `UDP6720_Series_Programming_Manual_V1.2.pdf` |
| E36 | Keysight E36300 | **no manual local**; sigrok `keysight_e36300a_cmd` via coverage §2.17, rest from vendor convention, marked `?` |
| TTi | Aim-TTi MX100TP / CPX400DP | `tti_mx/doc/MX100T_MX100TP_Instruction_Manual...pdf` p.35-37, `tti_cpx/doc/CPX400D+DP_...pdf` PDF p.28-30 |

Pro SMUs (Keysight B2900, Keithley 2450) are noted where they set the ceiling (B2900 SCPI
reference `keysight_b2900/doc/B2900A_SCPI_Reference_9018-01705.pdf`, e.g. `:TRACe:STATistic:DATA?`
p.4-173, `:SENSe:REMote`); they are SMUs, not PSU-class targets, so they are not columns.

---

## 1. Feature matrix

`Y` = remote command exists, `P` = partial (see note), `F` = front panel only (no remote command),
`N` = not available, `?` = unverified (no manual). "Remote" means over the documented interface.

| # | Feature | KA | RD | DPM | SPE | PPS | DP8 | S33 | S4K | GPP | UDP | E36 | TTi |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | V / I setpoint get+set | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| 2 | Measured V / I | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| 3 | Measured power from instrument | N | Y | N | Y | N | Y | Y | Y | Y | Y | N? | N |
| 4 | Output on/off per channel | P1 | Y | Y | Y | P1 | Y | Y | Y | Y | Y | Y | Y |
| 5 | All outputs on/off in one command | P1 | - | - | - | P1 | N | N | Y | Y | Y | Y? | Y |
| 6 | Output state readback | P2 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| 7 | CV / CC status | Y | Y | Y | Y | Y | Y3 | Y | Y | P | Y | Y | P4 |
| 8 | OVP / OCP tripped status | N | Y | N | Y | N | Y | N | Y | Y | Y5 | Y | Y4 |
| 9 | OVP / OCP level | P6 | Y | N | Y7 | N | Y | N | Y | Y | Y | Y | Y |
| 10 | OVP / OCP enable switch | Y | N7 | N | N7 | N | Y | N | P8 | Y | Y | Y | P9 |
| 11 | Trip clear command | N | N | N | N | N | Y | N | Y | N? | N? | Y | Y |
| 12 | OPP (over-power) | N | N | N | N | N | P10 | N | N | P11 | P10 | N | P12 |
| 13 | OTP status / readout | N | P13 | P13 | Y | N | Y | N | N? | Y | Y | Y | F |
| 14 | Series / parallel / tracking | P14 | N | N | N | Y | Y | Y | Y | Y | Y | Y? | Y |
| 15 | Remote sense (4-wire) | N | N | N | N | N | P15 | N | F | N? | N | Y? | F |
| 16 | Memory store / recall (presets) | Y | Y | N | F | N | Y | Y | Y | Y | Y | Y? | Y |
| 17 | List / sequence / timer mode | N | N | N | F | N | Y | Y | Y | Y | Y | Y? | N |
| 18 | Output on/off delay, sequencing | N | N | N | N | N | Y | N | Y | Y | Y | Y? | P16 |
| 19 | Slew / ramp / soft start | N | N | N | N17 | N | N | N | N | P18 | P18 | N? | N |
| 20 | Keylock / remote-local | P19 | Y | N | Y | P20 | Y | Y | Y | Y | Y | Y | Y |
| 21 | Beeper | Y | Y | N | N | N | Y | N | Y | Y | Y | Y? | N |
| 22 | Energy counters Ah / Wh | N | Y21 | N | N | N | N | N | N | N | N | N | N |
| 23 | Statistics (min/max/avg) | N | N | N | N | N | Y22 | N | N | N | N | N | P23 |
| 24 | On-instrument recorder / logger | N | N | N | N | N | Y | P24 | Y | Y | N | ? | N |
| 25 | Monitor: condition -> output off | N | N | N | N | N | Y25 | N | N | Y | Y | N | N |
| 26 | Voltage / current ranges | N | P26 | N | N | N | P26 | N | N | N | N | P26 | Y |
| 27 | Input / battery voltage, charge mode | N | Y27 | N | N | N | N | N | N | N | N | N | N |
| 28 | Max limits per model (identify-derived) | Y | Y | Y28 | Y | Y | Y | Y | Y | Y | Y | Y | Y |

Notes:
1. KA `OUT1/OUT0` and PPS `o1/o0` are **global** for every channel (TENMA p.2 #8; PPS p.1). Same
   on OWON ODP (`OUTP 0|1`, per channel only via the list command `CHAN:OUTP:ALL a,b,c`, ODP PDF p.13-16).
2. KA: from `STATUS?` bit 6 (TENMA p.2 #9); no direct query.
3. DP800 `:OUTP:CVCC?` returns `CV`, `CC` or `UR` (unregulated) (PG800 p.2-53).
4. TTi `LSR<N>?` is a *latched event* register cleared on read: bit0 CV, bit1 CC, bit2 OVP,
   bit3 OCP, bit4 power limit (CPX), bit6 hard trip (CPX PDF p.23, p.29).
5. UDP3305S trips only visible in the latched `:STAT:QUES:INST:ISUM<n>?` (bits 2/3) (PM p.9, p.42-43).
6. KA: OCP level is the ISET value; OVP level is front panel only (coverage §2.1).
7. OWON `VOLT:LIM` / `CURR:LIM` are always armed, no enable (PM p.10-12). Riden OVP/OCP in M0 regs 82/83, always armed.
8. SPD4000X: OVP always armed; `:SOURce:OCP:STATe` enables OCP; `OCP:DELay` exists.
9. MX: `OVP<N> ON|OFF`, `OCP<N> ON|OFF` (MX p.35); CPX always armed.
10. DP800 and UDP3305S have `:MONItor:POWER` conditions (PG800 `:MONItor` subsystem; UDP PM `:MONItor:POWER`), not a classic OPP trip.
11. GPP: OPP exists in its Load mode only (GPP PDF p.506 index line "Overload Protection (OPP in Load mode)").
12. CPX: power-limit regulation (PowerFlex), reported as LSR bit4, not a trip.
13. RD: internal temperature regs 4-7, external probe 34-37 (no trip flag). DPM: `:01r33=` reads device temperature (PDF p.3).
14. KA33xx two-channel models only: `TRACK0/1/2` (coverage §2.1).
15. DP800 `:OUTP:SENS` (model dependent); SPD1000X has `MODE:SET 2W|4W` (PDF p.59). TTi and SPD4000X sense are switches/menu.
16. MX `ONDELAY<N>` / `OFFDELAY<N>` / `ONACTION` for `OPALL` (MX p.37); CPX none.
17. OWON SPE ramps inherently (about 1.5 s to 16 V, coverage §2.3) but the rate is not settable.
18. GPP `:SEQUence:TEMPlet:RISERate/FALLRate`; UDP `:LISTout:TEMPlet:SELect {..|RISE|FALL}`: ramps only as list templates.
19. KA: USB connection locks the panel, no command (KA30 p.15).
20. PPS: `rl` reads the lock, no set (PPS p.2).
21. Riden regs 38-39 Ah, 40-41 Wh (u32, community map).
22. DP800 `:ANALyzer:RESult?` returns groups, median, mode, average, variance, range, min, max, mean deviation (PG800 p.2-6).
23. MX `DAMPING<N>` current-meter averaging (MX p.35), not statistics.
24. SPD3303X `OUTPut:WAVE` waveform display, no data export.
25. DP800 `:MONItor:STOPway {OUTOFF|WARN|BEEPER}` turns the output off when a V/I/P condition is met (PG800 `:MONItor:STOPway`).
26. DP811 `:OUTP:RANG P20V|P40V` (PG800 p.2-62); RD6012P current range reg 20; E3632A `VOLT:RANG`; MX `VRANGE<N>` 1..3 (MX p.36).
27. Riden reg 14 input voltage, regs 32-33 battery mode / battery voltage.
28. DPM: model from `:01r01=0,` max current (PDF p.3).

Extras worth knowing: MX and CPX have `DELTAV/DELTAI` + `INCV/DECV` step commands; DP800/DP700
`:SYST:KLOCk`, `:SYST:OTP`, `:MEMory:*`; SPD4000X `STORage:LIST:FILE:*`, `WAVE:*` (recorder),
`SOUNd:KEY/ALARm`; UDP6720 `OUTPut:TIMer` + `OUTPut:TIMer:DATA` (output-off timer), `OUTPut:POUT`
(power-on output state), `LIST:*`, `DELAyer:*` (UDP6720 PM V1.2).

---

## 2. Tiers

Counts over the 12 columns (Y or P counts as present; F does not, it is not remotely usable).

### Core (12/12 or nearly; the panel must always offer it)
| Feature | Count | Remark |
|---|---|---|
| V / I setpoint get+set | 12/12 | |
| Measured V / I | 12/12 | |
| Output on/off | 12/12 | 2 are global-only (KA, PPS); the panel must say so |
| Output readback | 12/12 | KA via status bit |
| CV / CC | 12/12 | 2 partial (GPP CC only, TTi latched) |
| Max limits per model | 12/12 | from definitions (`absolute_limits`) |
| **Power** | 7/12 measured, **12/12 with V*I** | software closes the gap completely |

### Common (6-11/12; show when bound)
| Feature | Count |
|---|---|
| OVP / OCP level | 9/12 |
| Keylock / remote-local | 11/12 (2 partial) |
| OVP / OCP tripped status | 8/12 |
| Memory store / recall | 9/12 remote (+1 panel only) |
| Series / parallel / tracking | 8/12 (+E36 unverified) |
| OVP / OCP enable switch | 7/12 (2 partial) |
| OTP status / temperature | 7/12 (2 are readouts only) |
| Beeper | 7/12 |
| List / sequence / timer | 6/12 (+E36?) |
| Output on/off delay / sequencing | 5/12 (+1 partial, +E36?) |
| All outputs in one command | 5 multi-channel families of 8 |
| Trip clear | 5/12 |

### Advanced (niche, <= 4/12)
Remote sense (1 remote + 2 panel), on-instrument recorder (4), monitor/condition trip (3), OPP
(0 true trips, 4 partial), ranges (1 full + 3 partial), slew / soft start (2 partial, only as list
templates), statistics (1: DP800), energy Ah/Wh (1: Riden), input/battery voltage (1: Riden).

**Key observation:** the cheap tier (KA, RD, DPM, SPE, PPS) has Core only, plus memories on 2.
Everything that makes a bench PSU pleasant (power, energy, statistics, sequences, timers, soft
start, guards) is Advanced on the instrument side but **cheap and universal on the software
side**. The value BenchMesh adds is largest exactly where the instruments are poorest.

---

## 3. Software-level value

Timing facts that bound every item below:
- Poll intervals in our definitions: 250 ms (Tenma) to 1000 ms (Siglent); slow groups 2-3 s
  (`classes.PSU.poll` across `devices/*.yaml`).
- Setter pacing: Tenma >= 50 ms (`devices/tenma_72.yaml` sources), Siglent SPD3303X >= 100-300 ms
  (coverage §2.4), Korad/Velleman up to 450 ms extra.
- Every set is confirmed by read-back within 6 s (UI `kit/confirm.ts`, automation executor).
- Recordings sample the registry once per whole second (`ai_context/migration/RECORDINGS.md` "Common time domain").
- Readback resolution on the cheap tier: Tenma 10 mV / 1 mA, setup accuracy <= 0.5% + 20 mV /
  0.5% + 5 mV (TENMA 72-2540 user manual spec table).

| Feature | How | Accuracy | Timing needs | Caveats / when not to do it |
|---|---|---|---|---|
| **POUT = V*I** | product of the latest VOUT and IOUT of the same poll group | sum of the relative errors of both readings (Tenma: about 1% + counts); V and I are separate queries tens of ms apart, so a fast-changing load skews it | none beyond the poll | prefer the instrument's own power when it exists (it is computed from simultaneous samples). Mark computed values. Already done ad hoc in 12 of 21 POUT bindings (see §4) |
| **R_load = V/I** | VOUT / IOUT | dominated by IOUT counts; meaningless near 0 A | none | report null when IOUT < max(2 counts, 0.5% of range) or output OFF; label as "apparent" (includes lead resistance unless sensing) |
| **Energy Ah / Wh, on-time** | server-side trapezoid integration of IOUT and POUT over the poll timestamps while OUTPUT = ON; elapsed on-time | good for DC and slowly varying loads (error about the reading error); poor for loads pulsing faster than the poll (aliasing: a 10 ms burst every second is invisible at 250-1000 ms polls) | must run in the server (actor or registry), not in the browser: it must survive closed windows and serve remote sessions; persist across reconnect, reset by an explicit operation | needs real timestamps, not the nominal poll period; stop integrating (gap, not zero) while the instrument is offline and say so; prefer Riden's own Ah/Wh where present |
| **Statistics min / max / avg / count** | rolling accumulators per status key (VOUT, IOUT, POUT) since reset; optionally a window | exact over the polled samples | server-side, same as energy | min/max over polls is not the true peak: inrush and short transients are missed; label "of polled samples". The recordings chart already keeps min/max per bucket |
| **CV/CC inference** | when the instrument does not report mode (OWON ODP, P4000; TTi LSR is latched): CC if OUTPUT ON and IOUT >= Iset - tol and VOUT < Vset - tol; CV if abs(VOUT - Vset) <= tol; else UR/unknown | tol = max(2 readback counts, 0.5% of setpoint) works for typical PSUs | needs the current setpoints (cached GETs or last confirmed set) | mark as inferred (`~CC`); never drive safety decisions from it; wrong when the setpoint was changed on the front panel since the last read (refresh setpoints in a slow poll group) |
| **Software OVP/OCP/OPP guard** ("trip and switch off") | server watches VOUT/IOUT/POUT against user thresholds; on violation sends `output OFF` (all channels of that instrument if output is global) and raises a latched SW-TRIP | detection limited by reading accuracy | reaction time = up to one poll interval + transport + setter gap: realistically 0.3-1.5 s, worse during a busy poll cycle or a hung serial port | **can never replace hardware protection**: hardware OCP/OVP acts in microseconds to milliseconds, a DUT can be destroyed within one poll; if the link drops the guard is blind (fail-unsafe). Offer it only as a secondary, clearly labelled "Software trip (slow, up to N s)"; set the hardware limit too where the instrument has one; raise an alarm when the guarded instrument goes offline. Do not enable automatically. DP800/GPP/UDP have a native `:MONItor` that does the same inside the instrument: prefer it when bound |
| **Software presets** | store {V, I, OVP, OCP, (tracking)} per instrument in `<data dir>` (e.g. a `presets` section next to flows); recall = confirmed sets in a safe order (output stays as is; when lowering V/I set them before raising limits) | exact (setpoint resolution) | sequential sets: 4 sets x 50-300 ms + read-back | never switch the output on as part of recall; if the output is ON warn before recalling (step change at the DUT). Expose hardware memories (KA 5, RD 10, GPP 10, UDP 10, TTi 50) as well |
| **Output-off timer** | server timer: output OFF after T or at a clock time; shows countdown | +- one setter gap (sub-second) | server-side (keeps working with the UI closed) | safety-adjacent but benign (it only switches off). Native timers exist on UDP6720 (`OUTPut:TIMer`) |
| **Software list / sequence, ramp, soft start** | automation flows already support set / wait / repeat / sweep (`ai_context/AUTOMATION.md`); add flow templates "ramp V from a to b in t", "soft start" and a "step list" table | setpoint resolution | step period >= setter gap + read-back (practically >= 0.2-0.5 s per step; every set is confirmed within 6 s) | coarse: a software ramp is a staircase of a few steps per second, not a slew rate; it cannot do inrush limiting faster than that; it runs through the one-run-at-a-time executor and needs `arm` when energising |
| **Tracking emulation** (across channels or instruments) | when CH1 (master) setpoint is confirmed, set CH2 = ratio x CH1 (or sum/difference); across instruments the same | setpoint resolution | one extra setter cycle: channels change hundreds of ms apart | not an electrical series/parallel connection (no current sharing, no common regulation); transient mismatch during each change; do not emulate *parallel* (needs hardware balancing). Native tracking exists on 8 families: prefer it |
| **All outputs off / panic** | one action that switches every output of every PSU off (native `OPALL`, `OUTP:ALL`, `ALLOUTOFF` where bound, otherwise per channel) | - | parallel per instrument | the automation executor already switches off what it switched on; expose it in the UI too |
| **Efficiency / loss** (with DMM or ELL) | eta = P_load (ELL POUT or DMM V x DMM I) / P_in (PSU POUT); loss = P_in - P_load | product of the instruments' errors; samples aligned on the recordings' whole-second sampler | cross-instrument, server-side | steady state only; label the instrument pair; the PSU's own power includes lead losses |
| Ripple estimate | **do not**: polled readings (several Hz, internally averaged) cannot see ripple | - | - | point the user to an OSC panel (AC coupled Vpp) instead |
| Headroom bargraph (IOUT / Iset) | exists in `PsuPanel.tsx` | - | - | keep |

---

## 4. Gap analysis

### 4.1 Contract (`devices/classes/PSU.yaml`)
- Status: VOUT, IOUT, POUT, OUTPUT, REMOTE required; CV, CC, SBY, FAIL, OVP, OCP, OTP optional.
- Operations: voltage, current (required get+set), output (required set), remote, voltage_limit,
  current_limit, ovp, ocp, output_voltage, output_current, output_power.
- **Missing vs Core/Common:** output scope (global vs per channel), all-outputs, `UR`
  (unregulated), trip clear, OVP/OCP enable readback convention, tracking mode, memory
  save/recall (although `features.memory_banks` is declared by 9 definitions), beeper,
  temperature, sequence/timer/delay, ranges, sense, energy/statistics keys.
- POUT is `required` and described as "measured or V*I", so every definition must compute it
  itself; there is no notion of provenance (measured vs computed).

### 4.2 Definitions (25 families bind PSU per `bm-devicedef export devices`, 21 files match `^  PSU:`, plus riden_rd6006, riden_rd6006p, riden_rd6012 and manson_hcs that inherit the binding from their bases)

| Operation | Bound by | Status key | Bound by |
|---|---|---|---|
| voltage, current, output (set), output_voltage, output_current | 25/25 | VOUT, IOUT, POUT, OUTPUT, REMOTE, SBY | 25/25 |
| output **get** | 12/25 | CV, CC | 21/25 (missing: owon_odp, owon_p4000, tti_cpx, tti_mx) |
| voltage_limit, current_limit | 18/25 | OVP, OCP, FAIL | 10/25 |
| remote | 14/25 | OTP | 2/25 (owon_spe, owon_spm) |
| output_power | 11/25 | | |
| ocp | 9/25 | | |
| ovp | 8/25 | | |

POUT source: measured by the instrument in 9 of the 21 POUT bindings (gwinstek_gpp, owon_p4000, owon_psu, owon_spm,
siglent_spd1000x, unit_udp3000s, unit_udp6700, riden_rd60xx, rigol_dp) and **computed in CEL as
V*I in 12** (tenma_72, korad_ka, juntek_dpm86xx, hantek_pps, owon_odp, siglent_spd3303x,
siglent_spd4000x, tti_psu, gwinstek_gpd, scpi_psu, both manson_hcs). So the user's Tenma example
is already solved, but per definition by copy-paste, invisible to the user, and SPD4000X computes
V*I although `MEAS:POWER?` exists (manual PDF p.78).

Features declared in `classes.PSU.features` (29 files incl. bases): absolute_limits 19, lock 18,
compulsory_lock 12, memory_banks 9, protection 9, memory_storage 6, modes 5, auto_lock 3,
channel_limits 3. The UI reads only absolute_limits, modes and (elsewhere) compulsory_lock;
`memory_banks` has no operation behind it.

**Manual commands we do not bind** (none of these is in any PSU definition; grep of command names):
- Trip clear: DP800/DP700 `:OUTP:OVP|OCP:CLEAR`, TTi `TRIPRST`, SPD4000X `:SOURce:RESET:PROTect`,
  UDP6700 `VOLT:PROT:CLE`, SPD1000X `OUTP:RESE:PROT`, E36300 `VOLT:PROT:CLE`.
- Tracking/series/parallel: KA33xx `TRACK0/1/2`, GPD `TRACK0/1/2`, SPD3303X/SPD4000X `OUTP:TRACK`,
  DP800 `:OUTP:TRAC`, UDP `:SOUR:MODE NORM|SER|PARA`, GPP `:OUTP:SER/PAR`, ODP `OUTP:SER/PAR/TRAC`,
  PPS `o2..o5`, TTi `CONFIG`.
- Memories: KA `SAV/RCL 1..5`, Riden preset regs 80-119 and reg 19, DP800 `*SAV/*RCL`, `:MEMory:*`,
  SPD3303X `*SAV/*RCL 1..5`, GPP `*SAV/*RCL 0..9`, UDP `*SAV/*RCL 1..10`, TTi `*SAV/*RCL`, `SAV<N>/RCL<N> 0..49`.
- All outputs: TTi `OPALL`, SPD4000X `OUTP:ALL`, GPP `:ALLOUTON/:ALLOUTOFF`, UDP `:OUTP:STAT ALL`.
- Extra readings: Riden temperature, input voltage, Ah/Wh, battery mode; Juntek temperature;
  DP800 `UR` is parsed (`cvcc` map) but not surfaced as a status key.
- OTP/temperature trips: DP800 `:SYST:OTP`, UDP `:STAT:QUES?` bit4, E36300 ISUM bit4.
- Sequencing/timers: DP800 `:TIMEr`, `:DELAY`, `:OUTP:TIMEr`; SPD3303X/SPD1000X `TIMEr:SET`;
  SPD4000X `LIST:*`, `OUTPut:ON|OFF:DELay`; GPP `:SEQUence`, `:DELAy`; UDP `:LISTout`, `:DELAY`;
  UDP6720 `OUTPut:TIMer`; TTi `ONDELAY/OFFDELAY`.
- Monitor/statistics: DP800 `:MONItor:*`, `:ANALyzer:*`; GPP/UDP `:MONItor:*`.
- Beeper/keylock: KA `BEEP0/1`, Riden reg 69, DP800 `:SYST:BEEP`, `:SYST:KLOCk`, SPD4000X `SOUNd:*`, GPP `:SYST:BEEPer`, UDP `:SYST:BEEPer`, `:SYST:KLOCk`.
- Ranges and sense: DP811 `:OUTP:RANG`, TTi `VRANGE`, SPD1000X `MODE:SET 2W|4W`, DP800 `:OUTP:SENS`.

### 4.3 UI (`PsuPanel.tsx`, `psuProfile()`)
Shows VOUT, IOUT, POUT (falls back to V*I in the browser if POUT missing), annunciators CV, CC,
OVP, OCP, OTP, FAIL, OUT, REM; setpoint editors for V, I, OVP level, OCP level; headroom bargraph;
arm switch for output; micro switches for ovp/ocp (state only remembered locally: "the instrument
does not report this setting") and remote. Sparkline history of V and I (150 points, browser only).

Missing vs Core/Common:
- No indication that output is **global** (KA/PPS/ODP): switching CH1 switches all channels.
- No "all outputs off" on multi-channel panels.
- No trip-clear button when OVP/OCP/FAIL is lit; latched trips cannot be acknowledged.
- No memories (hardware or software), no tracking selector, no beeper.
- No energy, statistics, R_load; POUT gives no hint whether it is measured or computed.
- OVP/OCP enable readback ignored even where the instrument can report it (DP800 `:OUTP:OVP? CH1`).
- Sparkline history lives in the browser and is lost on reload; statistics must not be built on it.

### 4.4 DSL capabilities missing for the software layer
1. **Stateful derived status.** Status CEL is evaluated over the latest poll results, `state` and
   `ch` (`DEVICE_DSL.md` "CEL environment"); `state` changes only through command `effects`.
   There is no previous value, no timestamp delta, so **an integrator (Ah/Wh), min/max/avg or
   on-time cannot be written in a definition**. It belongs in the server (per-device actor or a
   registry post-processor), not in the DSL.
2. **Class-level defaults.** No way for the class contract to say "POUT = VOUT*IOUT when the
   definition does not bind it"; each definition repeats it.
3. **Provenance.** Status values carry no `source: instrument|computed|inferred` marker.
4. **Instrument-wide operations.** Operations are routed per `{ch}`; global output, all-outputs,
   tracking and memories are per instrument. A convention (channel 0 or an `scope: instrument`
   flag in the contract) is needed so the UI does not render them per channel.
5. **Momentary actions** (trip clear, recall slot N, counters reset) fit `set` with an enum or
   integer value; the kit already has an action button confirmed by a reported change, which
   needs a status key to watch (e.g. FAIL going false after clear).

---

## 5. Proposal

### 5.1 Contract additions (`devices/classes/PSU.yaml`)

Status (all optional unless stated):
| Key | Type | Meaning / source |
|---|---|---|
| `POUT` | si, required (keep) | definition binds it **or the server derives it** as VOUT*IOUT; the registry adds provenance |
| `UR` | bool | unregulated (DP800 `UR`, CPX power limit LSR bit4) |
| `MODE` | enum `CV, CC, UR, OFF` | canonical mode; CV/CC/SBY stay for compatibility of annunciators |
| `TEMP` | si (degC) | Riden reg 5, Juntek fn 33 |
| `VIN` | si (V) | Riden reg 14 (battery/adapter supply) |
| `AH`, `WH` | si | instrument counters (Riden 38-41); otherwise server-derived |
| `TRACK` | enum `INDEP, SERIES, PARALLEL, TRACK` | instrument-wide tracking state where readable (GPD/KA `STATUS?` bits, TTi `CONFIG?`, ODP queries) |
| `OVP_EN`, `OCP_EN` | enum `ON, OFF` | enable readback where queryable (DP800, GPP, UDP, MX `OVP<N>?` -> `OFF`) |

Server-derived keys (computed by bm-server for every PSU, never by definitions; provenance `computed`):
`POUT` (when unbound), `RLOAD`, `AH`, `WH`, `ON_TIME` (s), `VOUT_MIN/MAX/AVG`, `IOUT_MIN/MAX/AVG`,
`POUT_MAX` (or one `STATS` map), `MODE` inferred when CV/CC are unbound (provenance `inferred`),
`SW_TRIP` (bool, latched software guard trip).

Operations:
| Op | Direction / value | Binds to |
|---|---|---|
| `output_all` | set enum ON/OFF, instrument scope, energizes | TTi OPALL, SPD4000X OUTP:ALL, GPP ALLOUT*, UDP OUTP:STAT ALL; server falls back to per channel |
| `protection_clear` | set enum `CLEAR` (momentary) | DP800/DP700, TTi TRIPRST, SPD4000X, SPD1000X, UDP6700, E36300 |
| `ovp`, `ocp` | add `get` | as above |
| `tracking` | get/set enum `INDEP, SERIES, PARALLEL, TRACK` (values per definition) | 8 families |
| `memory_save`, `memory_recall` | set integer slot (min/max from definition), instrument scope; recall `energizes` when the slot stores output state (TTi `*RCL`) | KA, RD, DP800, SPD3303X, GPP, UDP, TTi |
| `beeper` | set enum ON/OFF | KA, RD, DP800, SPD4000X, GPP, UDP |
| `range` | get/set enum (per definition) | DP811, MX, RD6012P, E3632A |
| `sense` | get/set enum `2W, 4W` | SPD1000X, DP800 (model dependent) |
| `counters_reset` | server operation (not in definitions): resets AH/WH/ON_TIME/statistics | all |

Features: `output_scope: channel | instrument` (default channel; KA, PPS, ODP = instrument),
`memory_banks` (existing) tied to the slot range of `memory_save/recall`, `tracking_modes`,
`protection.OVP.always_armed` / `protection.OCP.always_armed` (OWON, Riden, CPX, SPD4000X OVP).

Do **not** put lists/sequences/timers/delays into the class contract yet: 6 families, 6
incompatible models (groups x cycles x end state, templates, files). Cover them in software
(flows) first, and later as an Advanced extension if users ask.

### 5.2 Server software layer (new module, single responsibility)
`crates/bm-server/src/derive/` (or in the per-device actor in `bm-driver`): a class-keyed
post-processor fed by every registry update of a PSU channel, with real timestamps:
- stateless: POUT fallback, RLOAD, MODE inference;
- stateful: trapezoid integrators (AH, WH while OUTPUT = ON), ON_TIME, min/max/avg since reset;
  gaps (offline, poll failures) break the integration instead of bridging it;
- guard: user thresholds per channel, `SW_TRIP` latched, action `output OFF` through the normal
  operation path (confirmed, logged in the wire console), alarm when the guarded device is offline;
- timer: output-off at T, implemented with the same path;
- presets: stored in `<data dir>` (one file, 0600 not needed), REST under
  `/instruments/PSU/{id}/presets`; recall = confirmed sets, never output ON.
All of it is available to MCP, flows and remote sessions because it lives in the server.
Simulation (`--simulate`) keeps it in memory like recordings.

### 5.3 Panel UX sketch (one panel for every PSU)
```
+-- PSU  Tenma 72-2540 ----------------------------------- [fold][window][console] -+
|  VOUT 12.003 V   IOUT 0.512 A   POUT 6.15 W (calc)   R 23.4 ohm                  |
|  [CV] CC  OVP OCP OTP FAIL  SW-TRIP   OUT  REM                                     |
|  Energy 0.153 Ah  1.84 Wh  00:17:42 on   min/max I 0.498 / 0.611 A   [reset]       |
|  Vset [12.00] V   Iset [0.600] A   [=====    ] headroom                            |
|  OVP [13.00] V  OCP [0.700] A   OVP(o) OCP(o)   [Clear trip]   (shown when bound)  |
|  Preset [M1..M5 | BenchMesh: "5V logic", "12V fan" v] [Recall] [Save]              |
|  Output (arm)   [All off]   Tracking [INDEP v]   Timer [off v]   Guard [SW 1 s v]  |
+------------------------------------------------------------------------------------+
```
- `(calc)` / `(inferred)` tags from provenance; `Output (global)` label when `output_scope: instrument`.
- Guard dialog states plainly: "Software trip reacts in up to N s (poll interval + command). It
  does not replace the instrument's OVP/OCP." with N computed from the definition's poll and gaps.
- Instrument-scope controls (all off, tracking, memories, beeper) render once per instrument, not
  per channel, in multi-channel panels.
- Ramp / soft start / step list: a small "Sequence" button that opens a prefilled automation
  flow (existing editor and executor, arm dialog) instead of a second sequencer.

### 5.4 Suggested order
1. Server-derived POUT fallback + provenance; remove copy-pasted `vout * iout` from definitions
   (or keep them and let provenance mark them computed); SPD4000X switch to `MEAS:POWER?`.
2. Energy/statistics/on-time + `counters_reset` (highest user value, every PSU).
3. `output_scope` feature + `output_all` + `protection_clear` + OVP/OCP enable `get` (small contract steps, bind in the 6-10 families listed).
4. Memories (hardware ops + software presets), tracking.
5. Software guard and output timer (with the safety wording above).
6. Flow templates for ramp / soft start / step list.
