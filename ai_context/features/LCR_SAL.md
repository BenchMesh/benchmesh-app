# LCR and SAL: common functional denominator, tiers, software-level value

Research 2026-10-20 (read-only). Sources: contracts `devices/classes/LCR.yaml`, `SAL.yaml`;
definitions `devices/{keysight_e4980a,tonghui_th28xx,bk_894,bk_880,easttester_et44}.yaml`,
`devices/bases/scpi_lcr_e4980.yaml`, `devices/{rigol_dsa800,siglent_ssa3000x,unit_uts,gwinstek_gsp9300,tinysa}.yaml`,
`devices/bases/scpi_sa.yaml`; coverage `ai_context/migration/coverage/LCR.md`, `SAL.md`; UI
`frontend/src/ui/bench/panels/LcrPanel.tsx`, `SalPanel.tsx`, `profiles.ts` (`lcrProfile` l.293,
`salProfile` l.359); manuals in `/home/marek/project/BenchMeshDOC/<family>/doc/` (pdftotext).
Page numbers marked "PDF p" are PDF page indices from pdftotext (they can differ from the printed
page numbers used in the coverage reports and definitions). Layer names (L0 native, L1 class
derivation, L2 server state, L3 flow procedure, UI presentation) follow
`features/SOFTWARE_LAYER.md` in this scratchpad.

---

## Part A - LCR (LCR / impedance meter)

### A1. Feature matrix (8 families)

Legend: Y = remote command exists, fp = front panel only, - = not offered, ? = not verified.
Families: ET = East Tester ET44/ET45; TH = Tonghui TH2810B+/2817B+ (TH28xx); 894 = B&K 894/895
(Tonghui OEM); 880 = B&K 878B/879B/880 handheld; E49 = Keysight E4980A/AL; 891 = B&K 891;
UTR = UNI-T UTR2810E+; AT = Applent AT28xx / GW Instek LCR-6000.

| Feature | ET | TH | 894 | 880 | E49 | 891 | UTR | AT | Source |
|---|---|---|---|---|---|---|---|---|---|
| Pair readout primary + secondary | Y | Y | Y | Y | Y | Y | Y | Y | FETC? in all |
| Cs/Cp/Ls/Lp with D, Q | Y | Y | Y | Y | Y | Y | Y | Y | |
| Rs/Rp/ESR secondary | ESR | Rs,Rp | Rs,Rp | ESR | Rs,Rp | R | Y | Rs,Rp | |
| R-X, Z-θ | Y | Y | Y | Z-θ | Y | Y | Y | Y | |
| G-B, Y-θ, Cp-G | - | Y | Y | - | Y | Y | Y | - | base functions; 891 UM PDF p89 |
| DCR | Y | ? | ? | Y | ? | Y | ? | Y | ET PDF p8 `FUNC:IMP:A ...DCR\|ECAP` |
| SER / PAR | Y | Y | Y | Y | Y | Y | Y | Y | |
| Test frequency | list (ET44) / 10 Hz-100 kHz (ET45) | 50 Hz-100 kHz | 20 Hz-1 MHz | 5 points | 20 Hz-2 MHz | 20 Hz-300 kHz | 4 points | continuous | definitions; 891 PDF p90 |
| Level V rms | list / 10-2000 mV | 0.1-1 V | 5 mV-2 V | 0.3/0.6/1 | 0-2 V | 0.5/1 | 0.1/0.3/1 | Y | |
| Level as current (CURR) | - | ? | Y | - | Y (QCoDeS) | - | - | ? | 894 PM PDF p2 `CURRent` |
| Source resistance 30/100 Ω | Y `OUTP:RES` | ? | Y `ORES` | - | ? | fixed 100 Ω | Y `LEV:SRES` | Y `VOLT:SRES` | ET PDF p6; 894 PM p17; LCR.md §2.5, §2.6 |
| Range manual (ohm list) | AUTO/HOLD | Y | Y | auto | Y | AUTO/HOLD | Y | Y (index) | |
| Speed | 3 | 3 | 3 | - | 3 | 2 | 3 | 3-4 | |
| Averaging count | - | 1-255 (`APER s,n`) | Y | - | Y | - | - | 0-256 | TH UM PDF p100; 894 PM p28; LCR.md §2.6 |
| Open / short correction | AUTO (`CORR:EXEC`) | Y | Y | fp | Y | Y `CAL:OPEN/SHOR` | Y | Y | ET PDF p9; 891 PDF p80 |
| Load correction, spot frequencies | - | Y | Y | - | ? | - | - | ? | TH PDF p105-109; 894 PM p35-37 |
| Cable length compensation | - | Y `CORR:LENG` | Y | - | ? | - | - | ? | TH PDF p105; 894 PM p33 |
| DC bias | 0-1.5 V | - | ±2 V (+bias current) | - | 0/1.5/2 V (±40 V opt.) | - | - | ? | definitions; 894 PM p18-19 |
| Comparator / bins | 3 bins + 2nd (`COMP:TOL`) | Y `COMP` | BIN1-9, AUX, OUT | tolerance 1/5/10/20 % | ? | 9 bins + 2nd + OUT | Y `COMP:MODE ABS\|PER\|SEQ` | Y (BIN in FETC) | ET PDF p10-11; TH PDF p109-110; 894 PM p40-43; 880 UM p53; 891 PDF p80-83 |
| Nominal value | Y | Y | Y | Y (read) | ? | Y | Y | ? | same |
| Bin counting | - | fp page | Y `COMP:BIN:COUN:DATA?` | - | ? | Y + histogram | - | ? | 894 PM p43; 891 PDF p81-82 |
| Deviation ABS / % from reference | relative ON/OFF | Y `FUNC:DEV<n>:MODE ABS\|PERC`, `:REF`, `:REF:FILL` | Y | REL fp | ? | TOL/ABS bin mode | - | Y (monitor ABS/PER) | ET PDF p7; TH PDF p95-96; 894 PM p22-23 |
| Native statistics MIN/MAX/AVG | Y `FUNC:COM` | - | - | Y `CALC:REC` | - | - | - | - | ET PDF p9; 880 UM p53-54 |
| Vm / Im monitor | - | fp | Y `MON:VAC`, `SMON:IAC` | - | ? | - | - | Y `FUNC:MON VAC\|IAC` | 894 PM p22; LCR.md §2.6 |
| List sweep (freq/level/bias points) | fp (7 points) | Y `LIST:FREQ/VOLT/DEL/BAND` | Y (+`LIST:BIAS`) | - | ? | 300-point lin/log sweep `SWEEp` | - | ? | TH PDF p96-100; 894 PM p24-27; 891 PDF p84-86 |
| Trigger source + single | INT/MAN/EXT + `*TRG` | INT/EXT/BUS/HOLD | Y | - | Y | manual | INT/BUS/MAN/EXT | Y | |

