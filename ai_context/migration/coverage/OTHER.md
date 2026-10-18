# OTHER — further instrument classes (coverage research)

Status: research report, 2026-10-18. No contract or definition written yet; every family below is
**Awaiting confirmation** at best. Classes covered elsewhere: PSU, DMM, ELL, AWG, OSC (contracts in
`devices/classes/`), LCR and SAL (`SAL.md`). Manuals live outside the repo in
`/home/marek/project/BenchMeshDOC/<vendor>_<family>/doc/` (each with `SOURCES.md`); page numbers are
**PDF pages** of those local files (`pN`), printed page in parentheses where it differs. Source files
(firmware, drivers) are cited by line (`l.N`).

Limits of this research: prevalence is judged from forum/marketplace presence seen during the search
phase (EEVblog threads, AliExpress/Amazon listings, used-market availability) plus domain knowledge;
the session's web-search budget ran out before every candidate could be re-checked, so the ranking
is a reasoned judgement, not a count. Two programming manuals could not be downloaded (Rigol
DSG800: the vendor link now redirects; Windfreak: site behind a bot challenge) — those families are
marked low/medium confidence.

---

## 1. Ranking of classes

| # | Class (proposed code) | Why hobbyists/small labs connect it | Top families | Protocol | DSL today |
|---|---|---|---|---|---|
| 1 | **VNA** — vector network analyzer | NanoVNA is the most-discussed RF instrument in the hobby (EEVblog "NanoVNA Custom Software" thread > 550 k reads); every ham owns one | NanoVNA-H/H4/-D fw; (LiteVNA/V2 blocked) | text shell over USB CDC | settings yes, traces need `numbers` |
| 2 | **PWR** — power meter / power analyzer | Mains power/standby/efficiency measurement; PZEM-004T is a top-selling hobby module; GPM-8213/WT310E are the small-lab standard | Yokogawa WT300E, GW Instek GPM-8213, (PZEM-004T blocked) | SCPI-like `:NUMeric` (shared baseline), Modbus RTU | WT300E/GPM-8213 **yes**; PZEM no |
| 3 | **CNT** — frequency counter | Few standalone counters with PC links in hobby hands, **but the counter built into the AWGs they already own** (FY6900, SDG1000X/2000X, DG800/900/1000Z) is the same class | AWG built-ins (bind CNT in the AWG definition), Keysight 53220A/53230A | SCPI / FeelTech ASCII | **yes** |
| 4 | **SMU** — source-measure unit | Used Keithley 2400 is the classic EEVblog bargain for I-V curves, LED/diode/solar-cell work; 2450/B2900 in labs | Keithley 2400 series, Keithley 2450, Keysight B2900A — all speak the **2400 command set** | SCPI | **yes** (one safety caveat, §5.4) |
| 5 | **RFG** — RF signal generator | Receiver alignment, filter tests, ham projects; Siglent SSG3021X is the cheapest SCPI RF generator | Siglent SSG3000X, Rigol DSG800, Windfreak SynthHD | SCPI / single-letter ASCII | **yes** (SynthHD needs a computed channel param) |
| 6 | **DAQ** — data acquisition / multi-channel logger | Temperature logging, battery/thermal tests; 34970A is ubiquitous on the used market, DAQ970A its successor | Keysight DAQ970A/DAQ973A (34970A/34972A compatible) | SCPI | single channel yes; scan list needs `numbers` |
| 7 | **BAT** — battery internal-resistance tester (AC mΩ + V) | Li-ion cell sorting/matching (18650 packs, e-bike builders) | Hioki BT3561A/BT3562A/BT3563A; cheap RC3563/YR1035+ (no documented protocol) | SCPI over RS-232 | Hioki **yes**; cheap ones unknown/blocked |

Dropped / out of scope:

| Candidate | Reason |
|---|---|
| USB power meters (FNIRSI FNB58, ChargerLAB POWER-Z, ATORCH UD-series) | Very popular, but **USB HID** (FNB58, POWER-Z) or BLE; would need an `hid` link + binary reports (§10). Could later bind PWR (DC subset) |
| Digilent Analog Discovery, Pico TC-08, PicoScope | Vendor SDK / DLL only, no byte protocol |
| Battery *capacity* testers (ZKETECH EBC-A20, ATORCH DL24) | Electronic loads with a discharge program: belong to **ELL** (see the ELL research, `zketech_ebd`, `atorch_dl24`); EBC-A20 is binary (0xFA…0xF8 frames, unsolved checksum per mikrocontroller.net) |
| Insulation testers, soldering stations, environmental chambers, programmable AC sources | Rare PC interfaces in the hobby, little value for a bench cockpit |
| Switch matrices | Covered as DAQ (`ROUTe:CLOSe/OPEN` on DAQ970A multiplexer cards) |
| Cheap ADF4351/MAX2870 boards | No common protocol (each firmware differs, many have only buttons) |
| Juntek/MCH/"PLJ" cheap counters | No PC interface on the common models; FY6900's built-in counter covers the use |

---

## 2. VNA — vector network analyzer

`SAL.md` §2.5 documents the NanoVNA transport/framing (identical to tinySA) and §6 recommends a
separate VNA class; this section gives the contract and the family table. Docs:
`/home/marek/project/BenchMeshDOC/nanovna_nanovna/doc/` (firmware source; no vendor manual exists).

### 2.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: VNA
description: >
  Vector network analyzer. Traces are complex S-parameters as interleaved
  re,im lists (one pair per point); the UI derives |S| dB, phase, SWR,
  R+jX and the Smith chart. Frequencies in Hz.
status:
  START:    {type: number, required: true, description: "Sweep start (Hz)"}
  STOP:     {type: number, required: true, description: "Sweep stop (Hz)"}
  POINTS:   {type: integer, required: true}
  S11:      {type: list, description: "Reflection, interleaved re,im per point; null until swept"}
  S21:      {type: list, description: "Transmission, interleaved re,im per point"}
  FREQS:    {type: list, description: "Point frequencies (Hz); else derived from START/STOP/POINTS"}
  CAL:      {type: string, description: "Calibration state (e.g. 'applied', 'none', slot number)"}
  IFBW:     {type: number, description: "IF bandwidth (Hz)"}
  RUNNING:  {type: bool, description: "Sweep running (false after pause)"}
  BATTERY:  {type: number, description: "Battery voltage (V) for handheld units"}
operations:
  start:     {get: true, set: true, value: number, required: [set]}
  stop:      {get: true, set: true, value: number, required: [set]}
  points:    {get: true, set: true, value: integer}
  s11:       {get: true, value: list, required: [get]}
  s21:       {get: true, value: list}
  frequencies: {get: true, value: list}
  run:       {set: true, value: enum, values: [ON, OFF], description: "resume / pause sweep"}
  ifbw:      {set: true, value: number}
  recall:    {set: true, value: integer, description: "Recall calibration/setup slot"}
  screenshot: {get: true, value: bytes}
