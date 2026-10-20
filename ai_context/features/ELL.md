# ELL (DC electronic load): common functional denominator and software value

Research 2026-10-20. Read-only study, nothing in the repo was changed.

**Sources** (page numbers are PDF pages of the local files unless marked "printed"):
- Repo: contract `devices/classes/ELL.yaml`; base `devices/bases/scpi_load.yaml`; definitions
  `devices/{owon_oel,korad_kel10x,rigol_dl3000,siglent_sdl1000x,itech_it8500plus,bk_8600}.yaml`;
  coverage `ai_context/migration/coverage/ELL.md` (COV); UI `frontend/src/ui/bench/panels/EllPanel.tsx`,
  `frontend/src/ui/bench/profiles.ts` (`ellProfile`); automation `ai_context/AUTOMATION.md`;
  DSL `ai_context/migration/DEVICE_DSL.md`.
- Manuals in `/home/marek/project/BenchMeshDOC/`: Rigol `DL3000_ProgrammingGuide_EN_2019.pdf` (RG),
  Siglent `SDL1000X_programming_guide_E02B.pdf` (SG), ITECH `ITECH_IT8500plus_Programming-Guide_SCPI.pdf`
  (IS; printed page + 9 = PDF page), ITECH frame guide (IF), B&K `85xx_manual.pdf` (BK85),
  B&K `8600_Series_Programming_Manual.pdf` (BP), Korad `KEL10x_Communication_Commands_with_Computer.pdf`
  (KC), OWON `OEL15_30_Series_SCPI_Programming Manual.pdf` (OP) and `OEL15&30 ... User_Manual.pdf` (OU).
