# OSC: common functional denominator, software value, gaps and proposal

Class: `OSC` (bench DSO/MSO, USB/LAN SCPI scopes). Research date 2026-10-20, read-only.
Sources: contract `devices/classes/OSC.yaml`; 14 definitions (`grep '^  OSC:' devices/*.yaml`);
bases `devices/bases/scpi_osc.yaml`, `devices/bases/siglent_scope.yaml`; coverage report
`ai_context/migration/coverage/OSC.md` (cited "cov §n"); bench notes
`ai_context/migration/device-notes/rigol_dho800.md` (cited "bench"); UI
`frontend/src/ui/bench/panels/OscPanel.tsx`, `frontend/src/ui/kit/ScopeScreen.tsx`,
`frontend/src/ui/bench/profiles.ts` (`oscProfile`). Manuals: `/home/marek/project/BenchMeshDOC/<family>/doc/`
converted with `pdftotext -layout`; **page numbers are PDF pages** (form-feed count) unless marked
"pr." (printed). Layer names (L0 native, L1 stateless derivation, L2 stateful server, L3 procedure
on the automation executor, UI presentation) follow `features/SOFTWARE_LAYER.md` in this scratchpad.

---

## 1. Feature matrix

Family keys: **D8** Rigol DHO800/900 (bench-tested DHO804), **D1** Rigol DHO1000/4000, **DZ** Rigol
DS1000Z(-E), **M5** Rigol MSO5000, **D2** Rigol DS2000A, **SN** Siglent new tree (SDS800X HD,
SDS1000X HD, SDS2000X Plus, SDS2000X HD; guide EN11G), **SL** Siglent SDS1000X-E (legacy guide
EN02E), **KS** Keysight InfiniiVision 1000X/2000X, **TK** Tektronix TBS1000C, **GW** GW Instek
GDS-1000B, **UT** UNI-T MSO/UPO2000CS, **OW** OWON XDS3000 (no definition, blocked cov §2.10),
**HT** Hantek DSO2000 (no definition, blocked cov §2.11).

Cells: **Y** documented remote command; **o** optional (license, model variant or module);
**-** not in the programming guide; number = count of items. "Def" = definitions (of 14) that
bind it today (bases resolved).

### 1.1 Vertical, horizontal, trigger, acquisition

| Function | D8 | D1 | DZ | M5 | D2 | SN | SL | KS | TK | GW | UT | OW | HT | Def |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Channel on/off | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 14 |
| V/div | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y (string "1v") | Y | 14 |
| Offset | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y (div units) | Y | 14 |
| Coupling DC/AC/GND | Y | Y | Y | Y | Y | Y | Y | DC/AC | DC/AC | Y | Y | Y | Y | 14 |
| Probe ratio | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 14 |
| BW limit | 20M | 20M/250M | 20M | per model | Y | 20M/200M | Y | Y | 20M | Y | Y | ? | Y | 14 |
| Invert | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ? | Y | 14 |
| Input 50 ohm | - | DHO4000 | - | - | Y p101 | o | - | o (3000T) | - | - | - | - | - | 0 |
| Timebase s/div | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 14 |
| Horizontal position | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 14 |
| Run / stop | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | **-** | 14 |
| Single | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | sweep only | 14 |
| Autoset | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | **-** | 14 |
| Force trigger | Y p36 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ? | Y | 0 |
| Edge source/level/slope | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 14 |
| Sweep AUTO/NORM/SING | Y | Y | Y | Y | Y | Y | Y | AUTO/NORM | Y | Y | Y | Y | Y | 14 |
| Trigger status | Y | Y | Y | Y | Y | Y | Y | register | Y | Y | Y | Y | Y | 14 |
| Trigger coupling | Y p323 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | ? | ? | 0 |
| Holdoff | Y p325 | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y | 0 |
| Pulse width trigger | Y | Y | Y | Y | Y | Y | Y | Y (GLITch) | Y | Y | Y | Y | Y | 0 |
| Slope / video / runt | Y | Y | Y | Y | Y | Y | Y (TRSE p191) | Y (TV) | pulse/runt | Y | Y | partial | Y | 0 |
| Serial trigger (UART/I2C/SPI/CAN/LIN) | o | o | o | o | o | o | o | o | - | - | o | - | Y | 0 |
| Trigger types listed | 17 p322 | 17 p300 | 11 p174 | Y | Y p303 | 21+ p485 | 6 | 7 p93 | 2 pr.158 | 7 | 8 p40 | 3 | 12 | 0 (edge) |
| Memory depth | Y p43 | Y p38 | Y p21 | Y | Y p28 | Y p41 | Y (MSIZ p33) | read (POINts) | 1k/2k/20k pr.98 | 1e3-1e7 pr.40 | AUTO..56M p76 | Y (DEPMEM) | Y | 0 |
| Acq normal/peak/average | Y | Y | Y | Y | Y | Y p48 | Y (ACQW p30) | Y p189 | Y pr.22 | Y | Y | Y | Y | 14 |
| High-res / ERES | - | Y (+ULTRa) | Y | Y | Y | ERES p48 | Y | Y | HIRes | - | Y | - | Y | partial |
| Average count | Y p42 | Y | Y p20 | Y | Y | Y (AVER,n) | AVGA p32 | Y | Y | Y | Y | Y | ? | 0 |
| Sample rate (read) | Y | Y | Y | Y | Y | Y | Y | Y | Y | ? | Y | JSON | Y | 13 |
| Zoom / delayed | Y | Y | Y | Y | Y | Y | - | Y | Y | Y | Y | - | Y | 0 |
| Roll mode | Y | Y | Y | Y | Y | Y | - | Y | Y | - | - | - | Y | 0 |
| Segmented / history | - | - | - | - | Y | Y p328 | Y (p87) | Y p184 | - | - | - | - | - | 0 |