```

### 2.2 Panel

Like the NanoVNA screen/NanoVNA-Saver: one trace canvas (kit oscilloscope screen reused) with
format selector LOGMAG / PHASE / SWR / SMITH / R+jX, traces S11 and S21 coloured as
`--kit-series-*`; marker readout (frequency, |S| dB, SWR, R+jX, L/C equivalent); START / STOP
digit editors (Hz, 1-2-5 stepper for span), POINTS selector, IFBW selector, RUN/PAUSE micro
switch, CAL annunciator (green "C" when calibration applied, as on the NanoVNA status bar), battery
annunciator. No energize controls (output power is tiny; `power` setter is `configure`).

### 2.3 Families

**NanoVNA-H / H4, NanoVNA-D firmware (DiSlord), hugen firmware** — 30–80 €; by far #1.
- Interface: USB CDC-ACM **0483:5740** (`usbcfg.c` l.45-46) — shared with tinySA and every
  ChibiOS STM32 VCP, so identity comes from the `info` reply (`SAL.md` §2.5). Baud ignored.
- Framing: line ends `\r` (`\r\n` harmless), every char echoed (`main.c` l.3205), output lines
  end `\r\n` (l.42), then prompt `ch> ` (l.44). `rx.terminator: "ch> "`, strip the echo line
  with `{regex: {pattern: "^[^\\n]*\\n([\\s\\S]*?)\\r?\\n?$", group: 1}}`.

| Operation | Command | Reply | Source |
|---|---|---|---|
| identify | `info` | multi-line, `Board: NanoVNA-H 4` … | SAL.md §2.5 |
| version | `version` | git tag, e.g. `1.2.40` | l.2566 |
| sweep get | `sweep` | `start stop points` (`%u %u %d`) | l.1614 |
| sweep set | `sweep <start> <stop> [points]`, `sweep start|stop|center|span|cw <Hz>` | none | l.1611-1650 |
| frequencies | `frequencies` | one `%u` per point | l.2359 |
| S11 / S21 | `data 0` / `data 1` (2..6 = cal arrays) | `"%f %f"` (re im) per point | l.753-769 |
| measured scan | `scan <start> <stop> [points] [outmask]` (1 freq, 2 S11, 4 S21, 0x80 binary) | text lines `freq re im re im` | l.1384-1452 |
| pause/resume | `pause` / `resume` | none | l.515-530 |
| IF bandwidth | `bandwidth <n>` | | l.1299 |
| recall | `recall <slot>` | | l.2084 |
| screenshot | `capture` | raw RGB565 (or RLE8) frame, no header length | l.804 |

- Quirks: `CMD_WAIT_MUTEX` commands (scan, data, sweep, freq) execute at the next sweep point
  and break the running sweep (cmd table l.2919-2926); 101 points (H, F072) / 401 (H4).
  `capture` has no length prefix — the size is fixed by the LCD (`320×240×2` or `480×320×2`), so
  `reply: {bytes: N}` per model works.
- Confidence: **high** (firmware is the protocol). DSL: framing, sweep get/set, pause/resume
  today; `data 0/1` and `frequencies` need **`numbers`** (whitespace/newline separated → list);
  binary `scan … 0x80` would need `samples: f32le` + 4-byte header skip.

**NanoVNA-F (BH5HNU), NanoVNA V2 / SAA-2 / LiteVNA** — NanoVNA-F speaks a compatible text shell
(not verified here, medium). V2/LiteVNA use a **binary register protocol** (`zeenko_litevna`
folder, see `SAL.md` §2.12) → blocked (needs binary register read/write framing).

**Siglent SVA1000X (VNA mode)** — SCPI, documented in the SAL research (`siglent_sva1000x`); one
definition can bind SAL and VNA.

---

## 3. PWR — power meter / power analyzer

### 3.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: PWR
description: >
  Single-phase (or per-element) AC/DC power meter. Numeric readings are SI
  value dicts. Multi-element meters (WT332E/WT333E) report one channel per
  element; SIGMA is the last channel.
status:
  U:       {type: si, required: true, description: "Voltage RMS (V)"}
  I:       {type: si, required: true, description: "Current RMS (A)"}
  P:       {type: si, required: true, description: "Active power (W)"}
  S:       {type: si, description: "Apparent power (VA)"}
  Q:       {type: si, description: "Reactive power (var)"}
  PF:      {type: number, description: "Power factor λ (-1..1)"}
  PHI:     {type: number, description: "Phase angle (deg)"}
  FREQ:    {type: si, description: "Voltage frequency (Hz)"}
  ENERGY:  {type: si, description: "Integrated active energy (Wh)"}
  ITIME:   {type: number, description: "Integration elapsed time (s)"}
  UTHD:    {type: number, description: "Voltage THD (%)"}
  ITHD:    {type: number, description: "Current THD (%)"}
  MODE:    {type: string, description: "Measurement mode (RMS/VMEAN/DC/AC/ACDC; features.modes key)"}
  URANGE:  {type: string, description: "Voltage range or AUTO"}
  IRANGE:  {type: string, description: "Current range or AUTO"}
  OVERLOAD: {type: bool, description: "Range exceeded (display OL / -OL-)"}
  INTEG:   {type: enum, values: [RUN, STOP, RESET], description: "Integrator state"}
operations:
  mode:          {set: true, value: string}
  voltage_range: {set: true, value: string, description: "features.URANge value or AUTO"}
  current_range: {set: true, value: string, description: "features.IRANge value or AUTO"}
  integration:   {set: true, value: enum, values: [START, STOP, RESET]}
  update_rate:   {set: true, value: string, description: "Data update interval (features.rates)"}
  hold:          {set: true, value: enum, values: [ON, OFF]}
  measurement:   {get: true, value: si}
```

### 3.2 Panel

As on WT310E/GPM-8213 fronts: four stacked segment readouts (default V, A, W, PF — each
selectable from the status keys, like the FUNCTION keys of display areas a–d), annunciators
V-RANGE/A-RANGE (AUTO), OL, HOLD, INTEG RUN, harmonic/THD when present; controls: MODE select
(RMS/MEAN/DC or DC/AC/ACDC), V and A range steppers with AUTO, integration START/STOP/RESET
buttons with elapsed time readout, update-rate selector. Energy/time row below. No energize
controls (meters never source).

### 3.3 Shared baseline: the `:NUMeric` data-output model

Yokogawa WT300E and GW Instek GPM-8213 share the same command model (GW Instek copied the WT
command tree): `:NUMeric[:NORMal]:ITEM<x> <function>[,<element>]`, `:NUMeric[:NORMal]:NUMBer n`,
`:NUMeric[:NORMal]:PRESet p`, `:NUMeric[:NORMal]:VALue?` → comma-separated NR3 with `NAN` for
missing data, `[:INPut]:VOLTage:RANGe`, `[:INPut]:CURRent:RANGe`, `:INPut:MODE`. One `extends`
baseline: on connect set ASCII + a fixed item list, poll `:NUM:VAL?` and split with `fields`.

### 3.4 Families

