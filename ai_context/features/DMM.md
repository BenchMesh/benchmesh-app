# DMM: common functional denominator and software-level value

Research for BenchMesh (develop @ 2fd8e06), read-only. Scope: class `DMM`, contract
`devices/classes/DMM.yaml`, 20 definitions binding `DMM` (`grep -l '^  DMM:' devices/*.yaml`),
base `devices/bases/scpi_dmm.yaml`, coverage `ai_context/migration/coverage/DMM.md`, UI
`frontend/src/ui/bench/panels/DmmPanel.tsx` + `dmmProfile()` in `frontend/src/ui/bench/profiles.ts`.
Manual citations: `DOC/<family>/doc/<file>` = `/home/marek/project/BenchMeshDOC/...`, page =
**PDF page** of the local file (pdftotext form-feed count), not the printed number.

Manual short names used below:
XDM1 = `owon_xdm/doc/XDM1000_Digital_Multimeter_Programming_Manual.pdf` (XDM1041/1241/2041);
XDM3 = `owon_xdm/doc/XDM_Series_Digital_Multimeter_Programming_Manual.pdf` (XDM3041/3051);
UT88 = `unit_ut880x/doc/UT8805E_Series_Programming_Manual_unitrend.pdf`;
HDM = `hantek_hdm3000/doc/HDM3000_SCPI_Programming_Reference_EN_Ver1.03.pdf`;
GDM83 = `gwinstek_gdm8300/doc/GDM-8342_User_Manual_EN_Ver_D_20230823.pdf`;
DM30 = `rigol_dm3000/doc/DM3058_ProgrammingGuide_EN_2021-11.pdf`;
DM858 = `rigol_dm858/doc/DM858_ProgrammingGuide_EN.pdf`;
SDM = `siglent_sdm3000/doc/SDM3065X_Remote_Manual_RC06036-E01A.pdf` (SDM4000A manual has the same command set);
GDM90 = `gwinstek_gdm9000/doc/GDM-906X_User_Manual_EN_Ver_G_20240318.pdf`;
K2000 = `keithley_2000/doc/2000-900_J-Aug2010_User.pdf`;
K2110 = `keithley_2110/doc/2110-901-01C_Aug2013_Ref.pdf` (K2100 = `keithley_2100/doc/2100-900-01F_Aug_2018_user.pdf`);
34401 = `agilent_34401a/doc/HP_34401A_User.pdf` (Tek DMM4040/50 = `tektronix_dmm4000/doc/DMM4040_DMM4050_Programmer_077036300.pdf`, same dialect);
TV = `keysight_34400/doc/Truevolt_Operating_Service_Guide.pdf`;
6500 = `keithley_dmm6500/doc/DMM6500-901-01C_April_2026_Ref.pdf`;
FLK = `fluke_28x/doc/Fluke_187_189_89IV_87IV_remote_interface.pdf` (+ libsigrok for 287/289, per coverage §3.8).

---

## 1. Feature matrix (15 families, cheap to pro)

Legend: **Y** = remote command (page), **fp** = front panel only, **-** = not available,
**?** = not found in the manual text, **ro** = reported but not settable, **p** = some models only.
Columns ordered by price: XDM1 (OWON XDM1041/1241, ~130 EUR), XDM3 (XDM3051), UT88 (UNI-T UT8805E/06E),
HDM (Hantek HDM3055/65), GDM83 (GW GDM-8341/42/51), DM30 (Rigol DM3058/68), DM858 (Rigol DM858/E),
SDM (Siglent SDM3045X/55/65X, SDM4055A/65A), GDM90 (GW GDM-9060/61), K2000 (Keithley 2000), K21
(Keithley 2100/2110), 34401 (HP 34401A, Tek DMM4040/50), TV (Keysight 34460A..34470A), 6500
(Keithley DMM6500), FLK (Fluke 187/189/287/289 handheld, IR link).

### 1.1 Functions