- Web: East Tester ET5410 user manual (ET; https://static.wikitide.net/rlabwiki/7/71/ET5410_en.pdf),
  Atorch DL24P product pages (https://done.land/tools/devices/electronicload/dl24atorch/,
  https://gotronik.pl/elektroniczne-obciazenie-dl24p-200v-20a-180w-bluetooth-kartapdf-7825.html),
  ZKETECH EBD-A20H datasheet (https://cdn.ozdisan.com/ETicaret_Dosya/730979_162162.pdf), sigrok drivers
  for Atorch, ZKETECH, Re:load Pro and Maynuo (cited in COV and the `SOURCES.md` files).

---

## 1. Feature matrix (10 families, cheap USB loads to pro)

Columns: **AT** Atorch DL24P (~EUR 40, BLE/serial push stream), **ZK** ZKETECH EBD-A20H (~EUR 50,
binary), **RL** Arachnid Re:load Pro (~EUR 80, ASCII), **KO** Korad KEL102/103 (= Tenma 72-132x0, RND
320-KEL10x), **OW** OWON OEL15/30, **ET** East Tester ET5410, **SI** Siglent SDL1020X/1030X(-E), **RI**
Rigol DL3021/3031(A), **IT** ITECH IT8500+ (SCPI) / B&K 8500 (same instrument, binary frame), **BK**
B&K 8600 (= ITECH IT8800). Maynuo M97xx (Modbus) is close to IT and noted where relevant.

Legend: **R** = remotely controllable/readable (documented command), **P** = front panel only (no
remote command known), **-** = absent, **?** = unverified. "def" = bound in our definition today.

| Feature | AT | ZK | RL | KO | OW | ET | SI | RI | IT | BK | Count (any) | Sources |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| CC mode | P (keys only) | R | R | R def | R def | R | R def | R def | R def | R def | 10 | COV 2.1-2.10 |
| CV mode | P | - | - | R def | R def | R | R def | R def | R def | R def | 8 | DL24P pages; ZK datasheet (DSC-CC/CP only) |
| CR mode | P | - | - | R def | R def | R | R def | R def | R def | R def | 8 | as above |
| CP mode | P | R (DSC-CP) | - | R def | R def | R | R def | R def | R def | R def | 9 | ZK datasheet |
| CC+CV / CR+CV combined | - | - | - | - | partial (`CURR:LIMit` in CV, OP p.15) | P (ET p.8) | - | limits only (`VLIM/ILIM`, RG p.42) | - | - | 1-2 | ET p.8 "CC+CV, CR+CV" |
| Measured V, I | R (push) | R (stream) | R (`read` mA mV) | R def | R def | R | R def | R def | R def | R def | 10 | COV |
| Measured P (device) | R (push) | P (display) | - | R def | R def | ? | R def | R def | R def | R def (`FETC:POW?`) | 8 | BP p.66 |
| Measured R (device) | - | - | - | - | - | ? | R (`MEAS:RES?`) | R (`MEAS:RES?`, RG p.31-33) | R (IS printed 47) | - | 3 | |
| Input on/off | - (most firmware) | R (start/stop) | R (`on/off`) | R def | R def | R | R def | R def | R def | R def | 9 | COV 2.4 |
| Short circuit | - | - | - | R (a mode, `:FUNC SHORt`) | R def | P (ET p.20) | R def | P (`SYST:KEY 33` only) | R def | R def | 7 (6 remote) | COV 2.2, 4 |
| Transient/dynamic A/B (levels, widths or freq/duty, CONT/PULS/TOGG) | - | - | - | R (`:DYN 1..6`, KC #30) | R (`DYN:HIGH/LOW/DWEL/SLEW/MODE`, OP p.16) | P (ET p.12) | R (`CURR:TRAN:*`, SG p.21) | R (`CURR:TRAN:*` incl. FREQ/ADUT, RG p.43-47) | R (`DYN:*`, IS printed 36-43; BK85 0x32-0x39) | R (`CURR:TRAN:*`, BP p.49) | 7 | |
| List/sequence | - | - | - | R (`:LIST`, 7 slots, KC #20) | ? (feature `dynamic_list`) | P (ET p.14) | R (`LIST:*`, SG p.41) | R (`LIST:*`, RG p.51) | R frame only (BK85 0x3A-0x4D) | R (`LIST:*` 2-84 steps, BP p.63) | 6-7 | |
| Battery test (stop on V / capacity / time) | P (V cut-off, timer) | R (start with I and cut-off V) | - (UVLO only) | R (`:BATT`, KC #26) | P (T suffix, OU 4.13.1) | P (CC/CR, ET p.18) | R (`BATT:*`, SG p.35-40) | R (`BATT:*`, RG p.54-58) | R (min V `0x4E`, BK85) | ? (not in BP) | 8 | |
| Capacity/energy/time readback | R (Ah, Wh, time in push) | ? (displayed; sigrok reads V/I) | - | R (`:BATT:CAP?`, `:BATT:TIM?`) | - | ? | R (`DISCHA:CAP?` mAh, `DISCHA:TIM?`, SG p.39) | R (`MEAS:CAP?`, `MEAS:WATT?`, `MEAS:DISCT?`, RG p.33-34) | - | - | 4 | |
| Battery internal resistance (DCR) | - | - | - | - | - | - | R (`BATT:DCR:*`, SG p.38-40) | - | - | - | 1 | |
| OCP/OPP *test* (ramp until trip) | - | - | - | R (`:OCP`, `:OPP`, KC #22-25) | P (OCP test, T suffix, OU 4.13.2) | P (generic scan test, ET p.17) | R (`OCP:*`, `OPP:*`, SG p.45-50) | R (`OCP:*`, `OPP:*`, RG p.58-66) | R (OCP only, IS printed 48-52) | - | 6 (4 remote) | |
| LED mode (CR-LED, Vf/Io/Rd) | - | - | - | - | - | P (ET p.19) | R (`LED:*`, SG p.34) | - | R (`LED:*`, IS printed 58) | R (`RES:LED`, `RES:VDRop`, BP p.57) | 4 | |
| Von / Von latch | - | - | - | in OCP/OPP params only | R (`VOLT:ON`, OP p.21) | ? | R (`VOLT:LEV:ON`, `VOLT:LATC`, SG p.57) | R (`CURR:VON`, RG p.42) | R (`VOLT:ON`, IS printed 30) | R (`VOLT:ON`, `VOLT:LATC`, BP p.52) | 5 | |
| Voff / undervoltage cut-off (hardware) | P (V cut-off) | R (cut-off V) | R (`uvlo <mV>`) | - | - | ? | (battery stop only) | (battery stop only) | R (`VOLT:OFF`, IS printed 31) | - | 4 | |
| Slew rate | - | - | - | list/dyn only | R (rise/fall per mode, OP p.15) + presets | ? | R (`CURR:SLEW:POS/NEG`) | R (`CURR:SLEW`, RG p.40-41) | R (`CURR:SLEW:RISE/FALL`) | R (`CURR:SLEW`) | 5 | |
| Ranges (I, V, R) | - | - | - | - | R (`CURR:RANG`, `VOLT:RANG`, OP p.15) | P (ET p.8) | R (`IRANG/VRANG/RRANG`, SG p.20) | R (`CURR:RANG` 4/40 or 6/60 A, `VOLT:RANG`, `RES:RANG`) | R (`CURR:RANG`, `VOLT:RANG[:AUTO]`) | R (`CURR:RANG` ...) | 6 | |
| Remote sense | ? (4-wire DL24) | ? (4-wire) | - | R def (`:SYST:COMP`, undocumented) | R def | ? | R def | R def | R def | R def | 6-8 | |
| Protection levels settable (soft OCP/OPP/OVP) | - | - | - | - | R (`VOLT/CURR/POW:PROT`, OP p.15) | ? | R (`CURR/POW:PROT:*`, SG p.58) | - (OCP/OPP are tests) | R (`CURR/POW:PROT`, IS printed 28-29) | R (`CURR/POW:PROT:*`, BP p.48-49) | 4 | |
| Protection trip status (remote) | - | - | events (`overtemp`, `undervolt`) | - | R def (`MEAS:ALL:INFO?`) | ? | - (no `STAT:QUES`) | R def (`STAT:QUES`) | R def (`STAT:QUES`, OT bit 4) | R def (`STAT:QUES`, PS bit 13) | 5 | COV 2.3 |
| OTP status bit | - | - | R (event) | - | P (OU OTP) | P | - | via PS bit 13 | R (bit 4, not bound) | R (bit 4 OT, not bound) | 3-4 | |
| Protection clear | - | - | - | - | - | - | - | - | R (frame 0x90) | R (`PROT:CLE`, BP p.45) | 2 | |
| Load-on timer | P (timer cut-off) | ? | - | (`:BATT` time) | - | ? | (`BATT:TIM`) | (`BATT:TIM`) | R (0x50-0x53) | R (`INP:TIM`, BP p.35) | 2 (pure) | |
| Min/max/peak readings | - | - | - | - | - | - | - | R (`MEAS:VOLT:MAX/MIN`) | R (`MEAS:*:MAX/MIN/PTP`, `PEAK:*`, IS p.53, p.63) | - | 2 | |
| Waveform capture of readings | - | - | - | - | - | - | R (`MEAS:WAVE?`, 200 pts, SG p.17) | R (`MEAS:WAV?`) | - | - | 2 | |
| Rise/fall time test (V) | - | - | - | - | - | - | - | - | R (`TIME:VOLT:*`, IS printed 56) | - | 1 | |
| Averaging count | - | - | - | - | - | - | R (`SENS:AVER:COUN`, SG p.61) | - | - | R (`SENS:AVER:COUN`, BP p.67) | 2 | |
| Remote/local or panel lock | - | - | - | R def (`:SYST:LOCK`) | R def | P | - | - | R def (`SYST:REM/LOC`) | R def | 4 | |
| Parallel / master-slave | - | - | - | - | - (`parallel: false`) | - | - | - | - | - | 0 | |

Notes: Re:load Pro reports only mA and mV, so P is always computed (sigrok). Atorch DL24P has
CC/CV/CR/CP on the panel, but remote control is limited to simulated +/- key presses (COV 2.4). The
Rigol, Siglent and Korad "OCP/OPP" subsystems are **test routines**, not protection limits (COV 2.2).

---

## 2. Tiers

Tier rule: **Core** = 9-10 of 10 families, **Common** = 5-8, **Advanced** = 1-4.

**Core (4):** CC setpoint, measured V, measured I, input on/off.
(P is *core from the user's point of view* because software computes it when absent: section 3.1.)

**Common (12):** CV, CR, CP, measured P (device), short circuit, transient/dynamic, list,
battery test (stop conditions), OCP/OPP test routine, Von (+latch), slew rate, ranges, remote sense,
protection trip status. That is 14 lines in the matrix; CV/CR/CP share one "mode" control, so 12
distinct controls.

**Advanced (13):** CC+CV / CR+CV, measured R (device), capacity/energy readback, battery DCR, LED
mode, Voff/UVLO, settable protection levels, OTP status bit, protection clear, load-on timer,
min/max/peak, waveform capture, averaging count (plus V rise/fall test, ITECH only). Parallel: none.

What BenchMesh exposes today (contract + UI): Core 4/4; Common: mode CC/CV/CR/CP, measured P,
remote sense (set only), OVP/OCP/OPP status. Short is in the contract and bound in 4 definitions but
**has no UI control**. Everything else in Common/Advanced is absent.

---

## 3. Software-level value (what BenchMesh can add when the instrument lacks it)

General rule: a computed value is shown with the same readout as a measured one, plus a small
"calc" tag and a tooltip with the formula, and the device value always wins when present. All
computations belong **server-side** (one per instrument, from the registry poll stream) so they keep
running when no window is open, are recordable like any status key, and are the same for UI, flows
and MCP.

### 3.1 Power P = V * I
- **Who needs it:** Re:load Pro (mA/mV only), ZKETECH (stream has V and I), any future CC-only load.
  The 6 defined loads all report P, but the contract makes PIN *required*, so a definition can bind
  `PIN: {expr: "meas_v * meas_i", as: si}` today (Tenma PSU does exactly this for POUT:
  `devices/tenma_72.yaml:119`).
- **Accuracy:** error ~ eV + eI (e.g. 0.05 % + 0.1 % + offsets). Skew: V and I come from two queries
  10-100 ms apart; harmless for DC, meaningless in transient/list mode (show "-" there).
- **Not when:** the device reports P (its P is sampled simultaneously and averaged).

### 3.2 Resistance R = V / I (and conductance)
- Useful in every mode (the load's "equivalent resistance", or a source's apparent resistance).
  Device value only on SI/RI/IT.
- **Guard:** null when |I| < max(1 mA, 0.1 % of the active range), or input OFF; otherwise noise
  produces megaohms. Not meaningful in short mode (shows the lead resistance; label it so).

### 3.3 Charge (Ah), energy (Wh), elapsed time, min/max/avg
- **Who needs it:** everyone except RI/SI/KO/AT battery modes; and even those count only inside
  their battery mode, while users want Ah/Wh in plain CC/CP too.
- **Method:** trapezoidal integration of IIN and PIN over poll samples (500 ms poll in all 6
  definitions), counting only while INPUT=ON. Elapsed = time with INPUT=ON. Reset: explicit button,
  and automatically on a new "test" start; never silently on reconnect.
- **Accuracy:** for steady DC the integral error equals the current accuracy (~0.1 %); sampling at
  2 Hz adds error only when current changes faster than ~1 Hz (dynamic/list mode: do not integrate,
  or mark "approximate"). A gap (device offline > 2 s) stops accumulation and is reported as
  "n s missing", never interpolated.
- **min/max/avg:** running statistics of V/I/P since reset. Label "sampled at 2 Hz": ITECH's
  hardware `PEAK`/`MAX/MIN` (IS p.53, p.63) catch ms peaks that polling misses, so prefer it there.
- **Not when:** a native counter exists and the device's test is running (show the device's Ah,
  e.g. RG `MEAS:CAP?`, SG `DISCHA:CAP?`, KC `:BATT:CAP?`, Atorch push Wh/Ah).

### 3.4 Software battery discharge test (any load with CC and input on/off)
- **Flow:** set mode/setpoint (CC, or CP/CR if available), input ON, integrate Ah/Wh, stop on
  VIN <= Vcut (debounced: N consecutive samples, e.g. 3 = 1.5 s), or Ah >= limit, or time >= limit;
  input OFF; report capacity (Ah, Wh), duration, start/end voltage, mean V, and keep a recording
  (V, I, P, Ah). Optional rest phase afterwards to log recovery voltage.
- **Safety:** stopping = input OFF, the safe state; the automation executor already switches off
  what a failed/aborted run switched on (AUTOMATION.md "Safe state").
- **Caveats:** (a) poll latency 0.5-1 s plus command time: at the knee of a Li-ion curve dV/dt is
  ~ mV/s, so overshoot below Vcut is a few mV, acceptable; for NiMH/lead-acid fine. (b) Lead drop
  makes the load see a lower voltage: use remote sense or a separate DMM on the cell. (c) If the
  BenchMesh process or USB link dies, the load keeps discharging. Mitigation: arm a **hardware
  backstop** where it exists (IT `VOLT:OFF`, Re:load `uvlo`, ZKETECH cut-off V, device battery
  mode with its own Vstop on RI/SI/KO), and say clearly in the UI when there is none ("no hardware
  cut-off; do not leave unattended"). (d) Never below a chemistry-safe floor; offer presets
  (Li-ion 3.0 V/cell, LiFePO4 2.5 V, NiMH 1.0 V, lead-acid 1.75 V/cell).
- **Not when:** the load has a native battery mode with remote readback (RI, SI, KO): drive the
  native mode, it stops in hardware. Use software only to add Wh/plots where the device lacks them.
- **Possible today as a flow** (set, repeat-until `VIN <= cut`, read, let `ah = ah + i*dt/3600`,
  record): good template, but a built-in test is better (no 0.1 s loop floor, server-side
  integration, report).

### 3.5 Software CP or CR on a CC-only load (closed loop)
- **Who:** Re:load Pro, ZKETECH (CR), Atorch remote, any CC-only definition.
- **Method:** each fresh reading, I_set = P_target / VIN (CP) or VIN / R_target (CR), clamped to
  [0, Imax, Pmax/VIN], slew-limited (max step e.g. 10 % of range per cycle), skip when VIN < Vmin.
- **Timing:** loop rate = poll rate (1-2 Hz). Regulation is good for sources whose voltage changes
  slowly (batteries, bench PSUs in CV, solar panels in steady light).
- **Caveats/not when:** a source in current limit or a solar panel past its MPP has negative
  dV/dI; CP is then bistable even in hardware and software CP will collapse the voltage (stop
  condition VIN < Vmin required). Never emulate CV (needs a fast loop; a slow loop against a stiff
  source oscillates or slams the limit). Never emulate any mode while the load is in transient/list.
  Label the mode "CP (software, ~1 Hz)".

### 3.6 Stepped sweeps: I-V curves and internal resistance
- **I-V curve of a PSU/battery/solar panel:** step the setpoint (CC from 0 up, or CV from Voc down
  for solar panels, which avoids collapsing to 0 V past Isc), dwell >= 2 poll periods (>= 1 s), read
  V and I, record points; plot V(I) and P(I), mark MPP = max(V*I). Also gives PSU **load
  regulation** (dV over 0..Irated) and the CV->CC crossover.
- **Internal resistance (DC, two-step):** R = (V1 - V2) / (I2 - I1), steps held >= 2 s (polling).
  Report the pulse length with the result (ohmic + polarisation; ms pulses give lower values, so
  this is not comparable to 1 kHz AC IR meters). Use sense leads. Native on SI only (`BATT:DCR`).
- **Not when:** the device has list mode and the steps must be faster than ~0.5 s: upload a list
  (RI/SI/BK/KO) instead and read results; software steps are limited by the poll.

### 3.7 OCP trip test of a PSU (two instruments)
- Ramp load current in steps (dwell >= 1 s), watch the PSU (its OUTPUT/protection status, or VIN
  falling below a threshold = CV->CC transition or trip). Report trip current (step resolution),
  the PSU's set limit, error %. Same pattern for OPP (CP steps).
- **Safety:** cap the ramp at the PSU's rated current plus a margin; load input OFF at the end;
  arm required (energising).
- **Not when:** a native OCP/OPP test exists (RI, SI, KO, IT): it steps in ms and reports the
  trip value; software adds only the cross-check against the PSU's own reported state.

### 3.8 Efficiency with a PSU (and DUT)
- eta = P_load / P_source, with P_source from the PSU's POUT (or V*I computed, 3.1) feeding a
  converter whose output goes into the load. Sweep load current -> efficiency curve; also quiescent
  current at zero load.
- **Accuracy:** sum of both instruments' power errors (~0.2-0.5 %) plus inter-instrument skew
  (hundreds of ms): steady state only, dwell >= 2 s per point. Wire drops: sense on both sides.

### 3.9 Smaller additions
- **Software Von/Voff:** input ON only when VIN > Von, OFF when VIN < Voff (latency 0.5-1 s).
  Only a convenience; real inrush protection needs hardware Von.
- **Setpoint memory per mode:** remember the last CC/CV/CR/CP setpoints in the server so a mode
  switch restores them (OEL silently ignores setpoints in another mode, Korad switches mode on a
  setpoint; COV 4.3).
- **Power derating guard:** refuse/clamp a CC setpoint where setpoint * VIN > rated power (from
  `absolute_limits.power`), with a clear message instead of a silent device clamp.
- **Protection reason text:** decode PS/OT/OV/OC/OP bits into one message with a "clear" action
  where the device has one.

---

## 4. Gap analysis

### 4.1 Contract (`devices/classes/ELL.yaml`)
- Status (9): VIN, IIN, PIN, INPUT, MODE required; REMOTE, OVP, OCP, OPP optional.
  Missing: RIN, OTP, PS (protection shutdown), SHORT state, SENSE state, FUNC (fixed/transient/
  list/battery/test: COV 2.2 `FUNC:MODE`), CAP/ENERGY/ELAPSED, range, Von.
- Operations (9): mode, input, remote, curr, volt, res, pow, short, remote_compensation (set only).
  Missing every Common item beyond modes: ranges, slew, Von/latch, transient, list, battery,
  OCP/OPP tests, protection levels and clear.
- `MODE` is a free string; Korad emits `SHORT`, `BATT`, `OTHER`, OEL `DYN`, ITECH `DYN`, base
  `LED`/`IMP`: no canonical vocabulary separates "regulation mode" from "function".
- Contract description says the UI reads `remote` as required; COV 4.1 shows it is optional in
  practice (Rigol, Siglent have none). The file already treats it as optional: consistent.

### 4.2 Definitions (bindings counted from the YAML; base inherited)

| Definition | Status keys bound | Ops bound | Not bound although the manual documents it |
|---|---|---|---|
| owon_oel | 9/9 | 9/9 (sense set only) | ranges, slew rise/fall per mode, `DYN:*`, `VOLT:ON`, `VOLT/CURR/POW:PROT` levels, `CURR:LIMit`, trigger (OP p.15-21) |
| korad_kel10x | 6/9 (no OVP/OCP/OPP: none exist) | 8/9 (no short: it is a mode) | `:BATT`, `:BATT:CAP?/TIM?`, `:OCP`, `:OPP`, `:LIST`, `:DYN`, `*SAV/*RCL`, `:VOLT/CURR/POW/RES:UPP?` limits (KC) |
| rigol_dl3000 | 8/9 (no REMOTE) | 7/9 (no remote, short) | `FUNC:MODE`, `CURR:RANG`, `VOLT:RANG`, `RES:RANG`, slew, `CURR:VON`, VLIM/ILIM, `TRAN`, `LIST`, `BATT`, `OCP`, `OPP`, `MEAS:RES?`, `MEAS:CAP?/WATT?/DISCT?`, `MEAS:*:MAX/MIN`, `MEAS:WAV?`, `SENS?` query (RG) |
| siglent_sdl1000x | 5/9 (no REMOTE, no trip status exists) | 8/9 (no remote) | `IRANG/VRANG/RRANG`, slew, `CURR:TRAN:*`, `LIST`, `PROG`, `BATT:*` incl. `DISCHA:CAP?/TIM?` and `DCR`, `OCP/OPP` tests, `LED:*`, `VOLT:LEV:ON`, `VOLT:LATC`, `CURR/POW:PROT`, `MEAS:RES?`, `MEAS:WAVE?`, `SENS:AVER`, `SYST:SENS?` (SG) |
| itech_it8500plus | 9/9 (REMOTE = client state) | 9/9 | OT bit 4 parsed but **not exposed** (`ques_q.ot`), ranges, slew, `CURR/POW:PROT`, `VOLT:ON/OFF`, `DYN:*`, `OCP` test, `PEAK`, `MEAS:*:MAX/MIN/PTP`, `MEAS:RES?`, `TIME:VOLT:*`, `LED:*` (IS) |
| bk_8600 | 9/9 | 9/9 | PS/OT bits (base parses PS, not exposed), `PROT:CLE`, ranges, slew, `CURR/POW:PROT`, `VOLT:ON/LATC`, `CURR:TRAN:*`, `LIST:*`, `RES:LED/VDRop`, `INP:TIM`, `SENS:AVER`, `TRAC` buffer (BP) |

Definition issues found:
- `bk_8600` and `itech_it8500plus` list no `RES` mode in `features.modes` but inherit RES from the
  base (`scpi_load.yaml`), so CR appears; their RES setpoint max is the base 15000 ohm, not
  per-model. Verify limits per model.
- `korad_kel10x` declares `features.modes.SHORT` without a setpoint: `ellProfile` drops modes
  without a setpoint (`profiles.ts` `filter(... m?.setpoint ...)`), so SHORT is unreachable from the
  UI, but MODE=SHORT set at the panel shows up as an annunciator label. Same for OEL/ITECH `DYN`
  and Siglent `LED`.
- Rigol `sense_set` exists, no query bound although `:SOUR:SENS?` exists (RG p.67); the UI switch
  shows a local guess ("the load does not report this setting").
- Base `ques_q` parses `ps` (bit 13) for Rigol/BK and ITECH parses `ot` (bit 4); neither reaches a
  status key. Cheap win.

### 4.3 UI (`EllPanel.tsx`, `ellProfile`) vs tiers
- Shown: V/I/P trio (recordable), annunciators REM/INPUT/mode/OVP/OCP/OPP, mode selector (only modes
  with a setpoint), knob + digit editor for the active mode's setpoint, input arm switch, remote
  switch, sense switch (write-only).
- Core: 4/4 covered. Common: modes and P covered; **short not shown** although bound in 4
  definitions; protection status shown but no "clear" and no OTP/PS; nothing for ranges, slew,
  Von, transient, list, battery, OCP/OPP test.
- Features declared but unused by the UI: `absolute_limits.power` (only as readout width),
  `slew_presets`, `dynamic_list`, `parallel`, `lock`, `remote_sense` (UI uses the setter instead).
- No computed value anywhere: no R, Ah, Wh, time, min/max.

---

## 5. Proposal

### 5.1 Contract additions (all optional, `devices/classes/ELL.yaml`)

Status keys:
```yaml
  RIN:     {type: si, description: "Equivalent resistance V/I (ohm); device MEAS:RES? or computed"}
  OTP:     {type: bool, description: "Over-temperature tripped"}
  PROT:    {type: bool, description: "Protection shutdown (any cause; Rigol/BK PS bit 13)"}
  SHORT:   {type: enum, values: [ON, OFF]}
  SENSE:   {type: enum, values: [ON, OFF], description: "Remote sense state where queryable"}
  FUNC:    {type: enum, values: [FIXED, TRAN, LIST, BATT, OCPT, OPPT, LED, SHORT, OTHER],
            description: "What drives the input; MODE stays the regulation mode CURR|VOLT|RES|POW"}
  BATT_CAP:  {type: si, description: "Device discharge capacity (Ah) while FUNC=BATT"}
  BATT_ENERGY: {type: si, description: "Device discharge energy (Wh)"}
  BATT_TIME: {type: si, description: "Device discharge time (s)"}
  RANGE_I: {type: string, description: "Active current range key (features.ranges.curr)"}
```
Operations:
```yaml
  sense:        {get: true, set: true, value: enum, values: [ON, OFF]}   # replaces remote_compensation
  protection_clear: {set: true, value: enum, values: [CLEAR]}
  curr_range / volt_range / res_range: {get: true, set: true, value: enum}  # keys from features.ranges
  slew:         {get: true, set: true, value: number, description: "A/us (or enum preset, OEL)"}
  von:          {get: true, set: true, value: number}   von_latch: {get, set, enum ON/OFF}
  voff:         {get: true, set: true, value: number, description: "hardware cut-off (ITECH VOLT:OFF, Re:load uvlo)"}
  ocp_level / opp_level / ovp_level: {get: true, set: true, value: number}
  tran:         {get: true, set: true, value: enum, values: [ON, OFF]}  # energizes
  tran_mode:    {value: enum, values: [CONT, PULS, TOGG]}
  tran_a / tran_b: {value: number}  tran_a_width / tran_b_width: {value: number, unit: s}
  tran_freq / tran_duty: {value: number}   # Rigol/Korad style; features say which pair exists
  batt_stop_volt / batt_stop_cap / batt_stop_time: {value: number}
  batt_run:     {set: true, value: enum, values: [START, STOP]}    # energizes
```
Canonical vocabularies: MODE `CURR|VOLT|RES|POW` (+ `CURR_VOLT`, `RES_VOLT` for CC+CV / CR+CV);
FUNC as above (map Korad `SHORT/BATTERY/OCP/OPP/LIST/PULSE/TOGGLE`, Rigol `FIX/LIST/WAV/BATT/OCP/OPP`,
BK85 `FIXED/SHORT/TRAN/LIST/BATTERY`); transient mode `CONT|PULS|TOGG`; protection causes
`OV|OC|OP|OT|RV` (reverse voltage).

Features hints (per definition, data for the UI):
```yaml
features:
  ranges: {curr: [{key: LOW, max: 4}, {key: HIGH, max: 40}], volt: [...], res: [...]}
  slew: {min: 0.001, max: 2.5, unit: "A/us"}            # or slew_presets (exists, OEL)
  transient: {params: [a, b, a_width, b_width] | [a, b, freq, duty], modes: [CONT, PULS, TOGG]}
  battery: {native: true, modes: [CURR, RES, POW], stops: [volt, cap, time], readback: [cap, energy, time]}
  hardware_cutoff: voff            # what the software battery test can arm as a backstop
  ocp_test: native                 # native|none
  measured: {power: device, resistance: computed}   # tells the UI which values get the "calc" tag
```

### 5.2 Software layer ("derived signals" in bm-server, one module per class)
- Input: the instrument's status after each poll. Output: extra status keys merged into the
  registry with `source: computed` metadata: `PIN` (if not bound), `RIN`, `AH`, `WH`, `ELAPSED`,
  `VIN_MIN/MAX`, `IIN_AVG` ... plus `POST /instruments/ELL/{id}/{ch}/counters/reset`.
- Tests as server-side runners on the automation executor (one run at a time, arm, safe state,
  log, recording): **battery discharge**, **internal resistance**, **I-V sweep**, **PSU OCP/OPP
  trip**, **efficiency curve**. Each chooses native when `features` say so, else software, and
  writes a result summary (capacity, R, MPP, trip current, eta table). Shipped as built-in flow
  templates first (zero new engine code), promoted to dedicated runners when the 0.1 s loop floor
  or integration inside CEL becomes limiting.

### 5.3 Panel UX sketch (one ELL panel, everything driven by features + methods)
```
+--------------------------------------------------------------+
| V 12.034 V   I 2.000 A   P 24.07 W                            |  glass (as today)
| R 6.017 ohm calc   Ah 1.234 calc   Wh 14.8 calc   t 00:37:12  |  secondary row, reset (x)
| [REM][INPUT][CC][RANGE HI]  [OVP][OCP][OPP][OTP] [Clear]      |  annunciators + clear if bound
+--------------------------------------------------------------+
| Mode: [CC][CV][CR][CP][CP soft]   Short: [arm switch]         |  short = energizing, two presses
| Setpoint knob + digits (as today), Range [LO|HI], Slew [...]  |
| Input [arm switch]   Remote [ ]   Sense [ ] (state if queried)|
+--------------------------------------------------------------+
| > Protection & limits: OCP/OPP/OVP levels, Von, latch, Voff   |  folded sections, only when bound
| > Transient: A/B, widths or freq/duty, mode, [Run]            |
| > Tests: Battery | Internal R | I-V sweep | PSU OCP trip       |  native or "software" badge,
|          stop V / Ah / time, preset chemistry, [Start] [Stop] |  hardware backstop shown or warned
+--------------------------------------------------------------+
```
Test results appear inline (capacity, curve thumbnail) and open the recording.

### 5.4 DSL gaps that block this
1. **Stateful derived values:** CEL status expressions are stateless over the latest poll
   (DEVICE_DSL "CEL environment"); integration, min/max and elapsed time cannot be expressed. Keep
   them out of the DSL: class-level server module (5.2). Only the instantaneous `V*I`, `V/I` stay
   as CEL (already possible).
2. **Multi-parameter setters:** an operation binds one POST value to one param (DEVICE_DSL
   `operations`). Korad `:BATT g,range,I,V,Ah,t`, `:OCP ...` (11 fields), `:DYN 1,V1,V2,f,duty`
   and BK85 transient frames (0x32: level A, width A, level B, width B, mode in one frame) need a
   JSON-object value or a "staged params + apply" pattern.
3. **Array values** for list mode (steps of level/width/slew); class contract types have `list`
   only for numbers.
4. **Long-running device actions with results** (native battery/OCP test: start, poll state, read
   result): needs an operation kind `action` with a completion status key, so the executor can wait.
5. **Mode vs function split:** `features.modes` mixes regulation modes and functions (SHORT, DYN,
   LED, BATT); add `features.functions` and the FUNC status key.
6. Earlier blockers remain (COV section 5): binary frames (ITECH/BK 8500, ZKETECH, Array), push
   streams (Atorch, Re:load `monitor`), UDP (Korad LAN), BLE (Atorch). Modbus RTU is now in the DSL
   (DEVICE_DSL "Modbus RTU"), which unblocks Maynuo M97xx and OWON Modbus mode.

### 5.5 Suggested order
1. Cheap wins in definitions: expose OTP/PROT from parsed bits, bind Rigol `SENS?`, `MEAS:RES?`;
   UI: short switch (already bound in 4 definitions), protection clear (BK).
2. Derived signals module (P, R, Ah, Wh, elapsed, min/max) + secondary readout row.
3. Battery test (software with backstop + native on RI/SI/KO), then I-V sweep / internal R,
   then PSU OCP trip and efficiency (two-instrument tests).
4. Contract: ranges, slew, Von, protection levels; then transient; list last (needs DSL 5.4.3).