**Yokogawa WT300E (WT310E, WT310EH, WT332E, WT333E)** — 2–5 k€, small-lab reference. Docs:
`/home/marek/project/BenchMeshDOC/yokogawa_wt300e/doc/IMWT310E-17EN.pdf`.
- Interfaces: USB **USBTMC-USB488** (p9; Yokogawa vid 0b21, pid not in the manual), RS-232
  (1200–57600 baud, p21/p27), GP-IB, Ethernet (TCP — not implemented). Response message
  terminator NL^END (p33); send LF or CR+LF (p27).
- Identify: `*IDN?` → `YOKOGAWA,WT310E,123456789A,F1.01` (p74, 6-34).

| Operation | Command | Notes | Page |
|---|---|---|---|
| data format | `:NUMeric:FORMat ASCii` | FLOat gives `#N` blocks | p59 (6-19) |
| items | `:NUMeric:NORMal:PRESet 2` + `:NUMeric:NORMal:NUMber 9` | pattern 2 = U,I,P,S,Q,LAMBda,PHI,FU,FI of element 1, then element 2/3/SIGMA at 11–19/21–29/31–39 | p59, p66 (6-26) |
| readings | `:NUMeric:NORMal:VALue?` | `103.79E+00,1.0143E+00,…`, `NAN` = no data | p59 |
| one item | `:NUMeric:NORMal:ITEM1 U,1` / `:VALue? 1` | | p60 (6-20) |
| headers off | `:COMMunicate:HEADer OFF`, `:COMMunicate:VERBose OFF` | keep replies bare | p47 (6-7) |
| remote/local | `:COMMunicate:REMote ON|OFF` | | p47 |
| update rate | `:RATE <time>` | 100 ms–20 s | p43 (6-29 index) |

- Quirks: data update interval — poll no faster than `:RATE`; check bit 0 of the extended event
  register to see "updated" (p40) if exact sync matters. `:NUMeric:LIST` (harmonics) only with
  /G5 option.
- Confidence **high**. DSL: **today** (`usbtmc` or `serial`, `rx: message|terminator`,
  `fields` on `:NUM:VAL?`, CEL `num()` maps `NAN` to null; per-element channels via item
  offsets in separate commands).

**GW Instek GPM-8213 (+ GPM-8213G with GPIB)** — ~700–900 €, popular in small labs and with
EEVblog reviewers. Docs: `/home/marek/project/BenchMeshDOC/gwinstek_gpm8213/doc/GPM-8213_User_Manual_G.pdf`.
- Interfaces: USB **CDC virtual COM** (8 data, 1 stop, no flow; p65), RS-232 1200–115200 8N1
  (p65), LAN (TCP — not implemented). EOL **CR+LF** (p72). vid:pid not in manual (GW Instek
  usually 2184:xxxx).
- Identify: `*IDN?` → `GWINSTEK,GPM-8213,GXXXXXXXX,V1.00` (p75).

| Operation | Command | Notes | Page |
|---|---|---|---|
| mode | `:INPut:MODE {DC|ACDC|AC}` | | p85 |
| V range | `:INPut:VOLTage:RANGe {15|30|60|150|300|600}` (CF3; CF6 halves) / `:INPut:VOLTage:AUTO ON` | reply `600.0E+00` | p86 |
| I range | `:INPut:CURRent:RANGe {5m…500m,1,2,5,10,20}` | | p87 |
| items | `:NUMeric:NORMal:ITEM<x> {U|I|P|S|Q|LAMBda|PHI|FU|FI|UTHD|ITHD|WH|AH|TIME|URANge|IRANge…}` (1–34) | default NUMBer is **3** (not 10 as on WT) | p97-98 |
| preset | `:NUMeric:NORMal:PRESet {1..4}` | pattern 1 = U,I,P | p99 |
| readings | `:NUMeric:NORMal:VALue?` | NR3 list, `NAN` for "-----" | p96-97 |

- Quirks: queries echo the header (`->:INPUT:MODE DC`, p85) — strip with `{split: {sep: " ", index: -1}}`
  or regex; numeric `VALue?` is bare. Manual typo: `CFV` row means current crest factor (p98).
- Confidence **high**. DSL: **today** (`serial` 115200 or the CDC port; extends the WT baseline).

**Peacefair PZEM-004T v3.0 (10 A / 100 A CT), PZEM-014/016, PZEM-017 (DC)** — 5–15 €, extremely
common with ESP32/Arduino/Home-Assistant users, connected to a PC via a USB-TTL adapter (CH340/
CP2102). Docs: `/home/marek/project/BenchMeshDOC/peacefair_pzem004t/doc/PZEM-004T_V3.0_datasheet.pdf`.
- Link: UART 9600 8N1 (p2), **Modbus RTU**, slave 0x01–0xF7, 0xF8 = general address for a single
  device (p2).
- Read: function 0x04, registers 0x0000–0x0009: U 0.1 V, I 32-bit **low word first** 0.001 A,
  P 32-bit low word first 0.1 W, energy 32-bit 1 Wh, f 0.1 Hz, PF 0.01, alarm 0xFFFF (p3).
  Params: function 0x03/0x06 registers 0x0001 alarm threshold (W), 0x0002 address (p4). Reset
  energy: custom function 0x42 (p5).
- Quirks: datasheet says "CRC high byte + low byte", real devices send standard Modbus order
  (low first) — confirm on hardware; 32-bit values are word-swapped relative to the usual big-endian
  convention; forum reports some units refuse multi-register reads (esp32.com thread).
- Confidence: medium-high (protocol clear, quirks need a capture). DSL: **blocked** — needs Modbus
  RTU (§10 F2).

---

## 4. CNT — frequency counter

### 4.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: CNT
description: >
  Frequency counter / timer, standalone or built into an AWG (the AWG
  definition binds both classes). Readings are SI value dicts; null while
  no signal or gate running.
status:
  FREQ:     {type: si, required: true, description: "Measured frequency (Hz)"}
  PERIOD:   {type: si, description: "Period (s)"}
  DUTY:     {type: number, description: "Duty cycle (%)"}
  PWIDTH:   {type: si, description: "Positive pulse width (s)"}
  NWIDTH:   {type: si, description: "Negative pulse width (s)"}
  TOTAL:    {type: integer, description: "Totalize count"}
  FREQDEV:  {type: number, description: "Deviation from reference (ppm)"}
  MODE:     {type: string, description: "Active measurement (features.modes key: FREQ, PER, TOT, …)"}
  GATE:     {type: number, description: "Gate time (s)"}
  COUPLING: {type: enum, values: [AC, DC]}
  IMPEDANCE: {type: string, description: "50 or 1M (features.impedances)"}
  RUNNING:  {type: bool, description: "Counter enabled / running"}
operations:
  counter:     {set: true, value: enum, values: [ON, OFF], description: "Enable (AWG built-ins)"}
  mode:        {set: true, value: string}
  gate:        {get: true, set: true, value: number}
  coupling:    {set: true, value: enum, values: [AC, DC]}
  impedance:   {set: true, value: string}
  level:       {set: true, value: number, description: "Trigger level (V)"}
  hf_reject:   {set: true, value: enum, values: [ON, OFF]}
  reset:       {set: true, value: enum, values: [ON], description: "Reset totalizer"}
  measurement: {get: true, value: si, required: [get]}