### 1.2 Data, analysis and extra instruments

| Function | D8 | D1 | DZ | M5 | D2 | SN | SL | KS | TK | GW | UT | OW | HT | Def |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Screen trace download | Y 1000 pts | Y 1000 | Y 1200 | Y 1000 | Y 1400 | Y (any, defs ~1000) | Y | Y | Y | Y (record) | Y | Y ~1520 | Y (multi-packet) | 14 |
| Full-record download | RAW p427 | RAW | RAW 250k/read | RAW | RAW | POINt/MAXPoint p752 | Y | POINts p633 | CURVe 20k | 1e7 | RAW p80 | - | Y | 0 |
| Formats | BYTE/WORD/ASC | same | same | same | same | BYTE/WORD | DAT2 i8 | BYTE/WORD/ASC | RI i8 | i16be | BYTE | i16le | ? | BYTE (8-bit) |
| Native measurement items | 42 p218 | 42 p196 | 37 p141 | ~42 | ~37 (one query each) | ~71 p344 | ~40 (PACU p116) | ~27 (:MEASure:*) | 29 pr.113 | 36 | ~35 | 31 p19 | 37 p50 | **4** (VPP, VAVG, VRMS, FREQ) |
| Measurement statistics | Y p223 | Y p200 | Y (STATistic) | Y | Y | Y (ADV:STATistics) | PASTAT p121 | :MEASure:RESults? p369 | - | - | Y p31 | - | - | 0 |
| Cursors | Y p109 | Y p112 | Y p32 | Y | Y | Y p85 | Y (CRMS p52) | :MARKer p326 | Y | Y pr.58 | Y | - | Y | 0 |
| Math A+B, A-B, A*B, A/B | Y p183 | Y | Y p106 | Y | Y | Y p323 | DEFINE p93 | Y p303 | MATH:DEFINE | DUAL pr.50 | Y p20 | Y | Y | 0 |
| FFT | Y p192 | Y | Y | Y | Y | Y | Y (FFTW p107) | Y (ch. 15) | Y | Y pr.52 | Y | - | Y | 0 |
| Reference waveforms | Y | Y | Y | Y | Y | Y | - | Y | Y | Y | - | - | - | 0 |
| Mask / pass-fail | Y p172 | Y | Y p123 | Y | Y | :MTESt | PFDD p133 | :MTESt p408 | - | - | Y p72 | - | Y | 0 |
| Serial decode | o (BUS p49) | o | o p53 | o | o | o | - | o (SBUS p457) | - | - | o | - | Y | 0 |
| Frequency counter | Y p104 | Y p108 | Y p132 | Y | Y | Y | - | 1000X p352 | - | - | - | HARDfrequency | - | 0 |
| DVM | Y p139 | Y | - | Y | - | Y | - | o | - | Y pr.145 | Y p70 | - | - | 0 |
| Built-in AWG | DHO900 S p298 | - | DZ-S | o | o | o (WGEN) | o (ext. module) | o (WGEN p661) | - | - | - | - | 2D models (:DDS) | 0 |
| Bode plot | Y (BODeplot) | ? | - | - | - | Y | - | - | - | - | - | - | - | 0 |
| Screenshot | PNG p136 | Y | PNG | BMP | BMP | :PRINt? p34 | SCDP p143 | p234 | **-** | RLE pr.72 | Y p76 | BMP | - | **0** (disabled) |
| ADC bits | 12 | 12 | 8 | 8 | 8 | 12 (Plus 8/10) | 8 | 8 | 8 | 8 | 8 | 12 (XDS) | 8 | - |