| Function | XDM1 | XDM3 | UT88 | HDM | GDM83 | DM30 | DM858 | SDM | GDM90 | K2000 | K21 | 34401 | TV | 6500 | FLK | n/15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| DCV, ACV | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| DCI, ACI | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| 2W ohm | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| 4W ohm | p (1241/2041) | Y | Y | Y | p (8351) | Y | Y | Y | Y | Y | Y | Y | Y | Y | - | 14 |
| Continuity | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| Diode | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| Capacitance | Y | Y | Y | Y | Y | Y | Y | Y | Y | - | Y | - | Y | Y | ro | 13 |
| Frequency | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ro | 15 |
| Period | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | - | 14 |
| Temperature | Y | Y | Y | Y | p (TC, not 8341) | p (3068, fp) | Y | Y | Y | Y (TC) | Y | p (4050) | Y | Y | p (289) | 13 |
| AC+DC (true RMS of both) | - | - | - | - | Y `CONF:VOLT:DCAC` | - | - | - | - | - | - | - | - | - | p (289 `V_AC_PLUS_DC`) | 2 |
| DCV ratio | - | - | - | Y (p20) | - | - | - | - | - | - | - | Y (34401 p105) | Y (TV p314) | ? | - | 3 |

Sources for functions: coverage §3 per family (range tables with pages), definitions' `features.modes`.

### 1.2 Measurement control

| Control | XDM1 | XDM3 | UT88 | HDM | GDM83 | DM30 | DM858 | SDM | GDM90 | K2000 | K21 | 34401 | TV | 6500 | FLK | n/15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Auto / manual range | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | p (189 `SF 17` key) | 15 |
| Speed / NPLC | Y F/M/L | Y F/M/L | Y 3 rates | Y NPLC | Y S/M/F | Y F/M/S | Y NPLC 0.4/5/20 | Y NPLC | Y NPLC | Y NPLC | Y NPLC | Y NPLC | Y NPLC | Y NPLC | - | 14 |
| Resolution / digits (separate from speed) | - | - | - | Y RES (p27) | - | - | - | - | - | Y `:DIG 4..7` (p142) | ? | Y CONF res (p105) | Y RES (p332) | ? | - | 4 |
| Auto zero | - | - | - | Y (p26) | - | Y (p149) | - | Y `:AZ` (p62, p73) | Y (p301) | Y `:AZER` (p148) | Y (K2110 p81) | Y (p60; DMM4040 p64) | Y (p334, p359) | ? | - | 9 |
| Input impedance 10M / >10G | - | Y (p18) | Y (p43) | Y (p24) | ? | Y (p60) | ? | Y (p84) | Y (p299) | fixed | Y (K2100 p57) | Y (p54; DMM4040 p64) | Y (p383) | Y (p557) | - | 10 |
| AC filter / bandwidth (3/20/200 Hz) | - | - | - | Y (p20) | - | - | - | Y (p62, p85) | Y (p301) | ? | Y (K2110 p131) | Y (p52; DMM4040 p45) | Y (p322, p375) | Y (p556) | - | 7 |
| DC / digital averaging filter | - | Y `VOLT:FILT` (p17) | - | - | fp (FILT) | - | - | - | - | Y `AVER:TCON` (p177) | - | p (DMM4040 `FILT` p61) | Y `CALC:SMO` (p235) | Y (p547) | - | 5 |
| Continuity threshold | Y (p13) | Y (p18) | Y (p45) | ? | Y (p129) | ? | ? | Y (p91) | Y (p297) | Y (p145) | ? | fixed 10 ohm | fixed 10 ohm | ? | - | 7 |
| Trigger source / single / sample count | - | - | Y (p6, p49) | ? | fp | Y `TRIG:SOUR AUTO/SINGLE/EXT` (p88) | Y (p22) | Y (p5-14, p98) | Y | Y (p75) | Y (K2110 p73-76) | Y (p73-78) | Y (p307) | Y | - | 11 |
| Reading memory / buffer fetch | - | - | Y `R?` (p5, p23) | ? | - | ? | Y `R?` (p20, p45) | Y `R?` (p12, p46) | Y (p273-287) | Y `TRAC` (p148) | Y (K2110 p80) | Y (p85) | Y (p206, p257) | Y `TRAC` (p626-656) | p (289 logging) | 10 |
| Secondary display / 2nd function | Y `MEAS2?` (p26) | Y `MEAS2?` (p46) | - | Y `:SEC` (p20-47) | Y `VAL2?` (p135), `CONF2` (p120) | Y `FUNC2` (p46-50) | Y `:SEC` (p81-101) | - | ? | - | p (2110 `FUNC2` p71) | p (DMM4040 `FUNC2` p57) | Y `:SEC` (p320-381) | ? | ro (dual LCD) | 9 |
| Reading hold | fp | fp | fp | fp | Y `CALC:FUNC HOLD` (p131) | Y `TRIG:AUTO:HOLD` + sensitivity (p89) | fp | fp | Y `CALC:HOLD:REF` (p276) | fp | Y (K2110 p109) | fp | fp | fp | Y `SF 11/21` | 5 |
| Beeper on/off `SYST:BEEP:STAT` | Y (p22) | Y (p39) | Y (p46) | ? | Y (p137) | Y (p11) | Y (p13) | Y (p92) | Y | ? | Y (K2110 p41) | Y (p89) | Y (p394) | ? | - | 11 |