```

### 4.2 Panel

Like a 53220A / the counter page of an AWG: one large 10–12 digit segment readout (FREQ by
default, unit autoscaled), secondary row PERIOD · DUTY · +W · −W, MODE selector (FREQ / PERIOD /
TOTALIZE / PULSE), GATE 1-2-5 stepper (or the instrument's gate enum), input block with
AC/DC and 50 Ω/1 MΩ micro switches, trigger-level knob, HF-REJECT switch, RUN/STOP, annunciators
GATE (blinks per gate), NO SIGNAL, EXT REF. For AWG built-ins only the controls in `features`
show (FY6900: gate only).

### 4.3 Families

**AWG built-in counters** — owned by most hobbyists who own an AWG; bind `CNT` in the existing AWG
definitions (one definition, two classes, like SVA1000X for SAL/VNA).

| Family | Enable | Read | Reply | Settings | Source |
|---|---|---|---|---|---|
| Rigol DG1000Z / DG800 / DG900 | `:COUNter:STATe ON` (disables CH2 sync output) | `:COUNter:MEASure?` | 5 fields `freq,period,duty,+width,-width` (NR3), all zero when off | `:COUN:GATE USER1..6`, `:COUN:COUP AC|DC`, `:COUN:LEVE`, `:COUN:SENS`, `:COUN:HF` | `rigol_dg1000z/doc/DG1000Z_ProgrammingGuide_EN.pdf` p18-19 (2-6/2-7); DG800 PG p17-18 |
| Siglent SDG1000X/2000X/5000/6000X (not SDG800/7000A) | `FCNT STATE,ON` | `FCNT?` | `FCNT STATE,ON,FRQ,10000000HZ,DUTY,59.8568,REFQ,1e+07HZ,TRG,0V,PW,5.98568e-08S,NW,…,FRQDEV,0ppm,MODE,AC,HFR,OFF` | `FCNT REFQ,…`, `TRG`, `MODE AC|DC`, `HFR ON|OFF` | `siglent_sdg/doc/SDG_Programming_Guide_PG02-E05C.pdf` p74-75 (62-63) |
| FeelTech FY6900 (also FY6800, rebrands) | always on | `RCF` freq, `RCT` period ns, `RC+`/`RC-` widths ns, `RCD` duty ‰, `RCC` count | 10-digit integers, **freq scaled by gate** (1 s → Hz, 10 s → 0.1 Hz) | `WCG` gate, `WCZ0` reset, `WCP0` pause | `feeltech_fy6900/doc/FY6900_Serial_communication_protocol.pdf` p25-26, 115200 baud LF (p1-2) |

- DSL: **today** — Rigol `fields`; Siglent `regex` per key (`FRQ,([-+0-9.eE]+)`); FY6900 CEL
  with a cached gate query (`num(rcf) / gate_factor`).

**Keysight 53220A / 53230A (and 53210A)** — 2–5 k€ new, used ~1 k€; small labs, time-nuts.
Docs: `/home/marek/project/BenchMeshDOC/keysight_53200/doc/53220-90001_UsersGuide.pdf` (the full
Programmer's Reference is on the product CD and was not found online).
- Interfaces: USBTMC, example address `USB0::2391::1287::…` → **0957:0507** (p63), LAN, GPIB.
- Identify: `*IDN?` → `Agilent|Keysight Technologies,53220A,<serial>,<fw>` (format per IEEE 488.2;
  example not in the user's guide — confirm).

| Operation | Command | Notes | Page |
|---|---|---|---|
| configure | `CONF:FREQ [<expected>[,<resolution>]],(@1)`; `CONF:PER`, `CONF:TOT`, `CONF:FREQ:RAT` | ch 1/2, ch 3 = option 106/115 | p94-95 |
| query config | `CONF?` | `"FREQ +1.0E+006,+1.0E-004,(@2)"` (quoted) | p92 |
| read | `READ?` (initiate + wait gate) / `MEAS:FREQ? …` | NR3; `9.91E37` = no signal (display `-------`) | p41, p86, p94 |
| gate | `SENS:FREQ:GATE:TIME 0.1` / `SENS:FREQ:GATE:SOUR TIME` | | p92, p172-173 |
| mode | `SENS:FREQ:MODE AUTO|REC|CONT` | | p83 |
| input | `INP:IMP 50|1E6`, `INP:COUP AC|DC`, `INP:LEV <V>`, `INP:FILT ON|OFF` | per channel `INP2:` | p134-141 |

- Quirks: `READ?` blocks for the gate time (+ setup) — set `timeout_ms` ≥ gate + 1 s and poll
  every ≥ 2 × gate; `9.91E37` → null via CEL. 531xxA programs run in emulation mode
  (`53200_Programming_Comparison_Guide_5990-6570.pdf`).
- Confidence: medium-high (command set clear; reference manual not local). DSL: **today**.

---

## 5. SMU — source-measure unit

### 5.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: SMU
description: >
  Source-measure unit: sources voltage or current with a compliance limit on
  the other quantity and measures both. Status per channel. SOURCE and LIMIT
  are in the unit of SRC_FUNC (V or A) and its complement.
status:
  SRC_FUNC:   {type: enum, values: [VOLT, CURR], required: true, description: "Source function"}
  SOURCE:     {type: si, required: true, description: "Programmed source level (V or A)"}
  LIMIT:      {type: si, required: true, description: "Compliance limit (A when sourcing V, V when sourcing I)"}
  VMEAS:      {type: si, required: true, description: "Measured voltage; null while output off"}
  IMEAS:      {type: si, required: true, description: "Measured current; null while output off"}
  RMEAS:      {type: si, description: "Resistance V/I (ohm)"}
  OUTPUT:     {type: enum, values: [ON, OFF], required: true}
  COMPLIANCE: {type: bool, description: "In compliance (limit reached, source clamped)"}
  SENSE:      {type: enum, values: [2W, 4W], description: "Remote sense"}
  TERMINALS:  {type: enum, values: [FRONT, REAR]}
  OVERLOAD:   {type: bool, description: "Measurement over range"}
  REMOTE:     {type: enum, values: [ON, OFF]}
operations:
  source_function: {get: true, set: true, value: enum, values: [VOLT, CURR], required: [set]}
  source:          {get: true, set: true, value: number, required: [get, set], description: "Source level"}
  limit:           {get: true, set: true, value: number, required: [get, set], description: "Compliance"}
  output:          {get: true, set: true, value: enum, values: [ON, OFF], required: [set]}
  sense:           {set: true, value: enum, values: [2W, 4W]}
  terminals:       {set: true, value: enum, values: [FRONT, REAR]}
  source_range:    {set: true, value: string, description: "features.ranges value or AUTO"}
  nplc:            {set: true, value: number, description: "Integration time (power line cycles)"}
  measurement:     {get: true, value: si}
```