Missing manuals: E4980A only through the QCoDeS driver (comparator, list, load correction are in
the real E4980 tree but not verified here); AT28xx only ManualsLib summaries; 889B, Voltcraft
4080 and ES51919 handhelds (DE-5000 etc., one-way stream) are left out of the matrix.

### A2. Tiers

**Core (in all 8 families) - 7 features:** primary + secondary readout; function groups L, C, R/Z
with D/Q/θ/ESR-or-Rs/X secondaries; SER/PAR; test frequency (list or continuous); test level;
auto range; open/short correction (remote in 7 of 8, 880 front panel only).

**Common (4-7 families) - 9 features:** manual range or HOLD; speed; trigger source + single
trigger; comparator with nominal + tolerance -> bin; deviation / relative mode; averaging count;
G-B / Y-θ pairs; DCR; source resistance 30/100 Ω.

**Advanced (1-3 families) - 9 features:** DC bias (ET, 894, E49); load and spot correction;
cable length; list sweep / built-in frequency sweep (TH, 894, 891); bin counting / histogram;
Vm/Im monitor; current-driven level; native MIN/MAX/AVG statistics; bias current.

Contract coverage today (`devices/classes/LCR.yaml`): all Core; Common: range, speed,
trigger source / trigger, `BIN` status (unbound), nothing for comparator setup, deviation,
averaging, source resistance; Advanced: bias / bias_output only.

### A3. Software-level value (LCR)

**Key fact:** every non-DCR pair (Cs-D, Lp-Q, Z-θ, R-X, G-B, ...) at a known test frequency fully
describes the complex impedance Z = Rs + jXs of the DUT. So from **any** pair + FREQ the
software can compute **every** other parameter. The meter does the same internally; a meter that
offers only Cs-D (BK 880 has no Rs, no Z-θ when in C mode; UTR, ET44 lack G-B) gains all others.

Step 1 - normalise to Z (ω = 2πf):

| Pair | Rs | Xs |
|---|---|---|
| Cs-D / Cs-Q (D = 1/Q) | D / (ωCs) | -1 / (ωCs) |
| Cs-Rs / Cs-ESR | Rs | -1 / (ωCs) |
| Ls-D / Ls-Q | ωLs / Q | ωLs |
| Ls-Rs | Rs | ωLs |
| R-X | R | X |
| Z-θ | \|Z\| cos θ | \|Z\| sin θ (θ in rad, convert from °) |
| Parallel pairs (Cp-D, Cp-Rp, Cp-G, Lp-Q, Lp-Rp, Lp-G, G-B, Y-θ) | via Y = Gp + jBp: Cp: Bp = ωCp; Lp: Bp = -1/(ωLp); Gp = D·\|Bp\| or 1/Rp or G; then Z = 1/Y | |

Step 2 - every parameter from Z: |Z| = sqrt(Rs² + Xs²); θ = atan2(Xs, Rs); ESR = Rs;
D = Rs/|Xs|; Q = 1/D; Cs = -1/(ωXs) (Xs < 0); Ls = Xs/ω (Xs > 0); Y = 1/Z, Gp = Re Y,
Bp = Im Y; Rp = 1/Gp; Cp = Bp/ω; Lp = -1/(ωBp). Series <-> parallel shortcuts:
Cp = Cs/(1+D²), Rp = Rs(1+Q²), Lp = Ls(1+1/Q²); D and Q are the same in both models.
Also useful: "dominant kind" (sign of Xs: capacitive / inductive), so an auto-LCR display can
show C or L whatever the meter is set to (useful near self-resonance where θ crosses 0).