### 1.3 Math (instrument side)

| Math | XDM1 | XDM3 | UT88 | HDM | GDM83 | DM30 | DM858 | SDM | GDM90 | K2000 | K21 | 34401 | TV | 6500 | FLK | n/15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| NULL / REL offset | Y `CALC:NULL:OFFS` (p21) | Y per fn `:NULL` (p17-27) | ? | Y `:NULL:VAL[:AUTO]` (p20-22) | Y `CALC:REL:REF`, `NULL:OFFS` (p132-134) | Y `CALC:REL` (p33-34) | Y `:NULL:STAT` (p78) | Y `:NULL` (p57-65) | Y `CALC:FUNC REF` (p276) | Y `:REF` (p174) | Y (K2110 p95) | Y (p67) | Y (p316-363) | Y `:REL` (p566-569) | Y `SF 13` | 14 |
| dB / dBm with reference R | Y 17 refs 50..8000 ohm (p19-20) | Y (p35-36) | Y `CALC:SCAL` (p12-13) | - | Y (p133) | Y (p34-37) | Y (p31-32) | Y (p28-32) | Y (p278) | Y `UNIT:VOLT:AC:DB/DBM:IMP` (p206) | Y (K2110 p100-101) | Y (p69) | Y (p223-224) | Y (p552-553) | ro (289 unit DBM) | 14 |
| Min / max / avg / count | Y (p17-19) | Y (p33-35) | Y (p15-16) | ? | p min/max (p131) | Y (p31-32) | Y (p24-27) | Y (p33-35) | Y (p276-277) | Y `CALC2` (p153) | Y (K2110 p94) | Y (p66) | Y (p230) | Y `TRAC:STAT` (p324) | Y `SF 12` | 14 |
| Std deviation / peak-peak | - | - | Y SDEV (p16) | ? | - | - | Y SDEV (p27) | Y SDEV, PTP (p33-35) | Y SDEV, PTP (p277) | Y SDEV (p153) | - | - | Y SDEV, PTP (p230) | Y STDD, PK2P (p324) | - | 7 |
| Limits pass/fail | - | Y `CALC:LIM`, `FAIL?` (p36-37) | Y (p8-9) | ? | Y COMP (p132-133) | Y `CALC:PF` (p37-39) | Y (p28-31) | Y (p17-20) | Y + beeper mode (p277-278) | Y `CALC3` (p139, p155) | Y (K2110 p97) | Y (p71) | Y (p214-217) | Y `CALC2:LIM` (p470-478) | - | 12 |
| mx+b scaling | - | - | - | - | Y `CALC:MATH:MMF/MBF` (p133-134) | - | - | - | Y MXB (p276) | Y `KMAT` + units (p151) | Y (K2110 p98, K2100 p76) | - | Y `CALC:SCAL:GAIN/OFFS` (p225-226) | Y (p481-483) | - | 6 |
| Percent of target | - | - | - | - | Y (p134) | - | - | - | - | Y (p152) | Y (K2110 p93) | - | Y PCT (p224) | Y (p484) | - | 5 |
| Histogram / trend chart | - | - | Y (p10-12) | - | - | - | - | Y (p21-27) | Y `DISP:VIEW` (p289) | - | - | - | Y (p218, p237, p261) | Y (buffer graph) | - | 5 |

Observation: the same math (NULL, dB/dBm, statistics, limits) exists in 12-14 of 15 families, but in
**at least seven syntaxes** (`CALC:FUNC NULL|DB|DBM|AVER|LIM` 34401/XDM/U3606; per-function
`<fn>:NULL[:STAT]` Truevolt/SDM/HDM/DM858; `CALC:SCAL:FUNC DB|DBM` Truevolt/SDM/UT88; `:CALC:REL/DB/DBM/PF`
Rigol DM3000; `CALC:FUNC OFF|MIN|MAX|HOLD|REL|COMP|DB|DBM|MXB|INV|REF` GW; Keithley 2000 `CALC1/2/3`;
DMM6500 per-function `:REL` / `CALC2:<fn>:LIM`), with different reference sets (OWON 17 fixed
impedances, others any value) and different side effects (the math result replaces the
reading returned by `READ?` on several dialects - to verify per family before binding).