`output` must carry `safety: energize` in every definition (two presses in the UI), and so must
`source_function` while the output is on (switching V→I with output on is an energize-class step).

### 5.2 Panel

Like the 2450 home screen / 2400 two-line display: large measured readout (the "measure"
function, I by default when sourcing V), smaller second readout for the other quantity; SOURCE
digit editor with V/I unit by SRC_FUNC and LIMIT digit editor below it (labelled "Compl" on 2400
style, "Limit" on 2450 style); SOURCE V / SOURCE I selector (the 2400 SOURCE V/I keys); arm-to-
enable OUTPUT switch with red ON lamp; annunciators CMPL (blinking as on the 2400), 4W, REAR,
REM, OVP; range stepper with AUTO; NPLC (speed FAST/MED/NORMAL/HI ACC as on the 2400 SPEED key).
A later "I-V sweep" helper belongs to automation, not the panel.

### 5.3 Shared baseline: the Keithley 2400 command set

All three families accept it: native on the 2400 series; Keithley 2450 via `*LANG SCPI2400` +
reboot (`2450-901-01_D_Ref.pdf` p105, 2-79); Keysight B2900A via `:SYST:LANG "2400"`
(`B2900A_SCPI_Reference_9018-01705.pdf` p296; supported/partially supported conventional
commands ch. 6, p346+). One `extends` baseline = 2400; native 2450/B2900 SCPI as variants.

### 5.4 Families

**Keithley 2400 series (2400, 2400-C, 2401, 2410, 2420, 2425, 2430, 2440)** — used 1–3 k€, the
classic used-market SMU. Docs: `/home/marek/project/BenchMeshDOC/keithley_2400/doc/2400S-900-01_K_User.pdf`.
- Models (ranges p60-62): 2400 ±210 V / ±1.05 A (20 W); 2401 ±21 V / 1.05 A; 2410 ±1.1 kV /
  ±21 mA (1 A at 20 V); 2420 ±63 V / ±3.15 A; 2425 ±105 V / 3 A; 2430 pulse 10.5 A; 2440 ±42 V / 5 A.
- Interfaces: **RS-232** (DB-9; 300–57600 baud, factory 9600, 8N1 or 7/8 bit with parity, flow
  none/XON-XOFF, terminator CR / CR+LF / LF / LF+CR selectable in menu; p270-271, 14-13/14-14)
  and GPIB. No USB — hobbyists use a USB-RS232 cable (or a Prologix/AR488 GPIB adapter, §10 F6).
- Identify: `*IDN?` → `KEITHLEY INSTRUMENTS INC.,MODEL 2400,<serial>,<rev>` (p297).

| Operation | Command | Notes | Page |
|---|---|---|---|
| source function | `:SOUR:FUNC VOLT|CURR`, `:SOUR:VOLT:MODE FIX` | | p73 (3-18), Table 3-5 |
| source level | `:SOUR:VOLT:LEV <n>` / `:SOUR:CURR:LEV <n>` (query with `?`) | | p73 |
| source range | `:SOUR:VOLT:RANG <n>` / `:SOUR:VOLT:RANG:AUTO ON` | | p73 |
| compliance | `:SENS:CURR:PROT <A>` / `:SENS:VOLT:PROT <V>` | | p63 (3-8), Table 3-3 |
| measure function | `:SENS:FUNC "CURR"` (quoted), `:SENS:FUNC:CONC ON` | | p73-74 |
| output | `:OUTP ON|OFF`, `:OUTP?` → `1|0` | | p73 |
| reading | `:FORM:ELEM VOLT,CURR,RES,STAT` once, then `:READ?` → `+1.000E+00,+1.2E-03,+9.91E+37,+1.7E+04` | `+9.91e37` = not measured (NAN), `+9.9E37` overflow; STAT bit 3 = compliance, bit 0 = overflow, bit 22 = 4W | p354-356 (18-49…18-51) |
| sense | `:SYST:RSEN ON|OFF` | 4-wire | p92 |
| speed | `:SENS:CURR:NPLC 0.01..10` | | p153 |
| local | `:SYST:LOC` (RS-232 only) | | p417 |

- Quirks: **`:READ?` requires the output on** when auto output-off is disabled; with
  `:SOUR:CLE:AUTO ON` *any* `:READ?`/`:MEAS?` switches the output on for each reading (p209,
  11-11) — so a poll must never use `:MEAS?` and must not run `:READ?` while OUTPUT is OFF
  (needs a conditional poll, §10 F5; workaround: poll `:OUTP?` + `:SOUR:*:LEV?` only and read
  `:READ?` on demand). Commands queue while a trigger model runs; `^C`/`^X` on RS-232 clears
  (p271). Rule 2: read the complete reply before the next command (p270).
- Confidence **high**. DSL: **today** (serial), with the poll caveat.

**Keithley 2450 (and 2460/2461 with the same SCPI tree)** — 5–8 k€ new; universities, labs.
Docs: `/home/marek/project/BenchMeshDOC/keithley_2450/doc/2450-901-01_D_Ref.pdf`.
- Models: 2450 ±210 V / ±1.05 A (p429, 6-80); 2460 100 V / 7 A; 2461 pulse 10 A (same tree,
  separate manuals — not local).
- Interfaces: USBTMC (Keithley vid 05e6; pid expected 2450 — confirm), LAN (VXI-11/raw 5025,
  not implemented), GPIB. Command set selectable SCPI / TSP / SCPI2400 (`*LANG`, p105).
- Identify: `*IDN?` → `KEITHLEY INSTRUMENTS,MODEL 2450,<serial>,<fw>` (format — confirm).

| Operation | Command | Notes | Page |
|---|---|---|---|
| source function | `:SOUR:FUNC VOLT|CURR` | | p431 (6-82) |
| source level | `:SOUR:VOLT <n>` / `:SOUR:CURR <n>` (`?`) | ±210 V, ±1.05 A | p429 (6-80) |
| limit | `:SOUR:VOLT:ILIM <A>` / `:SOUR:CURR:VLIM <V>` | defaults 105 µA / 21 V | p430 (6-81) |
| limit tripped | `:SOUR:VOLT:ILIM:TRIP?` → `0|1` | = COMPLIANCE | p431 |
| output | `:OUTP ON|OFF`, `:OUTP?` | | p390 (6-41) |
| reading | `:MEAS:CURR?` / `:READ? "defbuffer1", SOUR, READ` | | p355 (6-6) |
| interlock | `:OUTP:INT:TRIP?` | 200 V range only with interlock | p390 |

- Quirks: output off → 2-wire regardless of sense setting (p113); front-panel "Output Off" text;
  >±21 V needs the interlock (p390). Changing `*LANG` needs a reboot.
- Confidence **high**. DSL: **today** (usbtmc).

**Keysight B2900A (B2901A/B2902A/B2911A/B2912A; B2900B/BL similar)** — 5–15 k€; labs.
Docs: `/home/marek/project/BenchMeshDOC/keysight_b2900/doc/B2900A_SCPI_Reference_9018-01705.pdf`
(+ Programming Guide 9018-01707).
- Interfaces: USBTMC (Keysight 0957/2a8d, pid not in manual), LAN, GPIB. Channels 1–2 as `[c]`
  suffix (`:OUTP2`, `:SOUR2:…`) or `(@1,2)` channel lists.
