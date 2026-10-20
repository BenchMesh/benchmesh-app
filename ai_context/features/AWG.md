# AWG (function / arbitrary waveform generator): common functional denominator

Research 2026-10-20. Inputs: contract `devices/classes/AWG.yaml`, 24 definitions that bind `AWG`
(21 with their own `classes.AWG` block + `gwinstek_afg2000`, `gwinstek_afg3000`, `owon_xdg` via
`extends`), bases `devices/bases/{scpi_awg,siglent_sdg,tek_afg,gw_afg,unit_utg}.yaml`, coverage report
`ai_context/migration/coverage/AWG.md`, device notes `ai_context/migration/device-notes/owon_dge.md`,
UI `frontend/src/ui/bench/panels/AwgPanel.tsx` + `awgProfile()` in `frontend/src/ui/bench/profiles.ts`,
and the manuals under `/home/marek/project/BenchMeshDOC/<family>/doc/`.

**Page convention:** `pN` = PDF page of the file (as in the definitions' `source:`), "printed" when
the manual's own page number is meant. Abbreviations of the documents:

| Key | Document (BenchMeshDOC) |
|---|---|
| DGE-PM | `owon_dge/doc/DGE2000_3000_SCPI_Protocol.pdf` |
| DGE-UM | `owon_dge/doc/DGE2000_Arbitrary_Waveform_Generator_User_Manual.pdf` |
| FY-P | `feeltech_fy6900/doc/FY6900_Serial_communication_protocol.pdf` |
| JDS-P / JDS-UM | `juntek_jds6600/doc/JT-JDS6600-Communication-protocol_2024-04-23.pdf` / `..._Manual-EN_2025-01-16.pdf` |
| U9E | `unit_utg900e/doc/UTG900E_Programming_Manual_V1.0.pdf` |
| U1X | `unit_utg1000x/doc/Uni-T_UTG1000X_Programming_Manual_0424.pdf` |
| AFG1 | `tektronix_afg1000/doc/AFG1000-Programmer-Manual-EN-077112902.pdf` (PDF = printed + 9; OWON XDG is the same command set) |
| DG1Z | `rigol_dg1000z/doc/DG1000Z_ProgrammingGuide_EN.pdf` |
| SDG | `siglent_sdg/doc/SDG_Programming_Guide_PG02-E05C.pdf` (covers SDG1000X, 2000X, 6000X; availability tables per model) |
| GW | `gwinstek_afg2225/doc/AFG-2225_User_Manual_farnell_2344129.pdf` (command index p188-192) |
| HDG | `hantek_hdg2000/doc/HDG2000B_SCPI_protocol_EN.pdf` |
| EDU | `keysight_edu33210/doc/EDU33210_Users_Guide_EDU33212-90002_Ed4.pdf` |
| 335B | no local manual (login wall, see coverage §2.7); entries from the public Keysight 33500B/33600A command reference, marked `(k)` = from knowledge, verify before binding |

---

## 1. Feature matrix (remote control)

Columns, cheapest to pro: **FY** FeelTech FY6900, **JDS** Juntek JDS6600, **DGE** OWON DGE2000
(the user's DGE2070), **U9E** UNI-T UTG900E, **U1X** UNI-T UTG1000X, **AFG1** Tek AFG1000 / OWON XDG,
**DG1Z** Rigol DG1000Z, **SD1X** Siglent SDG1000X, **GW** GW Instek AFG-2225, **HDG** Hantek HDG2000B,
**EDU** Keysight EDU33210, **335B** Keysight 33500B, **SD6X** Siglent SDG6000X.

`Y` = remotely controllable, `fp` = front panel only (no remote command), `p` = partial, `N` = absent,
`?` = not found in the local documents. `hw-N` = command documented but the hardware lacks the
connector (DGE2000 rear panel: power + USB device only, DGE-UM p10).

| Feature | FY | JDS | DGE | U9E | U1X | AFG1 | DG1Z | SD1X | GW | HDG | EDU | 335B | SD6X | Sources (examples) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Sine, square, ramp | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p26; FY-P p6; JDS-P p3; SDG p29 WVTP |
| Pulse | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p26 PULSe; JDS-P p3 "2 Pulse" |
| Noise | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p26 PRNoise |
| DC | Y | Y | fp | Y | Y | ? | Y | Y | N | Y | Y | Y | Y | DGE-UM p23 (DC is a built-in arb, no SCPI token); SDG p29 |
| Built-in arb select | Y | Y | fp | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-UM p23; U9E p18 ARB:INDex; AFG1 p52 |
| Arb upload | N | ? | p | N | N | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p38 (format undocumented); DG1Z p186-191; GW p191 DATA:DAC |
| Frequency entry | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | all |
| Period entry | N | N | fp | Y | Y | N | Y | Y | p | N | p | p(k) | Y | DGE-UM p15; U9E p12 BASE:PERiod; DG1Z p123 PERiod; SDG p29 PERI; GW/EDU pulse/square period only |
| Vrms / dBm units | N | N | N | Y | Y | N | Y | Y | Y | N | Y | Y(k) | Y | DGE-PM p35 (mVpp, Vpp only); U9E p11; DG1Z p199; SDG p30 AMPVRMS/AMPDBM; EDU p88 |
| High / low level | N | N | fp | Y | Y | N | Y | Y | N | N | Y | Y(k) | Y | DGE-UM p16; U9E p13-14; DG1Z p195-196; SDG p30 HLEV/LLEV; EDU p86-87 |
| Offset | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | all |
| Phase | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p27 (radians default) |
| Square duty | Y | Y | N | Y | Y | N | Y | Y | Y | Y | Y | Y | Y | FY-P p9 WMD; U9E p14; DG1Z p122; SDG p30 DUTY; coverage §2.6 (Tek dialect: pulse only) |
| Ramp symmetry | N | p | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p26 |
| Pulse width / duty | p | p | Y | p | p | p | Y | Y | p | Y | Y | Y | Y | DGE-PM p30-32 DCYC/WIDTh; SDG p30 WIDTH; FY-P p9 WMS (pulse period) |
| Pulse edge times | N | N | Y | Y | Y | N | Y | Y | N | Y | Y | Y | Y | DGE-PM p31-32 TRANsition; U9E p15 RISe/FALL; SDG p30 RISE/FALL |
| Load 50 ohm / HiZ | N | N | Y | Y | Y | Y | Y | Y | Y | N | Y | Y | Y | DGE-PM p13 OUTP:IMP (1 ohm-10 kohm, INF); U9E p11; SDG p28 (50, HiZ, 50-100k); coverage §2.10 (HDG none) |
| Output on/off | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | all |
| Output limit (max V) | N | N | N | Y | Y | N | N | Y | N | N | ? | Y(k) | Y | U9E p10 LIMit:ENABle/LOWer/UPPer; SDG p27 MAX_OUTPUT_AMP; 335B VOLT:LIM (k) |
| Sync output | ? | ? | hw-N | Y | Y | ? | Y | Y | ? | Y | Y | Y | Y | U9E p9 OUTPut:SYNC; DG1Z p71-73; SDG p66 SYNC; HDG p26 |
| Ch coupling / tracking / copy | Y | Y | N | p | ? | p | Y | Y | Y | p | Y | Y | Y | FY-P p31-33 USA/USD; JDS-UM p6 sync; U9E p6 phase sync only; AFG1 p23 CSCopy; DG1Z p22-34 COUPling, p192 TRACK; SDG p61 PACP, p96 COUP; GW p191; EDU p150-151 |
| AM | Y | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | FY-P p20 WPF4; DGE-PM p15-17; U9E p15 MODe; DG1Z p130-134; SDG p33 MDWV |
| FM | Y | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p20-23 |
| PM | Y | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p28-30 |
| FSK | Y | N | fp | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-UM p36-37 (no SCPI); U9E p19; AFG1 p49-51 |
| ASK | Y | N | N | N | Y | Y | Y | Y | N | Y | N | N | Y | FY-P p20 WPF0; U1X p18; DG1Z p135-138 |
| PSK / BPSK | Y | N | N | N | Y | Y | Y | Y | N | Y | Y | Y | Y | DG1Z p154-157; EDU p77 BPSK |
| PWM | N | N | N | N | Y | Y | Y | Y | N | Y | Y | Y | Y | U1X p21; AFG1 p63-67; DG1Z p158-163 |
| SUM / DSB-AM | N | N | N | N | N | N | Y | Y | Y | N | Y | Y | Y | DG1Z p172-174; GW p190; EDU p119-123; SDG p33 DSBAM |
| Mod source ext | Y | N | hw-N | Y | Y | Y | Y | Y | Y | Y | ? | Y | Y | FY-P p21 WPM1; DGE-UM p47 "source internal" |
| Mod shape / freq / depth / dev | p | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p15-30; FY modulates from CH2 (WPM0, FY-P p21) |
| Freq sweep lin / log | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | FY-P p30 SMO; JDS-P p9; DGE-PM p33 SPACing; U9E p15 Line/Log; SDG p37 SWMD |
| Sweep start/stop/time | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p25 STARt/STOP, p34 TIME (1 ms-500 s); U9E p20 |
| Sweep center/span | N | N | Y | N | N | Y | Y | Y | Y | Y | ? | Y | Y | DGE-PM p23-24 |
| Sweep hold / return time | N | N | N | N | N | N | Y | Y | N | Y | Y | Y | Y | DG1Z p175-177 HTIM/RTIM; SDG p37 STARTTIME/ENDTIME/BACKTIME; HDG p31 |
| Sweep trigger int/ext/man | p | ? | Y | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p33 SWE:SOUR TIM/MAN/EXT (hw: int, man; DGE-UM p48) |
| Ampl / offset / duty sweep | Y | Y | N | N | N | N | N | Y | N | N | N | N | Y | FY-P p27-28 SOB0-3; JDS-UM p4-5; SDG p25-46 (AMODe, ASNumber) |
| List / step sweep | N | N | N | N | N | N | Y | Y | N | N | Y | Y(k) | Y | DG1Z p180 SWE:STEP; SDG p37 SWMD STEP; EDU p133 LIST |
| Burst N-cycle | Y | Y | Y | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | FY-P p22 WPF3 + WPN; JDS-P p10; DGE-PM p19 NCYCles; U1X p24-25 |
| Burst gated / infinite | N | p | hw-N | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | DGE-PM p17-18 GATE (no connector on DGE2000) |
| Burst period / phase | N | N | Y | N | Y | p | Y | Y | Y | Y | Y | Y | Y | DGE-PM p18 INTernal:PERiod; DG1Z p95-97 |
| Remote trigger (*TRG etc.) | Y | Y | N | N | Y | Y | Y | Y | Y | Y | Y | Y | Y | FY-P p23 WPO; AFG1 p75 *TRG; DG1Z p48; SDG MTRIG; DGE-PM has no *TRG/TRIG command |
| Frequency counter input | Y | Y | hw-N | Y | Y | N | Y | Y | Y | Y | N | N(k) | Y | FY-P p25-27 RCF..; JDS-P p12; DGE-PM p7-9 (DGE2000 has no input); U9E p7-8 CYMometer; SDG p74-75 FCNT |
| Harmonic generator | N | N | N | N | N | N | Y | Y | N | Y | N | N | Y | DG1Z p123-127; SDG p142-143 (HARM no on SDG1000 old, yes 1000X/2000X/6000X) |
| Store / recall setups | Y | Y | N | N | ? | Y | Y | p | Y | Y | Y | Y | p | FY-P p31 USN/ULN; JDS-P p12; AFG1 p28-29 *SAV/*RCL; HDG p51 |

**Counts over the 13 families (Y only; fp, p and hw-N not counted):**
sine/square/ramp/pulse/noise/frequency/offset/phase/output 13 · freq sweep lin/log + start/stop/time
13 · AM, FM, PM 12 · burst N-cycle 12 · FSK 11 (+ DGE fp) · load 11 · built-in arb 12 · symmetry 11 ·
square duty 11 · remote trigger 11 · edge times 9 · Vrms/dBm 9 · high/low 8 · counter 9 · coupling 8 ·
sweep trigger 11 · gated burst 10 · PSK 9 · ASK 8 · PWM 8 · period entry 6 (+4 p) · sweep hold/return
6 · output limit 5 · harmonics 4 · ampl/offset sweep 4 (FeelTech, JDS, Siglent).

Even the cheapest units (FY6900 about EUR 60-150, JDS6600 EUR 70-120, DGE2000 EUR 200) have sweep and
burst; three of the four cheapest have AM/FM/PM. The user's assumption holds.

---

## 2. Tiers

### Core: the panel must offer it (12-13 of 13, including every cheap unit)
- Output on/off (energize), waveform SIN SQU RAMP PUL PRN (+ DC and ARB where the instrument has them).
- Frequency (Hz) **with period entry** (period is software, §3.1; only 6 families have it natively).
- Amplitude (Vpp), offset (V), phase (deg).
- **Frequency sweep**: on/off, start, stop, sweep time, LIN/LOG (13/13).
- **Modulation AM / FM / PM**: on/off, internal shape, modulation frequency, depth (AM) / deviation
  (FM Hz, PM deg) (12/13; JDS6600 is the only one without).
- **Burst N-cycle**: on/off, cycles, burst period, trigger source INT / MAN (12/13; UTG900E lacks it).
- **Derived readouts and alternative entry** that software computes for every instrument (§3):
  period, Vrms/dBm (for SIN/SQU/RAMP/PUL), high/low level, pulse width from duty.

### Common: shown when the definition binds it (8-11 of 13)
- Square duty (11), ramp symmetry (11, already bound in 23 definitions), load 50 ohm / HiZ (11,
  status already in 22 definitions), pulse width + edge times (9), FSK (11: hop frequency + rate),
  sweep trigger source INT/EXT/MAN + a **Trigger** action (11), gated / infinite burst (10), native
  Vrms/dBm unit (9), native high/low (8), external modulation source (10), channel coupling /
  tracking / copy (8), frequency counter (9; but as its own readout, see §5.6), center/span (8).

### Advanced: niche or out of scope for now (4-9 of 13 or heavy data)
- ASK (8), PSK/BPSK (9), PWM (8), SUM/DSB-AM (6): bindable later with the same `MOD_*` vocabulary.
- Sweep hold/return times (6), list/step sweep (5), amplitude/offset/duty sweep (4).
- Harmonics (4), noise-add, PRBS, IQ, differential output, sequences, multi-device sync.
- Arbitrary waveform upload (needs binary block writes, per-vendor sample formats; DGE format undocumented).
- Native output limit (5): useful, but the software guard (§3.7) covers every instrument.
- Store / recall setups: BenchMesh can store setups itself (flows / a saved-setup document).

---

## 3. Software-level value (BenchMesh computes or emulates)

Rule of thumb: **conversions** (pure arithmetic on read-back values) are always safe and exact up to
the instrument's own accuracy; **emulated time behaviour** (stepping, tracking) is only acceptable
where seconds-level timing and non phase-continuous changes are fine; **signal processing** that
needs microsecond timing (modulation, burst, fast sweep) must never be emulated.

### 3.1 Period <-> frequency
- `T = 1/f`. UI toggle Hz / s on the frequency editor; the setter always sends `frequency`, the read-back
  `FREQ` is converted for display and for confirmation (confirm against `1/FREQ` within the editor's
  resolution, not an exact string match).
- Accuracy: exact. At 1 uHz frequency resolution a period entry rounds to the nearest representable
  frequency; display the period from the read-back so the user sees what was really set.
- No need to bind native `PERI` (Siglent) or `PERiod` (Rigol, UNI-T): one path for all instruments.
- Pulse period on Rigol (`FUNC:PULS:PER`) and FeelTech (`WMS`) is the same quantity.

### 3.2 High / low level <-> amplitude / offset
- `Vpp = H - L`, `offs = (H + L) / 2`; display H/L for every instrument from `AMPL` and `OFFS`.
- Entry needs two setters when the instrument has no native `HLEV/LLEV` (FY, JDS, DGE, AFG1, GW, HDG).
  The intermediate state must stay inside the instrument envelope `|offs| + Vpp/2 <= Vmax(load, f)`
  (DGE-UM p46: +-10 Vpk HiZ, +-5 Vpk 50 ohm below 10 MHz). Safe order: set amplitude to `min(old, new)`,
  then offset, then the final amplitude. Instruments clamp silently, so confirmation by read-back is
  essential (the confirm red dot shows a clamp).
- Prefer native `HLEV/LLEV` (Siglent, Rigol, UNI-T, Keysight) when bound: atomic, no transient.
- Not meaningful for noise (Siglent refuses HLEV/LLEV for NOISE, SDG p30) or DC.

### 3.3 Vpp <-> Vrms <-> dBm (load- and waveform-aware)
- AC rms from peak-to-peak (offset excluded, as instruments define it):
  sine `Vpp / (2*sqrt(2))`; square 50 % `Vpp / 2`; triangle / ramp (any symmetry) `Vpp / (2*sqrt(3))`;
  pulse / square with duty D `Vpp * sqrt(D * (1 - D))`.
- `dBm = 10 * log10(Vrms^2 / R / 1 mW)` with R = the configured load. Undefined at HiZ: Keysight
  refuses dBm with HiZ termination ("Amplitude units cannot be dBm if output termination set to
  high impedance", EDU p88), so the UI must hide dBm when `LOAD` is `HIGHZ` or unknown.
- Must NOT be emulated for PRN (noise Vpp is a vendor convention), ARB / built-ins (rms depends on the
  samples) and DC; for PUL only when the duty is read back.
- Accuracy: the instrument's amplitude accuracy (DGE: +-(1 % + 1 mVpp), DGE-UM p46). It is the
  voltage **at a load equal to the LOAD setting**; a different real load changes it (§3.5).
- When the instrument itself is switched to Vrms/dBm on its front panel, `AMPL` is null today
  (`scpi_awg` binds `AMPL` only when `unit_q == 'VPP'`): the panel should say "instrument shows Vrms/dBm"
  instead of a blank, and offer "switch to Vpp" (an explicit setter, never a hidden one).

### 3.4 Duty <-> pulse width, edge limits
- `width = D * T`. Show width for PUL (and SQU) on every instrument; accept width entry and send duty
  where only duty exists (Tek-AFG/DGE `PULS:DCYC`, FeelTech `WMD`, UNI-T `BASE:DUTY`).
- Valid duty range from the minimum width (DGE-PM p30): `100 % * wmin / T <= D <= 100 % * (1 - 2 * wmin / T)`
  (DGE2070 wmin 24 ns, DGE-UM p47). Square duty limits depend on frequency (Rigol 20-80 % below 10 MHz,
  coverage §2.1). Put `wmin` and duty tables in `features`; the instrument clamps anyway.
- Caveat: on a frequency change some instruments hold width, others hold duty (Rigol `PULS:HOLD`,
  DGE "when any of them is changed, the other will be modified", DGE-PM p30): always re-read both.

### 3.5 Load awareness and DUT voltage
- Display: amplitude/offset are "into LOAD". With a user-entered real DUT resistance R the open-circuit
  and DUT voltages follow from the 50 ohm source: `V_dut = V_open * R / (R + 50)`, `V_open = 2 * V(50 ohm)`.
- Emulated 50 ohm / HiZ toggle for instruments without a load command (FY6900, JDS6600, HDG2000B): pure
  UI scaling (x2 / x0.5) of the setpoints, clearly labelled "assumed load". Limits halve at 50 ohm.
- Where the instrument has the load command (11/13) bind it instead (`load` operation, §5).

### 3.6 Software (stepped) sweep and lists
- Step a setter (frequency, amplitude, offset, duty) over lin/log/list points with a dwell. Already
  possible as an automation flow (`sweep` step, `ai_context/AUTOMATION.md`), every set confirmed by
  read-back within 6 s.
- Timing: DGE exchanges take 15-110 ms (device notes, reply latency), setters need a 20 ms gap and must
  not be read immediately (desync, device notes "Setters"). Realistic minimum dwell about 0.2-0.5 s,
  jitter tens of ms. Suitable for dwell >= 0.5 s: Bode / frequency-response measurement with a DMM or
  scope reading at each point (better than a hardware sweep here: each point is synchronous with the
  measurement), slow amplitude ramps, burn-in sequences.
- Must NOT replace a hardware sweep when: total time below a few seconds, phase-continuous chirps are
  needed (DDS frequency changes may or may not be phase continuous; not guaranteed), a sweep marker /
  trigger output is needed, or the DUT reacts to step glitches. Then use the native sweep (13/13 have it).
- Amplitude / offset / duty sweep (native on only 4 families) is a good software feature for all others.

### 3.7 Output-limit guard (DUT protection)
- Before any amplitude, offset, high/low, sweep end value or modulation depth is sent, check
  `|offs| + Vpp/2 (* load factor)` and, for AM, the peak at the configured depth against a per-channel
  user limit (`V_dut_max`). AM conventions differ: on DGE 100 % depth outputs the set amplitude and 0 %
  half of it (DGE-UM p33), so AM never exceeds AMPL there; Keysight-style AM can exceed it. For an
  offset/amplitude sweep check the end values. Fail closed: refuse when a needed reading is null.
- Limits: guards only BenchMesh-originated changes, not the front panel; prefer the native limit where it
  exists (UNI-T `LIMit`, Siglent `MAX_OUTPUT_AMP`, 33500B `VOLT:LIM`) and show it.
- This is the AWG counterpart of the PSU's computed power: one implementation for every instrument.

### 3.8 Channel tracking / copy emulation (2-channel units without COUP/TRACK, e.g. DGE)
- Copy: read CH1 status, write each value to CH2 (shape first, then frequency, amplitude, offset).
- Track: after every CH1 setter also send the CH2 setter (equal, ratio or offset). CH2 lags 20-100 ms.
- Phase relation is undefined after a frequency change unless the instrument re-aligns
  (Rigol `PHAS:SYNC`, Tek `PHAS:INIT`, UNI-T `SYSTem:PHASe:MODe SYNC`); DGE has no phase-align command.
  So: acceptable for static two-channel setups, NOT for differential / IQ / phase-coherent drive.
- Never track the output switch implicitly (energize must stay an explicit two-press action).

### 3.9 Must not be emulated
Modulation (AM/FM/PM/FSK need sub-ms updates), burst (cycle-accurate), gated burst, fast sweeps,
harmonics, counter. A "timed output" (output on for N s) is the only burst-like thing software may do,
as an explicit energize action with its jitter stated.

### 3.10 Validation helpers (always useful)
- Per-shape max frequency (`features.max_freq_hz`, already in FY6900 and DG1000Z model features) and
  per-frequency/load amplitude limits (DGE-UM p46: 20 Vpp HiZ up to 10 MHz, 8 Vpp up to 70 MHz) let the
  UI warn before sending; FM deviation limit = min(carrier, fmax - carrier) (DGE-UM p35).

---

## 4. Gap analysis

### 4.1 Contract (`devices/classes/AWG.yaml`)
Status: OUTPUT, SHAPE, FREQ, AMPL, OFFS, PHASE, LOAD, SYMM. Operations: output, shape, frequency,
amplitude, offset, phase, symmetry, screenshot. Nothing for duty, pulse width/edges, load setter,
amplitude unit, mode, modulation, sweep, burst, trigger, coupling. Versus the tiers: Core has 5 of
roughly 20 items (output, shape, frequency, amplitude, offset, phase), Common 1 of 12 (symmetry; LOAD
read-only).

### 4.2 Definitions (24 binding AWG)
| Operation | Bound by | Notes |
|---|---|---|
| output, shape, frequency, amplitude, offset | 24 / 24 | |
| symmetry | 23 / 24 | FY6900 has none |
| phase | 22 / 24 | GW AFG-2000/3000 have no CW phase command |
| status LOAD | 22 / 24 | read-only; FY6900 and **owon_dge** do not even read it (DGE has `OUTPut{ch}:IMPedance?`, DGE-PM p13) |
| duty, load (set), unit, high/low, pulse, mode, mod, sweep, burst, trigger, counter | **0 / 24** | every family's manual has most of them (matrix) |

Commands already defined but unused for the UI: `unit_q` (only a `requires` guard), `load_q`
(status only). The coverage report (§4) already proposed `duty` and `load`; neither was added.

### 4.3 UI (`AwgPanel.tsx`, `awgProfile()`)
- Shows: shape chips, output arm switch, WavePreview, frequency, amplitude, offset editors.
- **Bound but never shown:** phase (22 definitions), symmetry (23), LOAD status (22). The preview ignores
  symmetry (a 20 % ramp is drawn as 50 %).
- Frequency only in Hz (the user's complaint: no period), amplitude only in Vpp, no high/low.
- `AMPL` null (instrument in Vrms/dBm) shows an empty editor without a reason.
- No mode concept at all, so sweep/mod/burst switched on from the front panel are invisible: the panel
  then shows a CW carrier that is not what the output does.

### 4.4 OWON DGE2000 (the bench DGE2070): what the manual offers vs `devices/owon_dge.yaml`
Bound today: output, shape (SIN SQU PUL RAMP PRN), frequency, amplitude (Vpp), offset, phase (set only;
query in radians converted), symmetry. Available in DGE-PM but not bound:

| Group | Commands (prefix `SOURce{ch}:` unless noted) | Values / range | DGE-PM page |
|---|---|---|---|
| Load | `OUTPut{ch}:IMPedance {<ohms>\|INFinity\|MIN\|MAX}`, `?` -> ohms, `9.9E+37` = INF | 1 ohm-10 kohm, 1 ohm res | p13-14 |
| Modulation master | `MODE:STATe {ON\|OFF}`, `?` -> NR1 | enables "the modulation function" | p27 |
| AM | `AM[:DEPTh] <pct>`; `AM:INTernal:FREQuency <f>`; `AM:INTernal:FUNCtion {SINusoid\|SUQare\|RAMP\|PRNoise}` -> `SIN\|SUQ\|RAMP\|PRN`; `AM:SOURce {INTernal\|EXTernal}`; `AM:STATe` | 0-100 % (1 %), 2 mHz-100 kHz | p15-17 |
| FM | `FM[:DEViation] <f>`; `FM:INTernal:FREQuency`; `FM:INTernal:FUNCtion`; `FM:SOURce`; `FM:STATe` | dev 1 uHz to min(fc, fmax - fc) (DGE-UM p35) | p20-23 |
| PM | `PM[:DEViation] <x>[RAD\|DEG]` (default RAD); `PM:INTernal:FREQuency`; `PM:INTernal:FUNCtion`; `PM:SOURce`; `PM:STATe` | 0-180 deg | p28-30 |
| FSK | none (front panel only: rate 2 mHz-1 MHz, hop frequency) | | DGE-UM p36-37 |
| Sweep | `SWEep:STATe`; `SWEep:SPACing {LINear\|LOGarithmic}` -> `LIN\|LOG`; `SWEep:TIME <s>`; `SWEep:SOURce {TIMer\|MANual\|EXTernal}` -> `TIM\|MAN\|EXT`; `FREQuency:STARt/STOP <f>`; `FREQuency:CENTer/SPAN <f>` | time 1 ms-500 s; carrier SIN/SQU/RAMP/ARB | p23-25, p32-34 |
| Burst | `BURSt:STATe`; `BURSt:MODE {TRIGgered\|GATed}`; `BURSt:NCYCles {<n>\|INFinity}` (INF reads `9.9E+37`); `BURSt:INTernal:PERiod <s>`; `BURSt:SOURce {TIMer\|MANual\|EXTernal}`; `BURSt:GATE:POLarity {NORMal\|INVerted}` | 1-500 000 (PM) vs 1-60 000 (DGE-UM p31, p49) | p17-20 |
| Pulse | `PULSe:DCYCle <pct>`; `PULSe:WIDTh <s>`; `PULSe:TRANsition[:LEADing] <s>`; `PULSe:TRANsition:TRAiling <s>` | width >= 24 ns, edges >= 15 ns (DGE-UM p47) | p30-32 |
| Counter | `COUNter:FREQ?\|PERiod?\|DUTYcycle?\|PULSewidth?`, `COUNter:COUPling`, `:HFR`, `:SENSitivity` | DGE2000 has no counter input: do not bind | p7-9 |

DGE-specific traps for a binding (bench checks before marking "tested"):
1. `MODE:STATe` vs `AM:STATe`/`FM:STATe`/`PM:STATe`: the manual does not say how the type is selected
   (front panel: Mode key + AM/FM/PM/FSK softkey, DGE-UM p32). Probe whether `AM:STATe ON` alone switches
   to AM, whether `MODE:STATe ON` is needed, and what the queries report while FSK is on (panel-only).
2. The modulating-function token is printed `SUQare` / `SUQ` (p16, p22, p29) next to `SQUare` for the
   carrier (p26): accept both in the query map, probe which the setter takes.
3. No `*TRG` / `TRIGger` command: MANual sweep/burst trigger cannot be fired remotely; offer TIM only (or
   probe `*TRG`).
4. External source / gated burst / sweep EXT exist in SCPI but the DGE2000 has no input connector
   (DGE-UM p10, spec "Modulated signal source: Internal" p47-48, sweep and burst triggers "Internal,
   manual" p48-49): restrict the enums for DGE2000 models (per-model features or values).
5. No period, high/low, Vrms/dBm, DC or built-in ARB over SCPI (DGE-UM p15-16, p23; DGE-PM p26, p35):
   all of period / high-low / Vrms come from §3 software.
6. Setters never reply and a read right after a setter desyncs USBTMC (device notes): every new setter
   `reply: none`; rely on the existing gap and on the poll.
7. Poll budget: 15-110 ms per query; adding ~20 mode/mod/sweep/burst queries per channel belongs in the
   5 s group, or better a poll gated by the current mode (DSL gap F7 below).

---

## 5. Proposal

### 5.1 Contract additions (`AWG.yaml`, all optional unless stated)
Canonical vocabularies (API values; wire spellings mapped in the definitions as for SHAPE):

| Status key | Type | Values / unit | Operation | Tier |
|---|---|---|---|---|
| `MODE` | enum | `CW`, `MOD`, `SWEEP`, `BURST` | `mode` get/set (set `CW` switches mod/sweep/burst off) | Core |
| `MOD_TYPE` | enum | `AM`, `FM`, `PM`, `FSK`, `ASK`, `PSK`, `PWM`, `SUM`, `DSBAM` | `mod_type` | Core (AM/FM/PM), rest Common/Advanced |
| `MOD_SOURCE` | enum | `INT`, `EXT`, `CH` (FeelTech: the other channel) | `mod_source` | Common |
| `MOD_SHAPE` | enum | `SIN`, `SQU`, `RAMP`, `PRN`, `ARB` (internal modulating wave) | `mod_shape` | Core |
| `MOD_FREQ` | si | Hz (internal modulating frequency) | `mod_frequency` | Core |
| `MOD_DEPTH` | number | % (AM) | `mod_depth` | Core |
| `MOD_DEV` | si | Hz for FM, deg for PM, % duty for PWM (unit in the value spec) | `mod_deviation` | Core |
| `FSK_FREQ` | si | Hz (hop) | `fsk_frequency` | Common |
| `MOD_RATE` | si | Hz (FSK/ASK/PSK keying rate) | `mod_rate` | Common |
| `SWEEP_START`, `SWEEP_STOP` | si | Hz | `sweep_start`, `sweep_stop` | Core |
| `SWEEP_TIME` | si | s | `sweep_time` | Core |
| `SWEEP_SPACING` | enum | `LIN`, `LOG` (`STEP`, `LIST` advanced) | `sweep_spacing` | Core |
| `SWEEP_TRIG` | enum | `INT`, `EXT`, `MAN` | `sweep_trigger_source` | Common |
| `SWEEP_HOLD`, `SWEEP_RETURN` | si | s | same names | Advanced |
| `BURST_MODE` | enum | `NCYC`, `GATED`, `INF` | `burst_mode` | Core (NCYC) |
| `BURST_CYCLES` | integer | 1..N | `burst_cycles` | Core |
| `BURST_PERIOD` | si | s | `burst_period` | Core |
| `BURST_TRIG` | enum | `INT`, `EXT`, `MAN` | `burst_trigger_source` | Core (INT, MAN) |
| - | - | - | `trigger` set-only, enum `[ONCE]` (same pattern as LCR `trigger`) | Common |
| `DUTY` | number | % | `duty` | Common |
| `WIDTH` | si | s | `pulse_width` | Common |
| `RISE`, `FALL` | si | s | `pulse_rise`, `pulse_fall` | Common |
| `LOAD` (exists) | string | `"50"`..`"10000"`, `HIGHZ` | `load` get/set (enum `50`, `HIGHZ` + optional number) | Common |
| `HIGH`, `LOW` | si | V | `high`, `low` (only where native and atomic) | Common |
| `AMPL_UNIT` | enum | `VPP`, `VRMS`, `DBM` (what the instrument front panel uses) | `amplitude_unit` (explicit, never implicit) | Common |
| `VLIMIT_HIGH`, `VLIMIT_LOW` | si | V (native output limit) | `voltage_limit_high/low` | Advanced |
| `TRACK` | enum | `OFF`, `ON`, `INV` (native coupling/tracking) | `track` | Common |

Not in the contract (computed in the UI or a shared module, §3): period, Vrms, dBm, high/low on
instruments without native commands, pulse width from duty, DUT voltage, output-limit guard. Keep
`AMPL` canonical in Vpp: one setter path, conversions outside the driver.

Counter: model the counter input as its own readout, not as AWG channel keys: a small `CNT` class
(`FREQ`, `PERIOD`, `DUTY`, `WIDTH`, coupling, gate) would also cover Keysight 53200 (docs present in
BenchMeshDOC). Advanced for now.

### 5.2 Binding patterns (today's DSL)
- `MODE` status from flags, e.g. DGE: `burst_q == 1 ? 'BURST' : sweep_q == 1 ? 'SWEEP' : (am_q == 1 || fm_q == 1 || pm_q == 1) ? 'MOD' : 'CW'`;
  UNI-T 1000X reads `:CHAN{ch}:MODE?` directly; Siglent from `MDWV?`/`SWWV?`/`BTWV?` `STATE`.
- `mode` setter with step lists and `only:` (DSL classes table): `CW` -> all `*:STATe OFF`; `SWEEP` ->
  `BURS:STAT OFF`, mod off, `SWE:STAT ON`; the UTG900E combined enum `CONTINUE|AM|PM|FM|FSK|Line|Log`
  maps `mode` + `mod_type` + `sweep_spacing` onto one wire command (enum `from` CEL).
- `mod_*` setters on dialects with per-type commands (Tek/DGE `AM:INT:FREQ`, `FM:INT:FREQ`) use a computed
  param `from: "mod_type_q"` to pick the subtree, as SPM range commands do (DSL `from` over cached results).
- `requires` for mode-specific setters (e.g. `burst_cycles` only when `BURST_MODE == NCYC`) instead of
  hidden mode switches (DSL authoring rule 5).
- Per-model enum restriction (DGE2000: `INT`/`MAN` only, no `EXT`/`GATED`): today a separate value map per
  definition; per-model `features` can at least tell the UI.

### 5.3 Features hints (served by `GET /instruments/AWG/{id}`)
`max_freq_hz` per shape (exists), `amplitude_limits` `[{max_freq_hz, load, vpp_max, vpk_max}]` (DGE-UM p46),
`pulse_min_width_s`, `duty_limits` per frequency band, `mod_freq_range`, `sweep_time_range`,
`burst_cycles_max`, `has_sync_out`, `has_ext_input` (false on DGE2000), `channels_phase_align: bool`
(gates §3.8 tracking emulation), `noise_vpp_convention` (so §3.3 can stay off for noise).

### 5.4 Panel UX sketch (one panel, everything from features + value specs)
```
CH1 [~ sine][square][ramp][pulse][noise][DC]   [ CW | MOD | SWEEP | BURST ]   (ARM) OUTPUT
+--------------------------- preview (duty, symmetry, AM envelope, sweep ramp, burst train) --+
CW     Frequency [ 1.000 000 kHz ]  (Hz | s)        Amplitude [ 1.000 Vpp ] (Vpp | Vrms | dBm | H/L)
       Offset    [ 0.000 V ]  -> H/L mode replaces Amplitude/Offset by High [..] Low [..]
       Phase [ 0.0 deg ]   Duty [50 %] or Width [..] (SQU/PUL)   Symmetry [50 %] (RAMP)
       Rise / Fall (PUL)   Load ( 50 ohm | HiZ | 1 k.. )   derived line: T = 1 ms, 353.6 mVrms, H +0.5 V L -0.5 V
MOD    Type [AM][FM][PM][FSK]..  Source (INT|EXT)  Shape [sin][sq][ramp][noise]
       Mod freq [100 Hz]  Depth [100 %] | Deviation [100 Hz / 90 deg] | Hop [..] Rate [..]
SWEEP  Start [100 Hz]  Stop [1 kHz]  (start/stop | center/span, computed)  Time [1 s]  (LIN|LOG)
       Trigger (INT|EXT|MAN) [TRIGGER]      [Stepped sweep...] -> automation flow template (software)
BURST  Cycles [1] (N | INF | GATED)  Period [1 s]  Trigger (INT|EXT|MAN) [TRIGGER]
```
- Mode chips only for modes the definition binds; the carrier editors stay visible in MOD/SWEEP/BURST
  (they are the carrier settings on every family).
- The unit toggles (Hz/s, Vpp/Vrms/dBm/H-L) are per-viewer preferences (localStorage), conversions per §3;
  Vrms/dBm disabled for noise/ARB/DC and dBm disabled at HiZ.
- Phase, symmetry and LOAD are already bound: show them now (no backend work).
- Output-limit field per channel (`V_dut_max`) next to the load toggle; red when a setpoint is refused.
- Confirmation: derived entries confirm against the converted read-back (kit/confirm.ts semantics).

### 5.5 DSL capabilities that may be missing
| # | Gap | Why (AWG) |
|---|---|---|
| F7 | Poll groups gated by a CEL condition (`when: "mode_q == 'SWEEP'"`) | ~20 extra queries per channel only matter in that mode; DGE queries cost 15-110 ms |
| F8 | Value-spec limits from features/CEL (dynamic `max`) | FM deviation max depends on carrier; amplitude max on frequency and load; burst cycles max = period / T |
| F9 | Per-model enum value subsets | DGE2000 must not offer `EXT`/`GATED`; UTG900E has no burst |
| F10 | Number-or-token params (`INF` for burst cycles, `INF` load) | today needs a separate mode enum |
| F2 / F5 (coverage) | `pairs` parse, regex "no match -> null" | Siglent `MDWV?`, `SWWV?`, `BTWV?` are key,value lists with type-dependent keys |
| F1 (coverage) | Caller value in computed params | FeelTech / JDS register scaling for the new setters too |
| - | Multi-value atomic setter (two params in one POST) | native `HLEV`+`LLEV`, sweep start+stop on Siglent `SWWV START,..,STOP,..`; single-value ops work, but transient states (start > stop) may be refused by the instrument; order or a combined op needed |

### 5.6 Suggested order of work
1. UI only: show phase, symmetry, LOAD; period toggle; Vrms/dBm/H-L display and entry; duty/width
   display; output-limit guard (pure frontend + profiles tests).
2. Contract + bases: `load`, `duty`, then `mode` + sweep (Core, 13/13) in `scpi_awg`, `tek_afg` (DGE/XDG/AFG1000),
   `siglent_sdg`, `unit_utg`, `gw_afg`; FeelTech/JDS where F1 allows.
3. Modulation AM/FM/PM and burst N-cycle (12/13), then FSK and trigger.
4. DGE bench session for the §4.4 traps; capture for replay tests.
5. Software stepped sweep as an automation flow template opened from the SWEEP mode.