---

## 2. Tiers

Counted over the 15 families of §1 (n/15).

### Core (13-15/15) - 13 items; the panel must always offer them
DCV, ACV, DCI, ACI, 2W ohm, continuity, diode, frequency (15); period, 4W ohm (14);
capacitance, temperature (13); auto/manual range (15); speed/NPLC (14); reading with overload
indication (15). Plus, as **math everybody has**: NULL/REL (14), min/max/avg (14), dB/dBm (14).
Core math is only "core" for the user if we do it in software, because each meter spells it differently.

### Common (7-12/15) - 9 items; show when bound
Limits pass/fail (12), trigger single/continuous + sample count (11), reading buffer (10),
input impedance 10M / >10G (10), auto zero (9), secondary display (9), std deviation / peak-peak (7),
AC filter (7), continuity threshold (7). Beeper (11) is common but only a setting.

### Advanced (2-6/15) - 7 items
mx+b (6), percent (5), averaging/smoothing filter (5), histogram/trend (5), reading hold
remote (5), resolution digits separate from speed (4), ratio (3), AC+DC (2).

---

## 3. Software-level value (what BenchMesh can give every DMM)

All the following need only `MEAS`, `MODE`, `RANGE` (bound by all 20 definitions), so one
implementation serves every meter, including the handhelds of coverage §3.18 once they are readable.

| Feature | Formula / rule | Kind | Value even when the meter has it |
|---|---|---|---|
| REL / null | `y = x - x0`, `x0` captured (one reading or mean of N) or typed | per reading | same behaviour on all meters; kept per function; visible to recordings, flows, MCP |
| Statistics | n, min, max, mean, std dev (Welford), peak-peak, since reset or over a sliding window | aggregate | 8 families have no sdev/p-p; works also for FREQ/TEMP/CAP where some meters refuse |
| dB / dBm / dBV / dBu | dBm = `10 log10(V^2 / R / 1 mW)`, R any value (50, 75, 600, user); dBV = `20 log10(V / 1 V)`; dBu = dBm at 600 ohm; dB = `dBm - ref` | per reading | any R instead of OWON's 17 fixed values; HDM3000 (no dB) and handhelds get it |
| Limits | LOW <= y <= HIGH -> PASS / HIGH / LOW, counters, beep/flash in UI, flow `check` | per reading | one syntax; automation can stop a run on FAIL |
| mx+b scaling | `y = m*x + b`, unit label (e.g. shunt 0.01 ohm: m = 100 -> A; 4-20 mA transmitter -> bar; probe 1:10) | per reading | 9 of 15 families lack it |
| Sensor conversion | thermocouple mV -> degC (NIST polynomial + cold-junction value), NTC Steinhart-Hart from ohm, RTD Callendar-Van Dusen | per reading | temperature for meters without a TEMP function (34401A, K2000 has TC only) |
| Percent deviation | `100 * (x - nominal) / nominal`, with tolerance -> limits | per reading | component sorting (resistors, capacitors) on any meter |
| Hold / auto-hold / peak hold | hold on demand; auto-hold when N readings within a band (DM3058 p89 style); max-hold / min-hold | per reading + state | uniform; auto-hold for handheld-style probing on bench meters |
| Period <-> frequency | `T = 1/f`, `f = 1/T` | per reading | meters with only Hz (Fluke, OWON SPM DMM part) |
| Temperature unit | C / F / K conversion | per reading | meters without `UNIT:TEMP`; fixes the label (see 4.3) |
| Trend | sparkline / chart of the last N readings; rate of change (`dx/dt`, e.g. K/min for thermal settling, V/s for capacitor leakage) | aggregate | everywhere (today only the recorder) |
| Averaging filter | moving average or exponential (`y += (x - y)/N`), reset on step > band (like Keithley `AVER:TCON MOV/REP` p177) | aggregate | smoothing for noisy readings on meters without filters |
| Settled indicator | "stable" when sdev over window < k * resolution | aggregate | tells the user/flow when to take the value |
| Cross-instrument math | P = V(DMM A) * I(DMM B or PSU IOUT); R = V/I (4-wire with two meters); efficiency = Pout/Pin across PSU, load and meters | bench-level | no instrument can do it |