Facts behind the less obvious cells:
- DHO800 trigger types (p322): EDGE, PULSe, SLOPe, VIDeo, PATTern, DURation, TIMeout, RUNT, WINDow,
  DELay, SETup, NEDGe, RS232, IIC, SPI, CAN, LIN. Siglent (p485): EDGE, PULSE, SLOPe, INTerval,
  PATTern, RUNT, WINDow, DROPout, VIDeo, QUALified, NEDGe, DELay, SHOLd, IIC, SPI, UART, LIN, CAN,
  FLEXray, CANFd, IIS, M1553, SENT. Keysight 1000X (p93): EDGE, GLITch, PATTern, SHOLd, TRANsition,
  TV, SBUS1. Tek TBS1000C `TRIGger:A:TYPe {EDGe|PULSe}` (pr.158). GW `:TRIGger:TYPe {EDGe|DELay|
  PULSEWidth|VIDeo|RUNT|RISEFall|TIMEOut}`. UNI-T (p40): EDGE, PULSe, VIDeo, SLOPe, DURation, SHOLd,
  NE, PATTern.
- Measurement lists: DHO800 `:MEASure:ITEM <item>` (p218) VMAX VMIN VPP VTOP VBASe VAMP VAVG VRMS
  OVERshoot PREShoot MARea MPARea PERiod FREQuency RTIMe FTIMe PWIDth NWIDth PDUTy NDUTy TVMAX TVMIN
  PSLewrate NSLewrate VUPPer VMID VLOWer VARiance PVRMs PPULses NPULses PEDGes NEDGes RRDelay RFDelay
  FRDelay FFDelay RRPHase RFPHase FRPHase FFPHase ACRMs (42). Statistics per item: `:MEASure:
  STATistic:ITEM? MAXimum|MINimum|AVERages|CNT,...` (p223, examples p438). Siglent
  `:MEASure:ADVanced:P<n>:TYPE` 12 slots, ~71 items incl. STDEV, CRMS, RISE10T90, SKEW, PHA (p344).
  Tek `MEASUrement:IMMed:TYPe` 29 items incl. PK2Pk, RMS, CRMs, PDUty, RISe, PHAse (pr.113).
- Siglent acquisition (p48): `NORMal|PEAK|AVERage[,4..8192]|ERES[,0.5..4.0 bits]`. Tek (pr.22):
  `SAMple|PEAKdetect|HIRes|AVErage`. Keysight (p189): `NORMal|AVERage|HRESolution|PEAK`.
- **Correction to cov §2.11:** the Hantek DSO2000 guide does document measurements:
  `:MEASure:CHANnel<n>:ITEM? <type>` with 37 items, reply prefixed with the item name
  (`VPP 3.600e-01`, p50); also mask, cursors, math/FFT and a `:DDS` generator (`:DDS:TYPE` p58,
  SINE...ARB4). Still no RUN/STOP, autoset or screenshot.

---

## 2. Tiers (common functional denominator)

Rule: **Core** = in at least 12 of the 13 families and needed for "look at a signal";
**Common** = at least 8 of 13 with remote commands; **Advanced** = options, model variants or
heavy UI.

**Core (17)** - per channel: on/off, V/div, offset, coupling, probe; global: timebase, horizontal
position, run/stop, single, autoset, edge source, edge level, edge slope, sweep AUTO/NORM/SING,
trigger status, screen trace with scaling, basic measurements Vpp/Vavg/Vrms/Freq.
All 17 are in the contract and bound by all 14 definitions (Hantek lacks run/stop/autoset).