- Identify: `*IDN?` → `Keysight Technologies,<model>,<serial>,<revision>` (p124).

| Operation | Command | Notes | Page |
|---|---|---|---|
| source function | `:SOUR{ch}:FUNC:MODE VOLT|CURR` | | p260 |
| source level | `:SOUR{ch}:VOLT <n>` / `:SOUR{ch}:CURR <n>` | | p249 |
| compliance | `:SENS{ch}:CURR:PROT <A>` / `:SENS{ch}:VOLT:PROT <V>` | | p230 |
| compliance tripped | `:SENS{ch}:CURR:PROT:TRIP?` | | p232 |
| output | `:OUTP{ch} ON|OFF` / `:OUTP{ch}?` → `0|1` | | p214 (4-80) |
| reading | `:FORM:ELEM:SENS VOLT,CURR` then `:MEAS? (@{ch})` | `+9.91E+37` = function off | p198 (4-64) |
| command set | `:SYST:LANG "2400"` | 2400 compatibility | p296 |

- Confidence **high**. DSL: **today**.

Cheap SMUs: none with a PC protocol of note found (Kickstarter-class devices use apps).

---

## 6. RFG — RF signal generator

### 6.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: RFG
description: >
  CW RF signal generator with optional analog modulation. FREQ in Hz,
  LEVEL in dBm. Multi-output generators (SynthHD) report per channel.
status:
  FREQ:     {type: si, required: true, description: "Output frequency (Hz)"}
  LEVEL:    {type: number, required: true, description: "Output level (dBm)"}
  OUTPUT:   {type: enum, values: [ON, OFF], required: true, description: "RF on"}
  MOD:      {type: enum, values: [ON, OFF], description: "Modulation master switch"}
  AM:       {type: enum, values: [ON, OFF]}
  AM_DEPTH: {type: number, description: "%"}
  FM:       {type: enum, values: [ON, OFF]}
  FM_DEV:   {type: si, description: "Hz"}
  PULSE:    {type: enum, values: [ON, OFF]}
  REF:      {type: enum, values: [INT, EXT]}
  LOCKED:   {type: bool, description: "PLL locked (false = UNLOCK annunciator)"}
  SWEEP:    {type: string, description: "OFF or the sweep kind"}
operations:
  frequency:  {get: true, set: true, value: number, required: [get, set]}
  level:      {get: true, set: true, value: number, required: [get, set]}
  output:     {get: true, set: true, value: enum, values: [ON, OFF], required: [set]}
  modulation: {set: true, value: enum, values: [ON, OFF]}
  am:         {set: true, value: enum, values: [ON, OFF]}
  am_depth:   {set: true, value: number}
  am_rate:    {set: true, value: number}
  fm:         {set: true, value: enum, values: [ON, OFF]}
  fm_dev:     {set: true, value: number}
  fm_rate:    {set: true, value: number}
  pulse:      {set: true, value: enum, values: [ON, OFF]}
  reference:  {set: true, value: enum, values: [INT, EXT]}
```

`output` is `energize`.

### 6.2 Panel

As on the SSG3000X/DSG800 front: big FREQ readout with the AWG digit-wise editor (Hz…GHz) and a
LEVEL editor in dBm (with the model's min/max from `features`), arm-to-enable **RF ON** switch,
**MOD ON** switch, a modulation strip (AM depth/rate, FM deviation/rate, pulse) showing only the
types in `features`, annunciators UNLOCK, EXT REF, RF ON, MOD; multi-channel (SynthHD A/B) as two
stacked rows in one panel.

### 6.3 Families

**Siglent SSG3000X (SSG3021X, SSG3032X, -IQ variants; SSG5000X likely same tree)** — 1.5–4 k€,
the cheapest SCPI RF generator; many hobby/lab reviews. Docs:
`/home/marek/project/BenchMeshDOC/siglent_ssg3000x/doc/SSG3000X_ProgrammingGuide_E01B.pdf`.
- Interfaces: USBTMC (Siglent f4ec), LAN raw **5025** / telnet 5024 (p8; TCP not implemented).
- Identify: `*IDN?` → `Siglent Technologies,SSG3032X,1234567890, 03.01.16r2` (p16, note space).

| Operation | Command | Notes | Page |
|---|---|---|---|
| RF output | `:OUTP ON|OFF`, `:OUTP?` | | p27 |
| frequency | `:FREQ <Hz>` / `:FREQ?` → Hz | units Hz/kHz/MHz/GHz accepted | p28 |
| level | `:POW <dBm>` / `:POW?` | −110…+20 dBm (≤ +9 dBm below 100 kHz, ≤ +15 below 1 MHz); IQ on: −100…+10 | p30 |
| level offset / ALC | `:POW:OFFS`, `:POW:ALC ON|OFF|AUTO` | | p31 |
| modulation | `:MOD ON|OFF` | master | p50 |
| AM | `:AM:STAT`, `:AM:DEPT 0.1–100`, `:AM:FREQ`, `:AM:SOUR INT|EXT` | | p51-53 |
| FM / PM | `:FM:STAT`, `:FM:DEV 0.01 Hz–1 MHz`, `:FM:FREQ`; `:PM:…` | | p53-58 |
| pulse | `:PULM:STAT`, train list | | p59-67 |

- Confidence **high**. DSL: **today** (usbtmc, `message` framing).

**Rigol DSG800 (DSG815, DSG821, DSG830) / DSG3000** — 1–3 k€. Manual **not downloaded** (the
eu.rigol.com URL now redirects; scribd copy only). From Rigol's DSG command tree: `:FREQ`, `:LEV`
(level is `:LEVel`, not `:POWer`), `:OUTP`, `:MOD:STAT`, `:AM:DEPT`, `:FM:DEV` — **verify**
against the guide before writing. USBTMC 1ab1. Confidence **low** until the guide is fetched.

**Windfreak SynthHD v2 / SynthHD PRO v2 (10 MHz–15 GHz, 2 channels), SynthNV/SynthUSB** —
~1–2 k€; popular with hams, microwave and quantum-optics labs. Official manual not downloadable;
command set from the MIT driver `windfreak-python` (`/home/marek/project/BenchMeshDOC/windfreak_synthhd/doc/`).
- Link: USB CDC (vid:pid not documented), baud irrelevant; **no tx terminator**, replies end LF
  (`device.py` l.45-75).
- Identify: `+` → model string; `-` serial; `v0` fw, `v1` hw (`Version 2.x`), `v2` → `HD`|`HDPRO`
  (`synth_hd.py` l.349-353, l.430-440).

| Operation | Command | Reply | Source |
|---|---|---|---|
| select channel | `C0` / `C1` (stateful), `C?` | | l.339 |
| frequency | `f<MHz %.8f>` / `f?` (MHz) | | l.340 |
| level | `W<dBm %.3f>` / `W?` | | l.341 |
| RF on (mute) | `h1|h0` / `h?` | | l.346 |
| PA power / PLL power | `r1|r0`, `E1|E0` | | l.347-348 |
| PLL lock | `p` | `1|0` | l.357 |
| reference | `x<n>` (0 ext, 1 int 27 MHz, 2 int 10 MHz) / `*<MHz>` ext freq | | l.355, l.359 |
| temperature | `z` | °C | l.358 |
| save | `e` (EEPROM) | | l.354 |
| sweep / AM / pulse / FM | `l u s t g c`, `F q A`, `P O R G j D`, `< > , ; /` | | l.362-391 |

- Quirks: every per-channel command needs `C{ch-1}` first; RF really on = `h1` **and** `r1` **and**
  `E1`; setters silent.
- Confidence: medium (community driver, not the vendor manual). DSL: **today** with a `before:
  [select_ch]` step whose param is computed `from: "ch - 1"` (check `ch` is available to `from`).

---

## 7. DAQ — data acquisition / scanning logger

### 7.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: DAQ
description: >
  Multi-channel scanning logger. Each configured scan channel is one
  registry channel (channel number = slot*100 + input, mapped in the
  definition); readings are SI value dicts in the unit of its function.
status:
  READING:  {type: si, required: true, description: "Latest reading of this scan channel"}
  FUNC:     {type: string, description: "Channel function (features.functions key: VDC, TEMP, RES…)"}
  ALARM:    {type: enum, values: [NONE, LO, HI]}
  SCANNING: {type: bool, description: "Scan running"}
  POINTS:   {type: integer, description: "Readings stored in memory"}
operations:
  scan_list:  {set: true, value: string, description: "SCPI channel list, e.g. (@101:110)"}
  configure:  {set: true, value: string, description: "Function for a channel (features.functions)"}
  scan:       {set: true, value: enum, values: [START, STOP]}
  interval:   {set: true, value: number, description: "Scan interval (s)"}
  readings:   {get: true, value: list, required: [get], description: "One reading per scan channel"}
  close:      {set: true, value: string, description: "Close relay channel(s) (multiplexer/actuator cards)"}
  open:       {set: true, value: string}
```