### 3.1 Sampling limits (where the math must run, and what it can claim)

Facts: the `scpi_dmm` base polls `[meas, func, rng_q, auto_q]` every 500 ms (`scpi_dmm.yaml`
`poll`), `owon_xdm` 300 ms (meter refreshes about 1/s at rate S, `owon_xdm.yaml` meas description),
`rigol_dm3000` 500 ms. `READ?` blocks for the integration time (10 NPLC about 0.2 s, AC about 1 s on
older meters, 100 NPLC 2 s at 50 Hz; `meas.timeout_ms: 6000`). `/ws/registry` pushes a full snapshot
every 200 ms whether or not anything changed (`crates/bm-server/src/lib.rs::registry_stream`,
`BM_WS_INTERVAL` default 0.2 s), recordings sample once per second (`RECORDINGS.md`). Meters
convert much faster: DM3058 123/20/2.5 rdg/s (coverage §3.4), GDM-8351 up to 320 rdg/s, 34461A
1 k rdg/s, 34465A/70A 50 k rdg/s (coverage §3.7).

Consequences:
1. **Per-reading math** (REL, mx+b, dB/dBm, %, sensor conversion, limits, unit, 1/x) is exact at
   any poll rate: it is a pure function of each reading. It should run on the server where the
   reading arrives (registry apply), so recordings, flows, MCP and every browser see the same value.
2. **Aggregates** (min/max/avg/sdev/p-p, peak hold, filters, trend) computed in software are
   statistics of the *polled sub-sample* (at most 2-3 readings/s). Mean of a steady signal is
   fine; min/max/peak miss transients shorter than the poll interval; sdev reflects reading
   noise only when the poll interval is longer than the conversion time (otherwise readings may repeat).
   Each aggregate must be labelled with its sample count and rate ("N 120 @ 2/s").
3. Each reading must be counted **once**: a sample identity (sequence or timestamp of the poll
   result) is needed. Today there is none in the status (see 4.3, bug B1).
4. For true statistics of every conversion, use the instrument: either its own `CALC:AVER` (14/15
   have min/max/avg) read on demand, or buffer mode (`SAMP:COUN n` + `INIT` + `R?`/`DATA:REM?`/`TRAC:DATA?`,
   10/15) fetched as a `list` and fed into the same software statistics. Software stats are the
   default; instrument stats are an accuracy upgrade where bound.
5. Cross-instrument values combine readings taken at different instants (skew up to one poll
   interval of each device). Combine only values fresh within a window (the registry has
   `fresh_value(max_age)`, `crates/bm-server/src/devices.rs`) and show the skew; never extrapolate.
6. Instrument math left ON (front panel) changes what `READ?` returns; software math on top would
   double-apply. Bind a read of the meter's math state where cheap (e.g. `CALC:STAT?`) and show it.

---

## 4. Gap analysis

### 4.1 Contract (`devices/classes/DMM.yaml`)
- Status: `MEAS` (si, required), `MODE` (required), `RANGE`, `OVERLOAD`. Nothing for a secondary
  reading, the speed, impedance, auto zero, math state, temperature unit, or sample identity.
- Operations: `mode` (set, required), `range`, `measurement`, `function`, `temp_sensor`,
  `temp_scale`, `rate`. No impedance, auto zero, AC filter, continuity threshold, trigger/single,
  sample count, buffer, secondary function, math, beeper.
- `mode.set` is required: read-only meters (Fluke 28x, Keysight U1250, all chip-protocol
  handhelds, coverage §4) cannot be bound.
- No vocabulary: `MODE` values are "a features.modes key" chosen by each definition; `rate`
  is described as "F/M/S(L)" while the base uses NPLC numbers (`0.02`..`100`), Rigol DM3000 uses
  F/M/S, OWON F/M/L, GW S/M/F.

### 4.2 Definitions (20 files, `extends` resolved)

| Operation / status | Bound by | Missing |
|---|---|---|
| `mode`, `range`, `measurement`, `function` | 20/20 | - |
| `rate` | 18/20 | `keysight_u3606`, `owon_spm` |
| `temp_scale` | 16/20 | `agilent_34401a`, `keysight_u3606`, `owon_spm`, `rigol_dm3000` |
| `temp_sensor` | 14/20 | also `unit_ut8805e`, `unit_ut8806e` |
| status `OVERLOAD` | 18/20 | `owon_xdm` (remote OL form undocumented), `owon_spm` |
| status `RANGE` | 19/20 | `owon_spm` |
| anything from §1.2 rows 3-13 or §1.3 | **0/20** | all |