**Common (22)** - BW limit, invert (both bound, 14/14); force trigger; trigger coupling;
holdoff; pulse-width trigger; slope/video trigger; acquisition type NORM/PEAK/AVER (bound 14/14)
plus HRES/ERES; average count; memory depth; sample rate read (bound 13/14); full-record
download; extended measurements (Vmax, Vmin, Vamp, Vtop, Vbase, period, rise, fall, +width,
-width, +duty, -duty, overshoot, preshoot, delay, phase, counted as one feature group);
measurement statistics; cursors; math A+B/A-B/A*B/A/B; FFT; reference waveforms; mask/pass-fail;
screenshot; zoom.
Bound today: 4 of 22 (BW limit, invert, acquisition type, sample rate).

**Advanced (13)** - runt/window/pattern/setup-hold/N-edge/timeout triggers; serial trigger;
serial decode; frequency counter; DVM; built-in AWG; Bode plot; segmented/history; roll mode;
50 ohm input; digital channels (MSO); ULTRa/phosphor modes; area/slew/edge-count measurements.
Bound today: 0 of 13.

---

## 3. Software-level value (what BenchMesh can add)

The OSC already delivers, every poll, the raw material for most analysis: `WAVE` (codes) with
`WAVE_Y0/DY/X0/DX` (contract), i.e. volts = (code - Y0) * DY, t = X0 + i * DX. So most of the
scope's analysis can be computed once, the same way for every vendor, at zero extra link cost.

### 3.1 Data quality facts that bound every computed value

- **Points:** screen trace 1000 (DHO, MSO5000, Siglent defs), 1200 (DS1000Z), 1400 (DS2000A),
  ~1520 (OWON) (cov §2.1-2.10). The scope's own measurements use the full record (Mpts).
- **Resolution:** definitions read `BYTE`, so even 12-bit DHO/Siglent HD deliver 8 bits; measured
  on the DHO804: 234 codes over 8 div = ~29 codes/div, 1.71 mV/code at 50 mV/div (bench). One code
  = 3.4 % of a division. WORD (`u16le`) would give 12 bits at twice the bytes (cov §2.2).
- **Decimation:** NORMal mode returns the displayed record. Whether it is decimated by plain
  sampling or min/max is not stated in the guides read; with plain sampling, narrow pulses
  and peaks between kept samples are lost and FFT aliases. *Unverified: check on the DHO804 with
  a narrow pulse at slow timebase.*
- **Interpolation:** at fast timebases scopes draw with sin(x)/x; points per screen then exceed
  real samples. Computed rise times below about 3 real sample intervals are interpolation
  artefacts. The real interval is 1/SRATE; screen spacing is DX.
- **Timing (DHO804 USBTMC, bench):** one query round trip 23-29 ms; a 1000-point block ~55 ms.
  Trace group per channel = preamble + data ~80 ms, so 4 channels ~0.35 s per refresh: the 250 ms
  group cannot keep up (inference), giving about 3 trace sets/s. Native measurement: one
  query per item per channel (~25 ms), so 4 items x 4 channels ~0.4 s per 1 s group; 15 items x 4
  channels would be ~1.5 s. Full record: DS1000Z RAW at most 250 000 BYTE points per read, so
  12 Mpts = 48 reads; sigrok waits 1 s per read above 15 k points (cov §2.1): tens of seconds.
- **Coherence:** channels are read one after the other (`:WAV:SOUR CHAN{ch}` then data). While
  the scope runs, CH1 and CH2 traces can come from **different acquisitions**. Anything that
  combines channels (A-B, A*B, phase, delay) is only valid when the scope is stopped (STOP or
  after SINGLE), unless the scope computes it.
- **Clipping:** codes at the ADC rails (0/255 for u8, -128/127 for i8) mean the trace is clipped
  and Vpp/Vmax/RMS are wrong; most scopes do not report clipping over SCPI.

### 3.2 Feature by feature