Today the UI already does a small subset as display only (`LcrPanel.tsx` `derived()`: Q = 1/D,
ESR for Cs-D, Rs for Ls-Q); it is not recorded, not in automation, not in MCP.

| Software feature | Layer | How | Accuracy / timing | When not to emulate |
|---|---|---|---|---|
| All derived parameters (Z, θ, Rs/ESR, Xs, Cs, Cp, Ls, Lp, D, Q, G, B) | L1 | formulas above from PRIMARY, SECONDARY, FUNC, FREQ, one status update | exact algebra; error propagation matters: ESR from Cs-D is D·Xs, so with D = 0.001 and a D accuracy of ±0.0005 (891 UM PDF p93 example is ±0.5 % of value) the derived ESR is ±50 %. Rp from a tiny D explodes the same way | do not show derived Rs/ESR/Rp when D < ~10x the D accuracy; prefer the native ESR pair when the meter has one. DCR has no derivation. Never send derived values |
| Function-consistency guard | L1 | derive only when FUNC and FREQ belong to the same reading | E4980 base caches `func_q`/`freq_q` 2.5 s (`scpi_lcr_e4980.yaml`): after a front-panel change, PRIMARY can carry the new function for up to 2.5 s with the old FUNC | suppress derived keys for one poll cycle after FUNC/FREQ change |
| Deviation Δ and % from a nominal (REL) | L2 | Δ = x - nom; % = 100·(x - nom)/nom; "fill" = take the current reading as nominal | exact; nominal held on the server so recordings/automation see `DEV_PCT` | if the meter has native DEV (TH, 894) the result is the same; native only matters for the meter's own comparator |
| Tolerance bins / component sorting | L2 | nominal + symmetric or asymmetric limits per bin (BIN1..n), secondary limit (D max / Q min), OUT; counts per bin; optional E-series snap (E6/E12/E24/E96 nearest value + error %) | human pace (one part per 1-3 s); readings 2.5-20/s (UTR 3 / 6.25 / 20 per s, LCR.md §2.5) | not for handler-driven production sorting (no handler I/O, no deterministic latency); the instrument's comparator and handler port stay native |
| Stability detection + auto-capture | L2 | rolling window of N readings, relative std dev < threshold (e.g. 0.1 %) -> "STABLE", capture one value; OVERLOAD / no-data -> "part removed", re-arm | needs >= 5 readings: 0.3-2 s at FAST/MED | none: purely additive. Turns any meter into "auto hold" like DE-5000 handhelds |
| Statistics MIN/MAX/AVG/σ/N | L2 | server-side per channel (UI `useStats` today) | exact over polled readings; polled readings may skip instrument readings | native `FUNC:COM` (ET), `CALC:REC` (880) count every reading; software counts polled ones only |
| Averaging | L2 | mean of N fresh readings | reduces noise √N; adds N x reading time latency | prefer native `APER s,n` (TH, 894, AT): averages at the ADC rate with no transfer overhead |
| Frequency sweep, \|Z\|(f), θ(f), self-resonance | L3 | flow: for f in list (log spaced in the setter's min..max, or `features.frequency.list` for list-only meters): set `frequency`, wait for a reading newer than the set, read PRIMARY/SECONDARY, derive Z; SRF = f where Xs crosses 0 (θ crosses 0°), interpolate linearly between the two bracketing points; also capacitor ESR minimum, inductor Q peak | per point: set + settle (one or two measurement periods, autorange adds more) + poll (400 ms) -> ~0.5-1.5 s/point; 50 points ~ 0.5-1 min. Read-back confirmation within 6 s already in the executor | list-only meters (ET44 16 points max, 880 5, UTR 4) give a coarse curve only: show as points, not a line. Where native sweep exists (891 300 points `SWEEp:DATA:ALL?`, TH/894 `LIST`) a definition op is faster and atomic |
| Level sweep, bias sweep (C vs V of MLCCs) | L3 | same flow over `level` or `bias` | bias sweep energises: needs `arm`; cleanup sets bias 0 / OFF (executor already does this) | |
| Temperature / time drift | existing recordings | record PRIMARY and derived keys at 1 s | recordings only take numbers (`RECORDINGS.md`), so derived keys must exist as status keys (L1), not as UI | |
| Open/short correction reminder | UI | remember the last correction time + frequency per instrument; warn when FREQ changes on meters with spot-only correction | | |

### A4. Gap analysis (LCR)

**Bindings per definition** (operations of 16 in the contract / status keys of 13):

| Definition | Ops | Status | Unbound but in the manual |
|---|---|---|---|
| `keysight_e4980a` | 14 | 11 (+BIAS, BIAS_ON) | comparator, DEV, list, averaging count, current level (QCoDeS has it) |
| `tonghui_th28xx` | 12 | 9 | `COMP` (PDF p109-110), `FUNC:DEV<n>` (p95-96), `LIST:*` (p96-100), `APER s,n` avg (p100), `CORR:LOAD/SPOT/LENG` (p105-109), bin from FETC field 4 |
| `bk_894` | 14 | 11 | `COMP:*` + `COMP:BIN:COUN:DATA?` (PM p40-43), `FUNC:DEV<n>` (p22-23), `LIST:*` (p24-27), `MON:VAC`/`SMON:IAC` (p22), `ORES` (p17), `BIAS:CURR` (p19), `CURR` level (p16) |
| `bk_880` | 7 | 8 | `CALC:TOL:STAT/RANG/NOM?/VALU?` (UM p53), `CALC:REC:MAX?/MIN?/AVER?` (p53-54), bin field of FETC |
| `easttester_et44` | 13 | 10 | `COMP:STAT/TOL:NOM/TOL:RANG/TOL:BIN?` (PDF p10-11), `FUNC:DEV:MODE` (p7), `FUNC:COM`/`COMValue?` (p9), `OUTP:RES` (p6), `ECAP` / `AUTO` primary (p8) |

- `BIN` (contract status) is bound by **0 of 5** definitions although FETC already returns it on
  TH/894 (field 4, `0` out, `+1..+9`, `+10` AUX, LCR.md §2.3) and 880 (field 3).
- `REMOTE` status only on 880; `BIAS_ON` only E49/894 (ET44 bias is applied by the value itself).
- Families researched but **without a definition**: UTR2810E+, Applent AT28xx / LCR-6000, BK 891,
  BK 889B, Voltcraft 4080 (ES51919 blocked by the DSL: binary one-way stream).

**UI vs tiers** (`LcrPanel.tsx`): Core complete (pair readout, function keys group -> secondary
-> SER/PAR, frequency chips or editor + presets, level, range chips, correction with prompt).
Common: speed, trigger source + Trigger button, BIN annunciator (never lit, no binding), REL Δ and
HOLD are browser-only, statistics browser-only, derived Q/D/ESR/Rs browser-only and partial.
Missing: comparator setup, deviation %, averaging, source resistance, any sweep or chart, sorting.

### A5. Proposal (LCR)

**Contract additions** (`devices/classes/LCR.yaml`):
- `derive` (L1, computed for every LCR from PRIMARY/SECONDARY/FUNC/FREQ, suppressed where the
  definition binds the key natively): `Z`, `THETA` (deg), `RS` (= ESR), `XS`, `CS`, `CP`, `LS`,
  `LP`, `D`, `Q`, `G`, `B`, `KIND` (C|L|R). Each declared in `status:` with type `si` / `number`.
  Null for DCR, OVERLOAD, missing FREQ, or when the result is not physical (Cs for Xs > 0).
- New optional status: `VM`, `IM` (si, monitors), `DEV` / `DEV_PCT` (si/number, native or L2),
  `BIN` stays (bind it).
- New optional operations: `averaging` (integer 1..n), `source_resistance` (enum),
  `deviation_mode` (OFF|ABS|PCT) + `deviation_reference` (si) + `deviation_fill`,
  `comparator` (ON|OFF) + `comparator_nominal` (si) + `comparator_limits` (a string list
  `1L,1H,...` in %), `bin_counts` (get, list). Keep list sweep out of the contract: emulate in L3,
  add a native `sweep` op only when a definition (891, TH) needs speed.
- L2 server signals per LCR channel: `STATS` (min/max/avg/σ/N), `REL`, `SORT` (nominal,
  tolerance, bins, counts, last result), `STABLE` flag + captured value. All recordable numbers.

**Panel UX sketch** (one panel, tabs shown only when useful):
- **Measure** (today) + a "Component" grid under the readouts: |Z|, θ, ESR/Rs, X, Cs/Cp, Ls/Lp,
  D, Q, each with a small `calc` tag (tooltip: formula + accuracy note); a primary-from-any
  toggle ("show as C/L automatically").
- **Sort**: nominal (with E-series snap and part-type hint), tolerance chips 0.1/1/2/5/10/20 %,
  optional bins table, D-max / Q-min; a large PASS / FAIL / BINn annunciator; Auto-capture when
  stable; counts per bin; Export CSV. Native comparator is used when bound (setter + `BIN`),
  software otherwise; the label says which.
- **Sweep**: parameter (frequency / level / bias), start/stop/points/log or the frequency list,
  plot |Z| + θ (or C + D) vs f, SRF / Q-peak marker; Run = a built-in flow (arm only for bias).

**DSL gaps:** (1) the class-level `derive` section (shared with SOFTWARE_LAYER.md L1);
(2) CEL helpers `sqrt`, `atan2`, `pow`, `log10`, `pi` (none are listed in DEVICE_DSL.md "CEL
environment", which provides only `num()`); (3) an impedance helper is advisable so 20 FUNC tokens
do not become one unreadable CEL expression, e.g. `zs(FUNC, PRIMARY, SECONDARY, FREQ)` -> map
`{rs, xs}` in `bm_driver::expr`, then each derived key is a one-liner; (4) list values for
`bin_counts` (contract type `list` exists); (5) nothing new for BIN: `{split}` + `map` suffice.

---

## Part B - SAL (spectrum analyzer)

### B1. Feature matrix (9 families)

Families: tSA = tinySA / Ultra (shell); DSA = Rigol DSA800; SSA = Siglent SSA3000X (+Plus);
UTS = UNI-T UTS1000B/3000B; GSP = GW Instek GSP-9300; N93 = Keysight N9320B / N9322C;
5000 = Siglent SSA5000A; RSA = Rigol RSA3000; HSA = Hantek HSA2000 (user manual only).
Page refs: DSA = DSA800 Programming Guide PDF p; SSA = PG0703X E04A PDF p; UTS = UTS PM V1.0 PDF p;
GSP = GSP-9300 PM VB PDF p; N93 = N9320B / N9322C programmer's guides PDF p.

| Feature | tSA | DSA | SSA | UTS | GSP | N93 | 5000 | RSA | HSA | Refs |
|---|---|---|---|---|---|---|---|---|---|---|
| Center/span/start/stop, full/zero span | Y | Y | Y | Y | Y | Y | Y | Y | Y | |
| RBW (+auto) | Y (+auto) | Y | Y | Y | Y | Y | Y | Y | Y | |
| VBW (+auto) | - | Y | Y | Y | Y | Y | Y | Y | Y | |
| Ref level, dB/div | Y | Y | Y | Y | Y | Y | Y | Y | Y | |
| Ref level offset (ext. gain/loss) | Y (fw `levelOffset`, ?) | Y p63 | Y p28 | Y p52 | Y p69 | Y | Y | Y | Y p53 | |
| Attenuation (+auto) | Y (+auto) | Y | Y | Y | Y | Y | Y | Y | Y | |
| Preamp | Ultra LNA fp | Y p123 | Y | Y | option | Y | Y | Y | ? | |
| Detector POS/NEG/SAMP/NORM/AVER/RMS/QP | - (QP via calc ?) | Y (+QP, EMI filter p121) | Y | Y (no RMS/QP) | Y (no RMS/QP) | Y | Y | Y | ? | |
| Trace modes write/max/min/avg/view | fw `calc` ? | Y | Y | Y | Y (set only) | Y | Y | Y | Y | |
| Number of traces | 1-4 | 3 | 4 | <n> | 4 | | | 6 | | DSA p18; SSA p37 |
| Average count | ? | Y p208 | Y p37 | Y p76 | Y p71 | Y | Y | Y | Y | |
| Markers (count) | 4 | 4 (`MARKer<n>`) | 4 | <n> | <n> | Y | Y | Y | Y | |
| Delta marker | fw ? | Y p39 | Y p41 | Y p37 | Y p66 | Y p58 | Y | Y | Y p50 | |
| Noise marker (dBm/Hz) | - | Y `FUNC NOISe\|NDB` p37 | Y p52 | Y p33 | Y p67 | N9322C p103 | Y | Y | - | |
| Frequency counter | - | Y p24 | Y p52 | Y p27 | Y p67 | Y p76/p102 | Y | Y | Y p50 | |
| Peak table / marker table | - | Y p208 | Y p46-50 | Y p29-31 | Y p67 | p84 | Y | Y | - | |
| Channel power / ACPR / OBW | - | Y p58, p72 | Y p58+ | Y p44-47 | ACPR, OCBW p66-68 | Y | Y | Y | - | |
| TOI / harmonics / CNR | - | Y p72 | TOI | Y p46-48 | Y p66-67 | TOI | Y | Y | TOI ? | |
| Limit lines / mask test | - | Y `LLINe` p24-33 | Y p54-58 | Y p20-27 | Y `LIMit` p67 | Y | Y | - | - | |
| Units dBm/dBmV/dBuV/V/W | Y (`trace dBm\|dBmV\|dBuV\|V\|W`) | Y p221 | Y p28 | Y p126 | Y p75 | Y p54 | Y | ? | Y p62 | |
| Trace math (A-B, offset) | - | Y p208 | Y p35 | Y | Y p68 | Y | Y | Y | - | |
| Tracking generator / generator | generator (not offered) | -TG | TG models | T models | option | option | ? | ? | ? | |
| TG normalisation | - | ? | ? | ? | Y `CALC:NORM` p68 | ? | | | | |
| Amplitude correction tables | - | Y `CORR:CSET` p121 | ? | ? | ? | ? | | | | |
| Spectrogram / waterfall | Ultra fp | - | -R model | p48 ? | Y p69 | N9322C p77 ? | Y | Y real-time | - | |
| AM/FM demod (audio) | - | Y p121 | Y | Y | Y | Y | Y | Y | Y | |
| Single sweep + sweep status | `waitscan` | Y + `STAT:OPER` | Y | Y | Y + `STAT:OPER` | Y | Y | Y | | |
| Screenshot | `capture` (RGB565) | `HCOPy` p87 | `HCOP:SDUMp:DATA?` p21 | Y | Y | Y | Y | Y | | |

`?` entries are firmware features of the tinySA not seen in the coverage report's command table
(`calc`, `marker n delta`, `levelOffset`): check against the firmware source before binding.
RF Explorer (streamed binary) is excluded: the DSL cannot read it (SAL.md §2.6).

### B2. Tiers

**Core (all 9) - 8 features:** frequency axis (center/span/start/stop, full span), RBW,
ref level + scale, attenuation, trace read, marker 1 with peak search, sweep continuous/hold,
TRACE_UNIT (dBm).

**Common (6-8 of 9, i.e. every SCPI bench SA, rarely the tinySA shell) - 16 features:** VBW,
auto RBW/VBW/attenuation, detector, trace modes max/min/average, average count, multiple traces,
multiple markers, delta marker, noise marker, frequency counter, peak table, channel power /
ACPR / OBW, limit lines with pass/fail, units dBm/dBmV/dBuV/V/W, ref level offset, preamp,
single sweep with completion status, sweep time / points.

**Advanced (1-5 of 9) - 8 features:** TOI / harmonics / CNR, trace math, TG and TG normalisation,
amplitude correction tables, spectrogram / real time, EMI filters + QP, demodulation, screenshot.

Contract coverage today (`devices/classes/SAL.yaml`): Core complete; Common: VBW, autos,
detector, trace mode, preamp, sweep time, points, sweep mode (but one trace, one marker, no
delta, no noise marker, no counter, no average count, no units setter, no offset, no measurements,
no limits); Advanced: TG output + level, screenshot (declared, bound by none).

### B3. Software-level value (SAL)

Everything below works from what the contract already delivers on every SA: `TRACE` (levels in
TRACE_UNIT), `START`, `STOP`, `RBW`, optional `DETECTOR`, `VBW`, `POINTS`. Frequency of point i:
f_i = START + i·(STOP - START)/(N - 1) (contract). Bin width Δf = (STOP - START)/(N - 1).

| Software feature | Layer | How | Accuracy / timing | When not to emulate |
|---|---|---|---|---|
| Peak, next peak, peak table, N markers, delta markers | L1 (peak) / UI (markers) | local maxima with excursion (e.g. 6 dB) and threshold; marker = trace index; delta = level and frequency difference to a reference marker; parabolic interpolation over 3 points refines peak f and level | frequency resolution Δf (e.g. 10 MHz span / 450 points = 22 kHz on tinySA); interpolation improves to ~Δf/10 for a clean peak; level scalloping up to ~1 dB when Δf > RBW | not a frequency counter: native `FCOunt` (DSA, SSA, UTS, GSP, N93) resolves Hz. Label software marker frequency "± Δf/2" |
| `PEAK_FREQ`, `PEAK_LEVEL`, `NOISE_FLOOR` as status keys | L1 | max / argmax / median of TRACE | per trace update (poll 500 ms) | none: makes the trace **recordable** (recordings take numbers only): oscillator drift over warm-up, RF level vs time |
| Noise marker (dBm/Hz) | UI/L2 | L_n = mean of linear power of k points around the marker, back to dB, then - 10·log10(ENBW), ENBW ≈ 1.06·RBW (Gaussian RBW filter); add +2.51 dB when DETECTOR is SAMP/NORM and the trace is log-averaged (log-average under-reads noise) | ±1 dB typical when DETECTOR is SAMP/RMS/AVER; with POS detector noise reads several dB high | do not show when DETECTOR = POS/QP (flag "detector not suitable") or when the marker sits on a CW signal. Native noise marker knows the exact filter shape |
| Max hold / min hold / average over traces | L2 (UI ghost today) | per point max/min; average in linear power (power average) or in dB (video average), N selectable; restart on START/STOP/RBW change (UI uses `${start}:${stop}` key today) | trace poll is 500 ms but sweeps can be faster or slower: a slow sweep is read twice (duplicates bias the average), a fast one is under-sampled | for exact averaging use SINGLE sweep + completion (`SWEEPING` / `STAT:OPER` bit 3) in an L3 loop, or native `TRACE_MODE AVERAGE` + count where bound |
| Channel power, OBW, ACPR, CNR | L2 / L3 | P_ch = Σ_i in band 10^(L_i/10) · (Δf / ENBW) mW, back to dBm; OBW: cumulative linear power, frequencies where it reaches 0.5 % and 99.5 % (99 % OBW) or -26 dB points; ACPR = P_adj / P_main; CNR = carrier peak - noise density x BW | ±1 dB if RBW ≤ channel BW / 20, Δf ≤ RBW and DETECTOR = RMS/AVER/SAMP; worse with POS detector (overestimates noise-like signals by up to ~10 dB on wideband modulated signals) | where native CHP/ACP/OBW exists (DSA, SSA, UTS, GSP, N93) prefer it: RMS detector and integration bandwidth handled by the instrument. For tinySA (no native) the software version is the only one |
| Harmonics, TOI, spur search | L3 | flow: find fundamental f0 (peak), then for n = 2..k set center = n·f0, narrow span, read peak; TOI from two tones and the 3rd-order products: IP3 = P_tone + ΔdB/2 | one sweep per harmonic (0.1-2 s each) | native HARM/TOI where bound (DSA, UTS, GSP); software risks measuring the analyzer's own distortion: warn when the fundamental is above ref level - 10 dB or attenuation is low |
| Unit conversion dBm / dBmV / dBuV / V / W | UI (display) + L1 for marker values | 50 Ω: dBuV = dBm + 106.99, dBmV = dBm + 46.99, V rms = sqrt(R·10^(dBm/10)/1000), W = 10^(dBm/10)/1000; 75 Ω: +108.75 / +48.75 | exact | set the native `UNIT:POW` only when the user wants the instrument's own screen in that unit; the stored trace and the contract stay dBm (`TRACE_UNIT`) |
| External gain / attenuator / cable / antenna factor | L1 | level offset (single dB) or per-frequency correction table interpolated on f_i | exact as the table | native ref offset (`RLEV:OFFS`, 5 families) applies to the instrument's screen too; a table (DSA `CORR:CSET`) natively only on DSA |
| Limit lines / masks, pass/fail | L2 | piecewise linear upper/lower lines in absolute frequency or relative to center / marker / peak; FAIL when any point violates; failing points, margin; counts per trace | per polled trace; misses a violation that appears only between polls (unless max hold is used for the test) | native `LLINe` (DSA, SSA, UTS, GSP, N93) tests every sweep; software works on all (tinySA) and feeds automation `check` and recordings |
| Reference trace, trace math, TG normalisation | L2 / UI | store a trace (same START/STOP/N) as reference; show A-B in dB; with TG on: store the through-connection trace, then display S21 ≈ trace - reference | ±0.5 dB repeatability of the TG; invalid when START/STOP/N/RBW/ref changes (discard) | prefer native normalisation where bound (GSP `CALC:NORM`); never apply to a different axis |
| Waterfall / spectrogram | UI (history) + optional L2 buffer | ring of the last M traces with timestamps, heatmap (colour = level) | time resolution = trace poll (0.5-3 s; tinySA settings poll 3 s, trace 500 ms); 450 points x 4 B x 1000 traces = 1.8 MB | not real time: misses bursts shorter than the gap between traces. RSA3000 / SSA3000X-R real-time modes are a different instrument class of capability |
| Stitched wide / fine sweeps | L3 | split a wide span into k sub-spans with the instrument's max points, concatenate | k sweeps + settle | only for the tinySA (450 points) and similar; bench SAs have 601-10001 points |
| Trace export | UI + API | CSV (f_i, level, unit, RBW, VBW, detector, timestamp), PNG of the screen; optional "snapshot" saved with a recording | | the native screenshot (`HCOPy`) is the instrument's own picture; both are useful |

### B4. Gap analysis (SAL)

**Bindings per definition** (operations of 27 / status keys of 26):

| Definition | Ops | Status | Unbound but in the manual |
|---|---|---|---|
| `rigol_dsa800` (base `scpi_sa`) | 25 | 25 | preamp `:SENS:POW:RF:GAIN` (p123; `features.preamp: false` although the command exists), markers 2-4, delta `MARK:MODE` (p39), noise `MARK:FUNC NOIS` (p37), `MARK:FCO` (p24), `UNIT:POW` (p221), `RLEV:OFFS` (p63), `TRAC:AVER:COUN` (p208), `LLINe` (p24-33), `CONF/FETC:CHP/ACP/OBW/TOI/HARM/CNR` (p58, p72), `CORR:CSET` (p121), `HCOPy` screenshot (p87) |
| `siglent_ssa3000x` | 25 | 25 | `AVER:TRAC:COUN` (p37), markers 2-4 + delta (p41), `MARK:FUNC` noise (p52), `FCO` (p52), peak table (p49-50), `LLINe` (p54-58), ACPR / channel power (p58+), `UNIT:POW` (p28), screenshot `HCOP:SDUMp:DATA?` (p21) |
| `unit_uts` | 26 | 25 | `LLINe` (p20-27), `FCO` (p27-28), peak/marker table (p29-31), `MARK:FUNC` noise/band (p33-35), delta (p37), CHP/ACP/OBW/HARM/TOI (p44-48), `UNIT:POW` (p126), avg count (p76) |
| `gwinstek_gsp9300` | 24 | 24 | `CALC:ACPR`, `CNR`, `HARM`, `OCBW` (p66-68), `CALC:LIMit` (p67), `MARK:NOIS`, `FCO` (p67), peak table (p67), `CALC:NORM` (p68), `UNIT:POW` (p75), avg count (p71), trace mode readback missing (set only) |
| `tinysa` | 16 | 16 | rbw/attenuate/reflevel `auto`, `waitscan` (single sweep), `sweep cw` (zero span), `trigger`, `capture` screenshot (SAL.md §2.1); fw `calc` / delta markers to verify |

- `screenshot` (contract op) is bound by **0 of 5**; `TG_OUTPUT` is polled on every SCPI family
  but only offered where `features.tg` exists (by design).
- Families researched without a definition: Rigol RSA3000/5000, Siglent SSA5000A, Keysight N9320B
  / N9322C / FieldFox, R&S FPC/FPH, Hantek HSA2000 / OWON XSA, NanoVNA / LiteVNA (VNA class).

**UI vs tiers** (`SalPanel.tsx`): Core complete (screen, annotation bar, FREQ/BW/AMPT/SWEEP
groups, marker 1 placed by click, PEAK/NEXT/LEFT/RIGHT/MKR->CF). Common: detector, trace mode,
RBW/VBW steppers, preamp, auto switches. Browser-only: max-hold ghost (`useMaxHold`), peak index.
Missing: markers 2..n, delta, noise marker, counter, unit selector, offset, any measurement,
limits, averaging count, reference trace / normalisation, waterfall, export, screenshot.

### B5. Proposal (SAL)

**Contract additions** (`devices/classes/SAL.yaml`):
- `derive` (L1, any SAL with TRACE): `PEAK_FREQ` (si), `PEAK_LEVEL` (number), `NOISE_FLOOR`
  (number, median of TRACE), `TRACE_FREQS` not needed (implicit axis).
- New optional status / operations (native where bound, else hidden): `avg_count` (integer),
  `unit` (enum dBm|dBmV|dBuV|V|W; when bound the definition must convert TRACE back to dBm or set
  `TRACE_UNIT`), `ref_offset` (number dB), `marker_mode` (POS|DELTA|OFF), `marker_function`
  (OFF|NOISE|NDB), `counter` (ON|OFF) + `COUNTER_FREQ` (si), and measurement read-outs
  `CHANNEL_POWER` (number dBm), `OBW` (si), `ACPR_LOWER/UPPER` (number dB) with a `measure`
  op (OFF|CHP|ACP|OBW). Keep multi-marker and limit masks in L2 (server, per channel) so every
  SA gets them identically and automation/MCP can `check` them; bind native markers 2..4 only
  if a later user need appears.
- L2 server signals per SAL: trace processing (max/min hold, linear average N, reference trace),
  software markers (list of {freq, mode, ref}), mask test (`LIMIT_FAIL` bool, `LIMIT_MARGIN`
  number), channel-power band config + result. All numeric results recordable.
- L3 built-in flows: harmonics, TOI, stitched sweep, synchronous N-sweep average (SINGLE +
  `SWEEPING` wait).

**Panel UX sketch:**
- Screen: up to 4 marker diamonds (M1 native, M2-M4 software, delta shown as ΔM2), mask polyline
  with red failing points, reference trace (dim), max/min ghost; overlay band for channel power.
- Annotation bar gains `UNIT` and `OFFS`, and a PASS / FAIL annunciator when a mask is active.
- Groups: MKR gets a marker table (n, mode POS/Δ/NOISE, freq, level, unit), "Peak table"
  toggle; new **MEAS** group (Channel power / OBW / ACPR / Harmonics, band editor, native or
  `calc` tag); **TRACE** gets software average N (power / video), Reference store / clear /
  subtract, Normalise (TG only); **LIMIT** group (mask editor table, relative / absolute, beep);
  **VIEW**: waterfall toggle, unit and impedance (50/75 Ω), Export CSV / PNG, Screenshot (native).
- Every software result carries the `calc` / `SW` provenance tag with its precondition
  (detector, RBW vs Δf) shown as a warning when not met.

**DSL gaps:** (1) class `derive` with CEL list functions (`max`, `min`, `argmax`, `median`,
`sum`, list comprehension over TRACE) and `log10`, `pow`: none exist (DEVICE_DSL.md lists only
`num()`); (2) recording a `list` (TRACE) is not possible (`RECORDINGS.md`: list -> no sample),
so trace snapshots need a separate store or only scalar keys are recorded; (3) multi-instance
operations (`marker_freq` for marker n) would need a parameter beyond the single value (today a
POST binds one value; channel `{ch}` could double as marker index but that misuses channels);
(4) `HCOPy` / `capture` screenshots: SCPI block is expressible (`reply: block`), tinySA RGB565
raw frame without header needs a fixed-length binary read (`max_bytes` exact length) plus a
pixel-format conversion step; (5) measurement modes that change the instrument's screen (`CONF:CHP`)
need "enter / leave mode" steps: model as `measure` setter with `only:` steps (already supported).

---

## Summary of what the software can add to every instrument (both classes)

| Class | Free for every instrument (L1/L2) | Emulated procedures (L3) | Keep native |
|---|---|---|---|
| LCR | all 13 derived impedance parameters from any pair; deviation %; tolerance bins + E-series ID; stability auto-capture; server statistics; recordable derived keys | frequency / level / bias sweep with \|Z\|, θ plots and SRF | comparator + handler, averaging at ADC rate, load correction, fast list sweep (891 / TH / 894) |
| SAL | peak/noise floor as recordable keys; N software markers, delta, noise marker; unit conversion; offsets and correction tables; max/min/average; reference trace and TG normalisation; limit masks; channel power / OBW / ACPR from the trace; waterfall; export | harmonics, TOI, stitched sweeps, synchronous averaging | frequency counter, RMS-detector measurements, real-time spectrogram, EMI QP |