Mode-key drift (one class, four spellings): base/17 defs `VOLT`, `CURR`, `FRES`; `owon_spm`
`VOLT_DC`, `CURR_DC`; `owon_xdm` `FRESistance`; `gwinstek_gdm8300` adds `VOLT_DCAC`,
`CURR_DCAC`; `keithley_2110` `TEMP_TC`. The UI copes by prefix matching (`dmmRank`, `modeGlyph`),
but a flow `set mode VOLT` is not portable to the SPM, and recordings of "VOLT" vs "VOLT_DC" differ.
Range values: SI numbers in 18 defs, **range indices** in `rigol_dm3000` (`"0".."6"`, RANGE
status `string(int(rng_q))`).

Commands in the manuals not bound (all families with SCPI): every row of §1.3 (NULL, dB/dBm,
CALC:AVER, limits, mx+b, %), §1.2 impedance (10 families), auto zero (9), AC filter (7), continuity
threshold (7), trigger/sample count (11), buffer `R?`/`DATA:POIN?` (10), secondary
`MEAS2?`/`VAL2?`/`FUNC2`/`:SEC` (9), `DATA:LAST?` with unit (SDM p7-11, DM858 p20-22), `SYST:BEEP`.
`owon_xdm` already carries `features."DB:REFerence"`/`"DBM:REFerence"`, `delta`, `lock`
and `owon_spm` `hold`, `delta`: metadata without operations (legacy UI keys, unused).

### 4.3 UI (`DmmPanel.tsx`, `dmmProfile()`) vs tiers
Covered: Core functions (dial from `features.modes`), range chips, rate selector, temp
sensor/scale (when bound), bargraph, OL annunciator; client-side Hold, REL and MIN/MAX/AVG/N.
Missing: dB/dBm, limits, mx+b, %, sdev/p-p, secondary display, impedance, auto zero, AC filter,
continuity threshold, single trigger, trend, settled indicator.

Defects found (code facts):
- **B1 statistics count WebSocket pushes, not readings.** `useStats(value, tick=status, key)` adds
  a sample whenever the status object changes identity; the registry is re-sent every 200 ms
  (`registry_stream` ignores `Registry.version`, whose comment says senders skip unchanged
  snapshots), the meter is polled every 300-500 ms. N is inflated about 2.5x (base) to 5x (OWON at
  rate S), AVG is time-weighted, an sdev added the same way would be wrong.
- **B2 bargraph full scale wrong for index ranges.** `fullScale(range)` takes `Number(RANGE)`;
  `rigol_dm3000` reports the index (`"2"` = 20 V) so the bar uses 2 V full scale.
- **B3 digits fixed.** `dmmProfile` returns `counts: 5` for every meter (4 1/2 to 7 1/2 digit meters).
- **B4 temperature label.** The unit comes from `features.modes.TEMP.unit` (`°C` base, `K` OWON);
  after `temp_scale F` the meter returns degF (`UNIT:TEMP`) under a `°C` label.
- **B5 rate not tracked.** `rate` is read once when the meter comes online (`api.get('rate')`);
  a front-panel change is not seen (not a status key).
- **B6 client-only math.** Hold, REL and statistics live in React state: reset on reload or
  detach (`PanelWindow`), differ between the app window and remote browsers, are not recordable
  (`Recordable field="MEAS"` records the raw reading), not usable by flows or MCP. Same pattern as
  the PSU panel's `vout * iout` fallback (`PsuPanel.tsx:19`).

### 4.4 DSL gaps relevant to DMM
1. No stateful per-reading processing (REL, stats, filters) in definitions - correct, it belongs
   in the server layer (`features/SOFTWARE_LAYER.md` 1.1), not in each definition.
2. No sample identity in `StatusUpdate` (`bm_driver::actor` emits status without timestamp; the
   registry keeps `status_at` but does not export it).
3. Buffer readings: an operation can return `list`, but a poll cannot feed many readings per
   cycle into status; a `samples` list key (or event) is needed for instrument-rate statistics.
4. Read-only binding (`mode.set` optional or `capabilities: read_only`) and the handheld link
   features (stream rx, HID bridges, binary frames, segment decode, ack/prompt lines) - coverage §4.