`close` is `energize` (relay cards switch external circuits).

### 7.2 Panel

As on the DAQ970A "channel monitor": a table of scan channels (number, label, function, value,
sparkline, alarm dot), SCAN START/STOP, interval stepper, per-channel function selector; relay
cards as a grid of arm-to-enable micro switches.

### 7.3 Families

**Keysight DAQ970A / DAQ973A (and 34970A/34972A, SCPI-compatible)** — DAQ970A ~2 k€ + cards;
34970A used ~400–800 €, very common. Docs:
`/home/marek/project/BenchMeshDOC/keysight_daq970a/doc/DAQ970A_ProgrammingGuide_9018-04756.pdf`.
- Interfaces: DAQ970A USBTMC + LAN; 34970A RS-232 + GPIB (34970A manual not local; same SCPI).
- Identify: `*IDN?` → `Keysight Technologies,DAQ970A,MY12345678,A.02.04-…` (p149).

| Operation | Command | Notes | Page |
|---|---|---|---|
| configure | `CONF:VOLT:DC (@101:105)`, `CONF:TEMP TC,K,(@106)`, `CONF:RES`… | default 10 PLC | p88-89, p103 |
| scan list | `ROUT:SCAN (@101:110)`, `ROUT:SCAN (@)` clears | | p202, p212 |
| read | `READ?` → `+4.27150000E+00,…` one per channel | | p89 |
| memory | `INIT` then `DATA:REM? <n>` / `DATA:LAST?` / `DATA:POIN?` | 100 000 readings | p89, p108 |
| format | `FORM:READ:CHAN ON`, `FORM:READ:UNIT OFF`, `FORM:READ:TIME ON` | | p139 |
| trigger | `TRIG:SOUR TIM`, `TRIG:TIM <s>`, `TRIG:COUN` | | p332 |
| relays | `ROUT:CLOS (@201)` / `ROUT:OPEN` | | p201 |

- Quirks: `READ?` duration = channels × integration — set `timeout_ms` from the scan list; card
  type via `SYST:CTYP? <slot>` (p298) for `features`.
- Confidence **high**. DSL: single-channel `MEAS?`/`READ?` today; the scan reply into per-channel
  status needs **`numbers`** (or `split` by index per channel, which works today for a fixed
  scan list declared in the definition).

Other loggers seen: Pico TC-08 (driver DLL only), UNI-T UT325/UT320 (USB HID), cheap MAX31855
boards (ad-hoc firmware) — out of scope.

---

## 8. BAT — battery internal-resistance tester

### 8.1 Proposed contract

```yaml
schema: benchmesh.class/v1
class: BAT
description: >
  Battery tester measuring AC internal resistance (1 kHz) and open-circuit
  voltage simultaneously.
status:
  RES:      {type: si, required: true, description: "Internal resistance (ohm); null on overflow/fault"}
  VOLT:     {type: si, required: true, description: "Battery voltage (V); null on overflow/fault"}
  MODE:     {type: enum, values: [RV, R, V], required: true}
  RRANGE:   {type: string, description: "Resistance range or AUTO"}
  VRANGE:   {type: string}
  JUDGE:    {type: string, description: "Comparator result HI/IN/LO/OFF"}
  OVERLOAD: {type: bool}
operations:
  mode:        {set: true, value: enum, values: [RV, R, V], required: [set]}
  res_range:   {set: true, value: string}
  volt_range:  {set: true, value: string}
  auto_range:  {set: true, value: enum, values: [ON, OFF]}
  rate:        {set: true, value: string, description: "EXFAST/FAST/MEDIUM/SLOW"}
  zero_adjust: {set: true, value: enum, values: [RUN, CLEAR]}
  measurement: {get: true, value: si}
```

Panel: two segment readouts (mΩ and V) like the BT3562 LCD, comparator HI/IN/LO lamps, MODE
(ΩV / Ω / V), range steppers with AUTO, SPEED selector, 0 ADJ button.

### 8.2 Families

**Hioki BT3561A / BT3562A / BT3563A (+ BT3562, BT3563, -01 GPIB variants)** — 2–4 k€ new, the
reference cell tester in battery workshops. Docs:
`/home/marek/project/BenchMeshDOC/hioki_bt356x/doc/BT3562A981-12.pdf`.
- Models: max input BT3561A ±60 V, BT3562(-01) ±60 V, BT3562A ±100 V, BT3563 ±300 V (p11, 6);
  resistance ranges 3 mΩ…3000 Ω, voltage 6/60/100/300 V by model (p164, 159).
- Interfaces: RS-232C 9600/19200/38400 8N1, no flow; rx terminator CR+LF or CR, tx **CR+LF**
  (p105); LAN on A models (command port **23**, p111; TCP not implemented); GP-IB on -01. No USB
  (USB-RS232 cable).
- Identify: `*IDN?` → `HIOKI,BT3562,0,V1.00` (p132).