| Software feature | Layer | Works for | Accuracy | Timing | Do not emulate when |
|---|---|---|---|---|---|
| **Trace measurements** Vmax, Vmin, Vpp, mean, RMS, AC RMS, amplitude (top/base by histogram), overshoot | L1w (server, per trace) | all 14 defs, plus OWON/Hantek once traces work | +-1-2 codes (3-7 % of a div at 8 bit) on Vmax/Vmin/Vpp; noise biases Vpp up; mean/RMS average out quantisation | 0 link cost, < 1 ms CPU for 1000 points | the scope has the item and the user wants its full-record value; the trace is clipped (report null + `CLIP`) |
| **Cycle RMS / mean** (integer periods between crossings) | L1w | all | removes partial-cycle bias; whole-screen RMS of a sine with k cycles errs by up to 1/(8 pi k) (0.4 % at 10 cycles, 2 % at 2) | same | fewer than 1 full period on screen |
| **Frequency / period** by crossings with hysteresis and linear interpolation, averaged over all full periods | L1w | all | timing limited by DX and noise; with 5+ periods on screen better than 0.1 % (inference); bounded by the scope's timebase accuracy | same | a hardware counter exists (D8/D1/DZ/M5/D2/SN/KS 1000X: 6+ digits) or fewer than 2 crossings |
| **Duty, +width, -width** (50 % of top/base, hysteresis) | L1w | all | +-1 sample (DX) per edge, so +-2 DX on a width | same | pulse narrower than ~5 DX |
| **Rise / fall 10-90 %** | L1w | all | needs at least 5 real samples on the edge; result includes the scope's own rise time ~0.35/bandwidth (5 ns at 70 MHz); report `sqrt(t_meas^2 - t_scope^2)` only as a hint | same | edge spans fewer than ~3 real samples (1/SRATE), sin(x)/x region; use the scope's RTIMe |
| **Delay / phase between channels** | L1w | all | +-1 DX; valid only for coherent traces | same | scope running (see coherence); use the scope's RRPHase/SKEW or stop first |
| **Consistent definitions across vendors** | L1w | all | one definition of "top", "RMS", thresholds for every scope on the bench | - | never; label provenance instead |
| **Math A+B, A-B, A*B (power V x I), A/B** | UI for display, L1w for recorded values | all | inherits each channel's quantisation; A-B of two 8-bit channels has poor common-mode rejection (each at its own V/div) | 0 link cost | scope running and the signals are not repetitive/locked to the trigger; the scope's own math is coherent |
| **FFT in the browser** (Hann/flat-top window, dBV) | UI | all | N=1000: bin = 1/(N DX) = 1/screen time, 500 bins to Nyquist 1/(2 DX); 8-bit floor ~50 dB SNR + 27 dB processing gain; aliasing if the screen record is sample-decimated | ~5 ms | spectral purity, spurs, wide dynamic range: use the scope FFT on the full record or an SAL |
| **Cursors** (time, voltage, track on trace: dT, 1/dT, dV per channel) | UI | all | resolution DX and DY of the trace | none | never; read-only, no instrument commands |
| **Statistics over polls** (min/max/mean/sigma/n of any measurement) | L2 | all | counts polls (1/s), not acquisitions; the scope counts every acquisition (thousands) | 1 value/s per poll group | jitter, rare-event statistics: bind the scope's `:MEASure:STATistic` (9 families have it) |
| **Persistence / overlay** (last N traces as ghosts, max/min envelope) | UI | all | ~3 traces/s with 4 channels, so it misses rare glitches | none | glitch hunting: the scope's own persistence/trigger is orders of magnitude faster |
| **Mask / limit test** on traces or measurements | L2 (traces), L2g style alert | all | only the polled traces are tested; events between polls are invisible | ~3 tests/s | real pass/fail on every acquisition: bind `:MASK`/`:MTESt`/`PFDD` (10 families) |
| **Recording measurements** on the shared timeline | exists (recordings v2, `Recordable` in `OscPanel`) | all | whole-second sampler | 1 Hz | - |
| **Trigger-assisted capture** (arm SINGLE, wait TRIG_STATUS = STOP/TD, download full record, store snapshot with the recording, CSV/PNG export) | L3 flow + server op | all with SINGLE (not Hantek) | full record resolution (Mpts, WORD for 12-bit) | seconds to tens of seconds (cov §2.1) | - |
| **Software screenshot** (render the panel's screen + measurements to PNG) | UI | all, incl. TBS1000C with no screenshot | shows BenchMesh's data, not the scope's display | instant | when the user needs the scope's own screen (decode tables, menus) |
| **Software autoset** for scopes without it (Hantek) or a non-destructive one (keeps probe/coupling) | L3 | all | iterates V/div from Vpp/clipping, timebase from freq (2-4 periods) | 3-6 polls | the native autoset exists and the user accepts it resets everything |
| **Clipping / overrange flag** | L1w | all | exact (codes at the rails) | 0 | never |
| **Cross-instrument procedures**: Bode (AWG sweep + OSC Vpp ch1/ch2 + phase), PSU ripple at load steps (ELL + OSC Vpp AC) | L3 flows | any AWG/PSU/ELL with any OSC | as the measurements above, coherent when stopped per point | ~1 s per point | the scope has a built-in Bode with its own AWG (D8 `:BODeplot`, SN) and it is enough |

Where software should never replace native: anything that needs every acquisition (statistics,
masks, persistence, trigger qualification), full-record precision (rise time at fast edges, FFT
dynamic range), decode and hardware counters. Native always wins when bound; computed values
carry provenance `computed` (SOFTWARE_LAYER §2.2).

---

## 4. Gap analysis

### 4.1 Contract (`devices/classes/OSC.yaml`)

Has 25 status keys and 19 operations: the Core tier plus BW limit, invert, acquisition type,
sample rate and screenshot. Missing for Common: trigger type and its parameters, trigger
coupling, holdoff, force, memory depth, average count, full-record waveform, any measurement
beyond VPP/VAVG/VRMS/FREQ, statistics, cursors, math, FFT, mask, reference, zoom. Vocabularies
are free strings for `COUPLING`, `BWLIMIT`, `TRIG_STATUS`, `TRIG_SWEEP`, `TRIG_SOURCE`,
`TRIG_SLOPE`, `ACQ_TYPE` (only descriptions suggest values), and `probe` is `string` while
`PROBE` is `number`. There is no notion of trace coherence, ADC bits, or provenance.

### 4.2 Definitions (14, bases resolved)

| Contract item | Bound | Missing in |
|---|---|---|
| display, scale, offset, coupling, probe, bwlimit, invert, timebase, timebase_offset, trigger_status/sweep/source/slope/level, acquire_type, run, waveform | 14/14 | - |
| sample_rate / SRATE | 13/14 | gwinstek_gds1000b |
| VPP, VAVG, VRMS, FREQ | 14/14 (`features.measurements` = these 4 everywhere) | - |
| screenshot | **0/14** | rigol_dho800 has the command, commented out (kernel usbtmc stall, bench); tektronix_tbs1000c has none in the manual |
| run values | RUN/STOP/SINGLE/AUTO(/CLEAR) | keysight drops CLEAR; SDS1000X-E RUN=`ARM` |

Structure: 6 definitions extend `scpi_osc` (DS1000Z, DS2000A, MSO5000, DHO1000, Keysight, UNI-T),
4 extend `siglent_scope`, 4 stand alone (DHO800, SDS1000X-E, TBS1000C, GDS-1000B). Only rigol_dho800
is bench-tested; all others "awaiting confirmation".

### 4.3 UI (`OscPanel.tsx`, `ScopeScreen.tsx`, `oscProfile`)

Shows: screen with graticule, traces, ground markers, trigger level and position; annunciators
RUN/STOP/T'D/WAIT/sweep/acq type; summary; per-channel legend with the 4 measurements
(recordable); run/stop, single/auto/clear, acquisition type, time/div and H position steppers,
trigger source/slope/sweep/level; per channel on/off, V/div, position, coupling, probe chips,
"BW 20M" switch, invert.
Gaps against the tiers: Core is complete. Common: no cursors, no math/FFT, no statistics, no
measurement chooser (fixed 4 from features), no memory depth/average count, no trigger type,
coupling, holdoff or force, no capture/export/screenshot, no clipping indicator, no persistence.
The BW switch hardcodes `20M` (`OscPanel.tsx`, `set('bwlimit', v ? '20M' : 'OFF')`), so DHO1000
250M or Siglent 200M are unreachable from the UI although the setter's enum offers them. The
legend shows measurements only for channels that are ON (correct) but has no provenance tag.
`ScopeScreen` colours channels 1-4 only (`Math.min(4, ...)`); a math trace needs a fifth token.

### 4.4 Manual commands not bound (highest value first)

1. Measurement items beyond the four: one query each on Rigol/Keysight/UNI-T/Tek; Siglent via 12
   `:MEASure:ADVanced:P<n>:TYPE` slots + value; DHO800 `:MEASure:ITEM <item>,<src>` (p218).
2. Statistics: Rigol `:MEASure:STATistic:ITEM? MAXimum|MINimum|AVERages|CNT,<item>,<src>` (p223),
   Siglent `:MEASure:ADVanced:STATistics`, Keysight `:MEASure:RESults?` (p369), legacy `PASTAT`.
3. Memory depth `:ACQuire:MDEPth` (Rigol p43, Siglent p41), average count `:ACQuire:AVERages`.
4. Trigger type `:TRIGger:MODE` (Rigol p322) / `:TRIGger:TYPE` (Siglent p485) and pulse width
   parameters; trigger coupling (p323), holdoff (p325), force `:TFORce` (p36).
5. Full record `:WAVeform:MODE RAW` + `:STARt/:STOP` chunks (Rigol p427-429), Siglent
   `:WAVeform:POINt/MAXPoint` (p752); WORD format for 12-bit models.
6. Hardware counter `:COUNter` (DHO800 p104) and DVM `:DVM` (p139): cheap, precise, one query.
7. Math `:MATH<n>:OPERator` (p183) and FFT (p192): only to show the scope's own math; the trace
   of MATH1 can be read with `:WAV:SOUR MATH1` like a channel.
8. Mask `:MASK` (p172) / `:MTESt` (Keysight p408) for real pass/fail counts.
9. Screenshot (all but TBS1000C/Hantek): blocked by transport, not by the DSL.
10. Built-in AWG (`:SOURce`, `:WGEN`, `:DDS`): better exposed as a second class binding `AWG` on
    the same device than as OSC operations (the AWG panel already exists).

---

## 5. Proposal

### 5.1 Contract additions (`devices/classes/OSC.yaml`)

Canonical vocabularies (definitions map wire values to these with `map`, as DHO already does):

```yaml
# enums instead of strings
COUPLING:    {type: enum, values: [DC, AC, GND]}
TRIG_SWEEP:  {type: enum, values: [AUTO, NORM, SING]}
TRIG_SLOPE:  {type: enum, values: [POS, NEG, RFAL]}
TRIG_STATUS: {type: enum, values: [TD, WAIT, RUN, AUTO, STOP]}   # Siglent Arm/Ready/Trig'd/Roll, Tek, UNI-T map here
ACQ_TYPE:    {type: enum, values: [NORM, PEAK, AVER, HRES]}       # Siglent ERES, Tek SAMple/PEAKdetect/HIRes, GW PDETect map here
TRIG_TYPE:   {type: enum, values: [EDGE, PULSE, SLOPE, VIDEO, RUNT, WINDOW, TIMEOUT, PATTERN, NEDGE, DELAY, SETUP_HOLD, SERIAL]}
```

New status keys (all optional; per channel unless marked global):

| Key | Type | Source | Note |
|---|---|---|---|
| `VMAX`, `VMIN`, `VAMP`, `VTOP`, `VBASE`, `VACRMS`, `PERIOD`, `RISE`, `FALL`, `PWIDTH`, `NWIDTH`, `PDUTY`, `NDUTY`, `OVERSHOOT`, `PRESHOOT` | number | native (bound) or L1w computed from WAVE | canonical names, mapping table per dialect in the contract description |
| `DELAY`, `PHASE` | number | native or L1w (coherent traces only) | relative to `features.reference_channel` (default CH1) |
| `CLIP` | enum [ON, OFF] | L1w | codes at the rails |
| `WAVE_COHERENT` | bool, global | server | true when all channel traces of this poll were read while TRIG_STATUS = STOP |
| `TRIG_TYPE`, `TRIG_COUPLING`, `TRIG_HOLDOFF` (si), `MDEPTH` (string, e.g. 10M), `ACQ_AVERAGES` (integer) | global | native | Common tier |
| `COUNTER` | number, global | native | hardware counter where present |
| `MEAS_STATS` | later: `<KEY>_MIN/_MAX/_AVG/_N` from L2 | server | follows SOFTWARE_LAYER §2.4 |

New operations: `trigger_type` (enum), `trigger_coupling` (enum DC/AC/LFR/HFR), `trigger_holdoff`
(si), `trigger_force` (set, no value), `memory_depth` (enum from the definition), `acquire_averages`
(integer), `counter` (get), `waveform_full` (get, list; refused with a message unless stopped;
chunked), `capture` (server op: SINGLE, wait, `waveform_full` for every displayed channel, store as
a recording snapshot), `measure_stats_reset` (server). Fix `probe` to `number`.

Computed measurement layer **L1w** (new, server, stateless): a `bm-server` module that, after each
status update with WAVE, computes the canonical keys the binding does not bind natively (native
wins), using Y0/DY/X0/DX, with hysteresis 3 codes and thresholds 10/50/90 % of top/base.
Rationale for a module instead of CEL: CEL has no list arithmetic helpers today (`num` is the only
custom function, DEVICE_DSL "CEL environment"), and the results must be identical in recordings,
MCP and every window. Provenance `computed`, with `"basis": "screen trace, 1000 points, 8 bit"`.

### 5.2 Feature hints (`features`)

`adc_bits` (8/12; shown next to computed values), `wave_points` (screen record size),
`measurements` (keep: the native items, now canonical names), `computed_measurements` (server
fills), `memory_depths`, `trigger_types` (or from the setter enum), `counter: true`,
`max_srate`, `bandwidth_hz` (exists; UI derives the rise-time floor 0.35/bandwidth),
`reference_channel`, `math_tokens` (colour for MATH/FFT). The BW switch should read the
`bwlimit` setter's enum values instead of `20M`.

### 5.3 Panel UX sketch

```
+-- screen --------------------------------------------+-- side ------------------+
| RUN T'D AUTO NORM | 1 ms/div 1 GSa/s trig CH1 up 1.2 V| Acquisition [Run][Single] |
| [graticule + traces + MATH (5th colour) + ghosts]    |  type NORM PEAK AVER HRES  |
| [cursors: drag X1/X2, Y1/Y2; dT 1/dT dV readout]     |  averages [16]  depth [10M]|
| CH1 1 V/div DC  CLIP                                  | Horizontal  [1 ms] [pos]   |
|   Vpp 3.21 V scope | Freq 1.000 kHz scope | Rise 12 ns calc ...  (chooser +)    |
|   stats: min/max/avg/sigma/n (server, over polls)    | Trigger type [EDGE v]     |
+------------------------------------------------------+  src/slope/level/coupling |
| [Capture] [Overlay N] [FFT] [Math A-B v] [CSV] [PNG] |  holdoff [Force]          |
+------------------------------------------------------+---------------------------+
| CH1..CHn strips (unchanged)                                                       |
```

- Measurement chooser per channel: canonical list; native items tagged `scope`, computed `calc`
  (tooltip: basis, points, bits); every value stays `Recordable`.
- Cursors, overlay, FFT and math display live in the browser; a math or measurement that the
  user records is computed by the server (one value everywhere).
- "Not coherent" hint on math/phase while running, with a one-click Stop/Single.
- Capture opens a full-resolution view (zoomable, like the recordings chart) and saves to the
  timeline.

### 5.4 DSL gaps

From cov §4 (still open): block reply with a text prefix (Siglent legacy `C1:WF DAT2,#9`),
binary struct step (Siglent WAVEDESC), `u32le` length-prefixed reply and JSON/SI-suffix parse
(OWON), vendor bulk USB (OWON), multi-read accumulation (Hantek, RAW chunks), RLE image (GW).
New from this analysis:
1. **Parameterised GET** (`GET .../{op}?item=RISE` or an op with a value on GET): today GET takes
   no value, so 15 canonical items mean 15 commands and 15 operations per definition; a single
   `measure` op with an item enum would keep definitions small.
2. **Chunk loop** for full records: repeat `:WAV:STAR/STOP/DATA?` over n chunks and concatenate
   (a `repeat: {from, to, step}` step) - same primitive as multi-read accumulation.
3. **Poll groups by demand:** poll native measurement items only when shown/recorded (cost
   ~25 ms each), e.g. `poll.when: shown(KEY)`; otherwise compute.
4. **Read timestamps per command** inside a poll group (for `WAVE_COHERENT` and cross-group
   skew warnings).
5. **WORD traces** (`u16le`) for 12-bit models with `adc_bits` scaling in Y0/DY.
6. Screenshot: transport work (userspace USBTMC), not DSL.

### 5.5 Suggested order

1. L1w module with Vmax/Vmin/Vpp/mean/RMS/freq/period/duty/widths/rise/fall + CLIP, provenance,
   unit tests on synthetic traces (sine, square, clipped, noisy) - CI-safe.
2. UI cursors + measurement chooser + BW enum fix (vitest against `bench-fixtures.json`).
3. Contract enums + Common-tier ops (trigger type/coupling/holdoff/force, memory depth, averages)
   bound in `scpi_osc` and `siglent_scope` first (10 of 14 definitions at once), verified with
   `bm-probe` on the DHO804.
4. Capture (SINGLE + full record) as server op + L3 flow; browser FFT/math/overlay.
5. Native statistics/counter binding; Bode and ripple flows with AWG/PSU/ELL.