5. Long conversions block the actor: with 100 NPLC + AC the poll occupies the link for seconds and
   setters wait. A continuous-trigger + `FETC?`/`DATA:LAST?` pattern (non-blocking) is expressible
   with `session.on_connect` but is not used by the base.

---

## 5. Proposal

### 5.1 Canonical vocabularies (contract-level, validator-enforced)
- **MODE** (enum in `DMM.yaml`): `VOLT`, `VOLT_AC`, `VOLT_ACDC`, `CURR`, `CURR_AC`, `CURR_ACDC`,
  `RES`, `FRES`, `CONT`, `DIOD`, `CAP`, `FREQ`, `PER`, `TEMP`, `RATIO` (DC = the bare SCPI short
  form, as `FUNC?` reports it; 17 of 20 definitions already use these). Rename in `owon_spm`
  (`VOLT_DC`, `CURR_DC`), `owon_xdm` (`FRESistance`), `gwinstek_gdm8300` (`*_DCAC` -> `*_ACDC`),
  `keithley_2110` (`TEMP_TC` -> `TEMP` + sensor). No compatibility aliases.
- **RANGE**: SI full-scale value as a decimal string, or `AUTO`. Index meters map index <-> SI in
  the definition (enum params already allow it); `features.RANGe[MODE][].value` follows.
- **rate**: definition values stay, but each `features.rate[]` entry carries
  `{value, display, speed: FAST|MEDIUM|SLOW, nplc?, rdg_s?}`; flows and UI use `speed`, the
  value text stays the meter's own.
- **impedance**: `10M`, `10G` (= >10 Gohm on low ranges), `AUTO`. **autozero**: `ON`, `OFF`, `ONCE`.
  **ac_filter**: `SLOW` (3 Hz), `MEDIUM` (20 Hz), `FAST` (200 Hz). **trigger**: `CONT`, `SINGLE`.
- **LIMIT** result: `PASS`, `LOW`, `HIGH`.

### 5.2 Contract additions (`devices/classes/DMM.yaml`)
```yaml
status:
  MEAS2:     {type: si, description: "Secondary display reading (MODE2), null when off"}
  MODE2:     {type: string, description: "Secondary function (MODE vocabulary) or null"}
  RATE:      {type: string, description: "Active features.rate value"}
  UNIT:      {type: string, description: "Unit of MEAS when not the mode's base unit (TEMP C/F/K)"}
  IMPEDANCE: {type: string, description: "10M | 10G | AUTO (DCV)"}
  AUTOZERO:  {type: string, description: "ON | OFF | ONCE"}
  METER_MATH:{type: bool, description: "The meter's own math is on: MEAS is a math result"}
operations:
  mode:          {set: true, value: string}            # no longer required (read-only meters)
  impedance:     {get: true, set: true, value: string}
  autozero:      {get: true, set: true, value: string}
  ac_filter:     {get: true, set: true, value: string}
  cont_threshold:{get: true, set: true, value: si}
  secondary:     {get: true, set: true, value: string} # OFF or a features.secondary[MODE] key
  measurement2:  {get: true, value: si}
  trigger:       {set: true, value: string}            # CONT | SINGLE
  trigger_now:   {set: true, value: string}            # momentary: one reading in SINGLE
  readings:      {get: true, value: list, description: "Buffered readings (R?/DATA:REM?/TRAC:DATA?)"}
  meter_stats:   {get: true, value: list, description: "Optional: the meter's min,max,mean,sdev,count"}
  beeper:        {set: true, value: string}
```
Instrument NULL/dB/limits/mx+b are deliberately **not** contract operations: one software
implementation (5.3) covers 20/20 definitions; binding seven math dialects buys only the
"every conversion" accuracy that `readings`/`meter_stats` already give.

**features hints** (per model): `counts` (e.g. 60000, 220000, 2200000) or `digits` (4.5..7.5);
`rate[]` with `speed/nplc/rdg_s`; `impedance: [10M, 10G, AUTO]`; `secondary: {VOLT_AC: [FREQ, VOLT],
CURR: [CURR_AC], FREQ: [PER, VOLT_AC], ...}` (DM858 p81-101, TV p320-381, DM3058 p46-50);
`cont_threshold: {min, max}` (e.g. SDM `CONT:THR:VAL` p91); `dbm_references: [50, 75, ...]` only
for meters where the list matters; rename the legacy `DB:REFerence`/`DBM:REFerence`/`delta`/`hold`/
`lock` keys or drop them.