| Operation | Command | Notes | Page |
|---|---|---|---|
| mode | `:FUNC RV|RESistance|VOLTage` / `:FUNC?` | | p126, p137 |
| ranges | `:RES:RANG <0–3100>`, `:VOLT:RANG <−300–300>`, `:AUT ON|OFF` | | p126 |
| zero adjust | `:ADJ?` (→ `0|1`), `:ADJ:CLEA` | | p126 |
| rate | `:SAMP:RATE EXF|FAST|MED|SLOW` | | p126 |
| reading | `:FETC?` → `288.02E-3,1.3921E+0` (ΩV) | no trigger; use in polls | p162 (157) |
| triggered read | `:READ?` | error while `:INIT:CONT ON` | p163 (158) |
| local | `:SYST:LOC` | | p129 |

- Quirks: overflow returns `±1.0E+9` scaled per range (`±10.0000E+8`, `±1000.00E+6`), fault
  `+1.0E+10` (p164) → CEL `abs ≥ 1e9 → null, OVERLOAD`; **blanks inside numbers**
  (`-   7.51E+0`, p164) → `{replace: {from: " ", to: ""}}` before `number`; wait 100 ms after
  changing conditions (p163). Accepts all Hioki 3560 commands (p165).
- Confidence **high**. DSL: **today** (serial).

**Cheap testers (YR1035+, RC3563, SM8124…)** — 20–60 €, extremely popular for 18650 sorting.
YR1035+: forum reports the USB port only charges (akkudoktor.net). RC3563 "serial" version:
CH340/CP210x USB link, protocol only in a community wiki (not fetched); believed to stream frames
→ unknown. Not definable until someone captures the wire (scripts/wire_capture).

---

## 9. Shared command baselines (input for `extends`)

| Baseline | Members | Core |
|---|---|---|
| **Keithley 2400 SMU** | 2400 series native; 2450 (`*LANG SCPI2400`); B2900A (`:SYST:LANG "2400"`) | `:SOUR:FUNC`, `:SOUR:<f>:LEV`, `:SENS:<f>:PROT`, `:OUTP`, `:FORM:ELEM`, `:READ?` |
| **`:NUMeric` power meter** | Yokogawa WT300E, GW Instek GPM-8213 (likely GPM-8310/8330 too) | `:NUM:FORM ASC`, `:NUM:NORM:ITEM<x>`/`PRES`/`NUMB`, `:NUM:NORM:VAL?`, `:INP:VOLT/CURR:RANG` |
| **ChibiOS shell** | NanoVNA-H/H4/-D, tinySA (SAL) | echo + `\r\n` + prompt `ch> `, `info`, `sweep`, `data`, `capture`, `pause/resume` |
| **IEEE 488.2 SCPI generic** | all SCPI families above | `*IDN?`, `*RST`, `*CLS`, `SYST:ERR?` |

---

## 10. DSL features needed by blocked or partly blocked families

| # | Feature | Unblocks | What exactly |
|---|---|---|---|
| F1 | **`numbers` parse step** (ASCII list → `list` of f64; separators `,`/whitespace/newline) | NanoVNA `data`/`frequencies`, DAQ scan replies, SAL traces | Same step proposed in `SAL.md` §5; one implementation |
| F2 | **Modbus RTU link/framing** | PZEM-004T/014/016/017, many cheap meters/loads (also XY-series, Riden) | Framing that builds `addr, fn, reg_hi, reg_lo, cnt_hi, cnt_lo, CRC16(lo,hi)` from a command spec `{modbus: {fn: 4, reg: 0, count: 10}}`, expects `5 + 2n` bytes, checks CRC; parse step `{registers: [{name, reg, type: u16|u32_lswf, scale}]}`; config `unit` (slave id, default 0xF8 for single device) |
| F3 | **Binary frame templates with checksum** | ZKETECH EBC-A20 (ELL), RC3563 (if binary) | `send` as byte template with `{sum8|xor|crc16}` placeholders, `reply: {bytes: N}` and a `{struct: [{name, offset, type, scale}]}` parse step |
| F4 | **HID link** | FNIRSI FNB58, ChargerLAB POWER-Z (USB power meters → PWR DC subset), UNI-T UT325 | `link: {hid: {vid, pid}}`, 64-byte reports as messages; usually also F3 for payload decoding |
| F5 | **Conditional poll** | Keithley 2400 (`:READ?` only with output on) | Poll entry `when: <CEL over earlier poll results>` → skipped commands yield null status instead of sending (safety: a read that would energize must never run) |
| F6 | **GPIB via USB adapters (Prologix / AR488)** | used 2400, 34970A, 53131A, HP 3478A… (the dominant used-market path) | Serial link + `session.on_connect` `++mode 1`, `++addr {config.gpib_addr}`, `++auto 1`, `++eos 2`; needs a **config-supplied variable** in templates |
| F7 | **TCP link** (already in schema, not implemented) | Hioki BT356x LAN (23), SSG3000X/DAQ970A/2450 LAN (5025), WT310E Ethernet | raw socket with the same terminator framing |
| F8 | Computed param from `ch` | Windfreak SynthHD `C{ch-1}` | confirm `ch` is in the `from` CEL environment |
| F9 | `samples: f32le` + header skip | NanoVNA binary `scan … 0x80`, faster sweeps | extend `samples` |

---

## 11. Local documents

| Folder (`/home/marek/project/BenchMeshDOC/…/doc/`) | Content |
|---|---|
| `keithley_2400` | 2400S-900-01 Rev K User's Manual (496 pp) |
| `keithley_2450` | 2450-901-01 Rev D Reference Manual (1003 pp) |
| `keysight_b2900` | B2900 SCPI Command Reference Ed. 8 (364 pp), B2900A Programming Guide |
| `siglent_ssg3000x` | SSG3000X Programming Guide E01B (102 pp) |
| `keysight_53200` | 53220A/53230A User's Guide (303 pp), programming comparison guide |
| `keysight_daq970a` | DAQ970A/DAQ973A Programming Guide (392 pp) |
| `yokogawa_wt300e` | IM WT310E-17EN Communication Interface manual (104 pp) |
| `gwinstek_gpm8213` | GPM-8213 User Manual Rev G (120 pp, remote chapter) |
| `peacefair_pzem004t` | PZEM-004T V3.0 user manual (Modbus map, 7 pp) |
| `hioki_bt356x` | BT3561A/62A/63A Instruction Manual (226 pp, ch. 8 commands) |
| `nanovna_nanovna` | NanoVNA-D firmware `main.c`, `usbcfg.c`, nanovna-tools README |
| `windfreak_synthhd` | windfreak-python 0.3.0 driver sources (command table) |
| (existing, reused) `rigol_dg1000z`, `rigol_dg800`, `siglent_sdg`, `feeltech_fy6900` | AWG built-in counter commands |

Keysight PDFs: the `keysight.com/<cc>/en/assets/…` links return an HTML wrapper; the file is at
`https://www.keysight.com/content/dam/keysight/en/doc/gate/<type>/<doc>.pdf`.