### 5.3 Server software layer for DMM (one module, cross-class pattern)
A processor at the registry apply choke point (see `SOFTWARE_LAYER.md`), per device/class/channel:
- input: each *new* poll result (sequence number from the actor; fixes B1 by construction);
- pipeline per reading: `x` -> REL (`x - x0`) -> scale (`m*x + b`, unit) or sensor conversion ->
  transform (none | dBm(R) | dBV | dBu | dB(ref) | % (nominal) | 1/x) -> LIMIT -> hold/auto-hold;
- aggregates over the pipeline output: n, min, max, mean, sdev, p-p (Welford), window or since reset,
  moving/exponential filter, settled flag, rate of change; reset on MODE/RANGE change (REL and
  stats per MODE, as the panel does today);
- outputs as status keys with provenance: `MATH` (si, the processed value), `MATH_UNIT`,
  `STAT_N`, `STAT_MIN`, `STAT_MAX`, `STAT_MEAN`, `STAT_SDEV`, `STAT_PP`, `STAT_RATE` (readings/s),
  `LIMIT`, `LIMIT_FAILS`, `HELD`, `SETTLED`; marked `source: software` in `/instruments/{CLS}/{id}`
  so the UI, recordings, flows (`check` on `LIMIT`) and MCP can tell computed from measured;
- settings as class-independent operations (`math/rel`, `math/scale`, `math/transform`,
  `math/limits`, `math/hold`, `math/stats/reset`) stored with the device (data dir, not in
  memory per browser); `arm`/energize rules do not apply (nothing touches the instrument).
- bench-level derived channels (separate small module): named expressions over fresh values of
  several devices (`P = dmm1.MEAS * psu1.IOUT`), freshness window, skew shown, recordable.

### 5.4 Panel UX sketch (one DmmPanel for every meter)
```
+--------------------------------------------------------------------------+
| [dial V- V~ A- A~ ohm ohm4W ))) ->| C Hz PER deg]   OWON XDM1241  [_][^] |
|  +---------------------------------------------------------------------+ |
|  | V-        12.3456 V              AUTO  REL  dBm600  HOLD  OL  SLOW  | |
|  | MATH  -14.21 dBm   (raw 12.3456 V, REL 0.0012 V)           [PASS]   | |
|  | 2nd   1.0002 kHz  (MEAS2, only with features.secondary)             | |
|  | bargraph 0 ................................................. 20 V   | |
|  | MIN 12.3401 MAX 12.3502 AVG 12.3449 SDEV 2.1m P-P 10.1m  N 120 @2/s | |
|  | sparkline (last 60 s)                         settled: yes          | |
|  +---------------------------------------------------------------------+ |
|  Range [AUTO 200m 2 20 200 1000]   Rate [FAST MED SLOW]   [Single] [Hold] |
|  Math  [REL capture | type] [Scale m b unit] [dB/dBm R: 600] [%] [Limits] |
|  Input [10M | 10G]  AutoZero [ON OFF ONCE]  AC filter [3 20 200 Hz]       |
|  Stats [reset] [window 60 s]  Continuity threshold [10 ohm]  (when bound) |
+--------------------------------------------------------------------------+
```
Rules: math row always present (software, every meter); measurement-setting rows appear only
when the operation is bound; the readout precision follows `features.counts`; the bargraph uses
SI RANGE; the stats line shows N and rate so the user sees what the statistics are based on;
LIMIT FAIL turns the readout red and can beep in the UI; secondary line only with `MODE2`.

### 5.5 Suggested order
1. Fix B1-B5 (sample identity in status, SI RANGE for `rigol_dm3000`, `features.counts`, `UNIT`
   status, `RATE` status) and the mode vocabulary renames (small, testable with the bench fixtures).
2. Server software math + stats module and the panel math row (largest user value, 20/20 meters).
3. `impedance`, `autozero`, `secondary` + `measurement2` in `scpi_dmm` (one base edit covers
   Truevolt, HDM, SDM/DM858 dialect variants; OWON XDM3000, DM3000 and GW override).
4. `trigger`/`trigger_now`/`readings` (buffer statistics at instrument rate), `cont_threshold`,
   `ac_filter`.
5. Read-only binding (`mode` optional) for Fluke 28x and the DSL link features for handhelds.
