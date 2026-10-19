# DMM coverage report

Class contract: `devices/classes/DMM.yaml` (status `MEAS` si, `MODE`, `RANGE`, `OVERLOAD`; operations
`mode`, `range`, `measurement`, `function`, `temp_sensor`, `temp_scale`, `rate`). Existing
definition: `devices/owon_xdm.yaml` (XDM1241 bench-tested). Research date 2026-10-18.

Manuals live in `/home/marek/project/BenchMeshDOC/<vendor>_<family>/doc/` (abbreviated `DOC/…`
below), each with a `SOURCES.md`. Page numbers are **PDF pages of the local file** unless
marked "printed". Everything below is from manuals or open-source code, not from hardware;
every new model starts as **Awaiting confirmation**.

Legend, DSL fit: **OK** = today's DSL (text over serial/USBTMC); **OK\*** = OK with a small
caveat noted; **TCP** = LAN only would need the unimplemented TCP link; **X** = needs a DSL
feature that does not exist (named).

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Type | Price band (new / used) | Protocol | Link | Doc quality | Confidence | DSL fit |
|---|---|---|---|---|---|---|---|---|
| 1 | UNI-T UT61E / UT61E+ (+ B+/D+, UT161x) | handheld | €50-80 | UT61E: ES51922 14-byte stream · UT61E+: binary `AB CD` frames | UT-D04 HID (CH9325) / UT-D09 HID (CP2110) / RS-232 opto | chip datasheet / community RE | high (code) | **X** (stream, HID, binary) |
| 2 | OWON XDM1041/1141/1241/2041, XDM3041/3051 | bench | €130-450 | SCPI-like (OWON dialect) | USB-CDC (CH340), RS-232; 30x1 + LAN | good (PM) | high (1241 tested) | **OK** (exists) |
| 3 | Siglent SDM3045X / SDM3055 / SDM3065X | bench | €450-900 | SCPI (Keysight style) | USBTMC, LAN | good | medium-high | **OK** |
| 4 | Rigol DM3058 / DM3058E / DM3068 | bench | €400-900 | SCPI ("RIGOL" set; AGILENT/FLUKE emulation) | USBTMC, RS-232 (+LAN/GPIB) | good | high (cmds) | **OK** |
| 5 | HP/Agilent 34401A | bench (used) | €150-400 used | SCPI (reference dialect) | RS-232 (DTR/DSR), GPIB | excellent | high | **OK\*** (handshake) |
| 6 | Keysight Truevolt 34460A/61A/65A/70A | bench | €1.1-3k / €600+ | SCPI | USBTMC, LAN | excellent | high | **OK** |
| 7 | Fluke 287/289 (+ 187/189, 87-IV/89-IV) | handheld | €450-650 + IR cable | ASCII command/response | IR-USB (FT232R) serial | official (189) / community (289) | high | **OK\*** (ack line) |
| 8 | Hantek HDM3055/3065/3075 | bench | €250-550 | SCPI (34461A clone) | USBTMC, LAN, RS-232/485 | medium | medium-high | **OK** |
| 9 | UNI-T UT8805E/UT8806E (SCPI) · UT8802E/8803E/8804E (binary) | bench | €80-700 | SCPI · binary HID | USBTMC/LAN/RS-232 · CP2110 HID | good · RE only | medium-high · medium | **OK** · **X** |
| 10 | GW Instek GDM-8341/8342/8351, GDM-9060/9061 | bench | €350-1.5k | SCPI (own `CONF:FUNC?` dialect; 906x = 34461A style) | USB-CDC (8351/906x also TMC), RS-232 | good | high (cmds) | **OK** |
| 11 | Keithley 2000 · 2100 · 2110 · DMM6500/DMM7510 | bench | €250-450 used · €600-1.5k | SCPI (2000 dialect; 2100 = 34401A-like; 6500 SCPI or TSP) | RS-232/GPIB · USBTMC · USBTMC/LAN | excellent | high | **OK** (6500: SCPI mode only) |
| 12 | Brymen BM867s/BM869s · BM257s | handheld | €100-260 + adapter | LCD-segment binary | BU-86X HID · BRUA-20X serial stream | official protocol PDFs | high | **X** (HID/stream/segments) |
| 13 | Rigol DM858 / DM858E | bench | €300-500 | SCPI (Keysight style) | USBTMC, LAN | good | high | **OK** |
| 14 | BK Precision 2831E/5491B · 5492B · 5492C/5493C | bench | €450-1.3k | SCPI (Keithley-ish; 549xC Keysight-ish) | USB-CDC/RS-232 (char echo) · USBTMC | medium | medium | **OK\*** (echo off) |
| 15 | Keysight U1250 series (U1231A…U1253B, U127x/U128x) | handheld | €250-500 + U1173A | SCPI-like read-only | IR-USB serial | community code only | medium | **OK\*** (unsolicited `*N`) |
| 16 | Keysight U3606A/B (DMM + PSU) | bench | €400-700 used | SCPI | USBTMC, GPIB | excellent | high | **OK** |
| 17 | Tektronix DMM4040/4050 · DMM4020 | bench | €250-600 used | SCPI · Fluke-45 style | RS-232, LAN(3490), GPIB · RS-232 | good | high | **OK** · **X** (prompt line) |
| 18 | Siglent SDM4055A / SDM4065A | bench | €900-1.5k | SCPI (as SDM3000) | USBTMC, LAN | good | medium-high | **OK** |
| 19 | Chip-protocol handhelds: Cyrustek ES51922 (UT61E, …), Fortune FS9922 (UT61B/C/D, Victor 70C), FS9721 (VC-820, UT60E, TP4000ZC), UT71x, Victor 86C/70C USB, UT181A, Owon B35T | handheld | €20-250 | streaming binary / BLE | RS-232 opto, HID, BLE | datasheets + libsigrok | high (code) | **X** |
| - | Mastech MS8040, UT171A/B/C, Brymen BM235, Fluke 87V | - | - | unknown / none | - | none found | low | n/a |

Ranking rationale: the UT61E-class handhelds are by far the most common "meter with a PC cable"
(sigrok's most-requested devices), but they need DSL work. Among meters that fit today's DSL,
OWON XDM, Siglent SDM3000, Rigol DM3000, a used 34401A and the Truevolt series dominate hobby
benches (EEVblog "which bench DMM" threads), followed by Fluke 28x owners with the IR cable.

---

## 2. Shared baselines (input for a future `extends`)

The DSL has no `extends` yet; this is what a shared base would hold. Until then, each family
copies the block.

### 2.1 `scpi_dmm` - Keysight/34401A dialect (Truevolt, 34401A, Siglent SDM3000/4000A, Rigol DM858, Hantek HDM3000, GW GDM-906x, Keithley 2100, BK 549xC, UNI-T UT8805E/06E, Tek DMM4040/50 in SCPI mode, Rigol DM3000 in `CMDSET AGILENT`)

| Item | Wire text | Reply | Notes |
|---|---|---|---|
| identify | `*IDN?` | `<Vendor>,<model>,<serial>,<fw>` | match model with a case-insensitive regex |
| mode set | `CONF:VOLT:DC AUTO` / `CONF:VOLT:AC AUTO` / `CONF:CURR:DC AUTO` / `CONF:CURR:AC AUTO` / `CONF:RES AUTO` / `CONF:FRES AUTO` / `CONF:CAP AUTO` / `CONF:FREQ` / `CONF:PER` / `CONF:CONT` / `CONF:DIOD` / `CONF:TEMP <probe>,<type>` | none | `CONF` resets range/NPLC/trigger to defaults. `FUNC "VOLT:AC"` switches without reset (keeps per-function settings); prefer CONF for a known state |
| function query | `FUNC?` | quoted short form: `"VOLT"`, `"VOLT:AC"`, `"CURR"`, `"CURR:AC"`, `"RES"`, `"FRES"`, `"CAP"`, `"FREQ"`, `"PER"`, `"CONT"`, `"DIOD"`, `"TEMP"` | parse `[trim, unquote, map]` |
| config query | `CONF?` | `"VOLT +1.00000000E+01,+3.00000000E-06"` (function, range, resolution) | one query gives MODE **and** RANGE: `regex` `^"?([A-Z:]+) ([^,"]+)` |
| range | `[SENS:]<fn>:RANG <value>` · `…:RANG:AUTO ON` · `…:RANG?` | `+1.00000000E+01` | `<fn>` = `VOLT[:DC]`, `VOLT:AC`, `CURR[:DC]`, `CURR:AC`, `RES`, `FRES`, `CAP`; value in base SI units, meter snaps up to the next range |
| rate | `[SENS:]<fn>:NPLC <plc>` · `…:NPLC?` | `+1.00000000E+01` | allowed values differ per model (see families); not for AC/FREQ/CAP |
| reading | `READ?` (= INIT + FETC?), `MEAS:<fn>?` (= CONF + READ?), `INIT` + `FETC?` | `-4.27150000E+00` | with trigger source IMM (default), `READ?` returns one fresh reading; at 10 NPLC ≈ 200 ms; AC ≈ 1 s on older meters |
| overload | - | `+9.90000000E+37` / `-9.9E37`; `9.91E37` = NaN / "range change in progress" | `OVERLOAD: 'meas >= 9.9e37 \|\| meas <= -9.9e37'`; MEAS null in that case |
| temp | `CONF:TEMP {FRTD\|RTD\|FTH\|THER\|TC},<type>` · `UNIT:TEMP {C\|F\|K}` | | sensor names differ (Siglent `KITS90`, Keysight `K`) |
| errors | `SYST:ERR?` | `+0,"No error"` | useful for setter verification |
| remote | `SYST:REM` / `SYST:LOC` | none | **required on RS-232** (34401A, DMM4040, Keithley 2000); not needed on USBTMC |

Framing: USBTMC → `rx.mode: message`, `tx.terminator: "\n"`. Serial → `tx "\n"`, `rx terminator "\n"`, `strip_trailing "\r"`.

Proposed base status binding (CEL):
```yaml
status:
  MEAS: {expr: 'meas >= 9.9e37 || meas <= -9.9e37 ? null : meas', as: si}
  MODE: func
  RANGE: conf_range          # from CONF? regex, or "AUTO" when …:RANG:AUTO? = 1
  OVERLOAD: 'meas >= 9.9e37 || meas <= -9.9e37'
```

### 2.2 Dialect differences

| Aspect | Keysight / 34401A | Siglent SDM | Rigol DM3000 ("RIGOL") | Rigol DM858 | Keithley 2000 / DMM6500 | GW GDM-834x | OWON XDM | Fluke-45 style (DMM4020) |
|---|---|---|---|---|---|---|---|---|
| select | `CONF:VOLT:DC` | `CONF:VOLT:DC` | `:FUNCtion:VOLTage:DC` (no arg) | `CONF:VOLT:DC` | `:SENS:FUNC 'VOLT:DC'` | `CONF:VOLT:DC` | `CONF:VOLT:DC AUTO` | `VDC` |
| function query | `FUNC?` → `"VOLT"` | `FUNC?` → `"VOLT"` | `:FUNC?` → `DCV` (unquoted, own names) | `FUNC?` → `"VOLT"` | `FUNC?` → `"VOLT:DC"` (long form) | `CONF:FUNC?` → `VOLT` (unquoted) | `FUNC?` → `"VOLT AC"` (space!) | `FUNC1?` → `VDC` |
| config query | `CONF?` quoted | `CONF?` quoted | - | `CONF?` **unquoted** | - | `CONF:RANG?` | (30x1: `RANGE1?` index) | `RANGE1?` index |
| range | SI value | SI value | **index** `:MEAS:VOLT:DC 0..4` | SI value | SI value | SI value via CONF | index `RANGE n` or CONF value | index `RANGE n` |
| auto | `:RANG:AUTO ON` | `:RANG:AUTO ON\|OFF\|ONCE` | `:MEASure AUTO` | `:RANG:AUTO ON` | `:RANG:AUTO ON` | `CONF:AUTO ON` | `AUTO` | `AUTO` |
| speed | NPLC | NPLC | `:RATE:<fn> F\|M\|S` | NPLC 0.4/5/20 | NPLC | `SENS:DET:RATE S\|M\|F` (query → SLOW/MID/FAST) | `RATE F\|M\|S` | `RATE S\|M\|F` |
| reading | `READ?` | `READ?` | `:MEAS:<fn>?` | `READ?` | `:READ?`/`:FETC?` (+unit elements!) | `VAL1?` (834x: `+0.333E-4,V DC`) | `MEAS1?` | `VAL1?` |
| overload | 9.9E37 | 9.9E37 | undocumented | 9.9E37 | 9.9E37 | undocumented (`-OL-` on screen) | undocumented | `±1.0E+9` |

---

## 3. Families

### 3.1 OWON XDM1041/1141/1241/2041 and XDM3041/3051 (already defined: `owon_xdm`)

- Existing definition covers XDM1241 (bench-tested), XDM1041, XDM1141 (awaiting). See
  `ai_context/migration/device-notes/owon_xdm.md` for the full command table.
- **XDM2041**: same programming manual (`DOC/owon_xdm/doc/XDM1000_Digital_Multimeter_Programming_Manual.pdf`,
  title page "XDM1041 & XDM2041"; 2041-only edition `XDM2041_Digital_Multimeter_Programming_Manual.pdf`
  = same text, adds `SYST:DATE?`/`SYST:TIME?`, CONF without MIN/MAX/DEF/AUTO keywords).
  Hardware (`XDM2041_USER_MANUAL.pdf`): **RS-232 DB9 only, no USB** (UM p14, p41), 115200 default,
  2400-115200 (p35-36); adds FRES and 50000 µF CAP (p24, p27). Action: add model
  `{name: XDM2041, match: "^OWON,XDM2041,"}` with a `features` override (FRES ranges, CAP to 50E-3); note
  the 2041 CONF without `AUTO` keyword - the existing `CONF:VOLT:DC AUTO` mode setter must be
  verified on a 2041 (fallback: `CONF:VOLT:DC` then `AUTO`).
- **XDM3041/XDM3051** (`DOC/owon_xdm/doc/XDM_Series_Digital_Multimeter_Programming_Manual.pdf`, 47 pp.;
  `XDM_Series_Digital_Multimeters_USER_MANUAL.pdf`):
  - `*IDN?` → `OWON,XDM3051,1546011,V2.0.2.0,2` (PM p11); last field `{1|2}`.
  - `FUNC?` → same values as 1x41 (`VOLT AC`, `VOLT`, `CURR AC`, `CURR`, `FREQ`, `PER`, `CAP`, `CONT`, `DIOD`, `FRES`, `RES`, `TEMP`) (p14-15).
  - **Range is queryable**: `RANGE1?` → index (p43-44), and `SENS:VOLT:DC:RANG?` → scientific (p15-16). Solves the 1x41 shadow-state problem.
  - Ranges XDM3051 (5½, 200 k counts): DCV 200m/2/20/200/1000; ACV …/750; DCI 200µ/2m/20m/200m/2/10 A; ACI 20m/200m/2/10 A; Ω/FRES 200/2k/20k/200k/2M/10M/100M; CAP 2n…200µ, 10 mF. XDM3041 (4½, 60 k): 600 mV/6/60/600 V… pattern.
  - Temperature: 14 sensor indices (KITS90 … Cu50). `RATE F|M|L` (verify `S` as on 1241), `MEAS1?`, `MEAS2?` (p45-46), `SYST:REM/LOC` (p41).
  - Interfaces: LAN, USB device, USB host, RS-232; baud 1200-115200, **default 9600** (UM p45); LAN port user-set (TCP, not implemented). **USB device class (CDC vs TMC) and VID:PID undocumented** (OWON USBTMC usually `0x5345:0x1234`; verify).
  - Overload: screen "overload" (UM p19-29); remote form undocumented (same open question as 1x41).
- Confidence: high for commands (shared with tested 1241), low for 30x1 USB class.
- DSL fit: **OK**. Separate definition `owon_xdm3000` recommended (different baud default, range query, 14 temp sensors), copying the 1x41 command block.

### 3.2 Siglent SDM3045X / SDM3055 / SDM3065X (-SC = scanner card)

Docs: `DOC/siglent_sdm3000/doc/SDM3065X_Remote_Manual_RC06036-E01A.pdf`, `SDM3055_RemoteManual_RC06035-E01A.pdf`,
`SDM3045X_ProgrammingManual_EN.pdf`. Cites are SDM3065X manual unless noted.

| | SDM3045X | SDM3055 (also 3055X-E) | SDM3065X |
|---|---|---|---|
| Digits / counts | 4½ / 60 k | 5½ / 240 k | 6½ / 2.2 M |
| DCV | 600m, 6, 60, 600, 1000 V (3045X p44) | 200m, 2, 20, 200, 1000 V (3055 p43) | same as 3055 (p44) |
| ACV | …750 V | …750 V | …750 V |
| DCI/ACI | 600µ, 6m (DC only), 60m, 600m, 6, 10 A (p40) | 200µ, 2m, 20m, 200m, 2, 10 A (p39) | same as 3055 (p40) |
| Ω / FRES | 600, 6k, 60k, 600k, 6M, 60M, 100M (p42) | 200, 2k, 20k, 200k, 2M, 10M, 100M (p41) | 200 … 1M, 10M, 100M (p42) |
| CAP | 2n…10000µ (p45) | 2n…10000µ (p44) | 2n…100 mF (p45) |
| NPLC (DC V/I, Ω) | 0.3, 1, 10 (p61, p83) | 0.3, 1, 10 (p60, p83) | 100, 10, 1, 0.5, 0.05, 0.005 (p61, p84) |

- Interfaces: USBTMC, LAN (VXI-11; raw 5025 likely, unverified), optional GPIB; **no RS-232**.
  USB `0xF4EC:0xEE38` (SDM3055, sigrok udev rules; the same PID is shared with SDS1104X-E - always
  match on IDN). PIDs for 3045X/3065X unknown.
- `*IDN?`: no example in the manuals; expect `Siglent Technologies,SDM3055,<serial>,<fw>` (sigrok vendor alias). Low confidence.

| Op | Wire | Reply | Page |
|---|---|---|---|
| mode | `CONF:VOLT:DC AUTO`, `CONF:VOLT:AC AUTO`, `CONF:CURR:DC AUTO`, `CONF:CURR:AC AUTO`, `CONF:RES AUTO`, `CONF:FRES AUTO`, `CONF:CAP AUTO`, `CONF:FREQ`, `CONF:PER`, `CONF:CONT`, `CONF:DIOD`, `CONF:TEMP RTD,PT100` / `CONF:TEMP THER,KITS90` | none | p37-45 |
| function | `FUNC?` | `"VOLT"`, `"VOLT:AC"`, `"CURR"`, `"CURR:AC"`, `"RES"`, `"FRES"`, `"CAP"`, `"FREQ"`, `"PER"`, `"CONT"`, `"DIOD"`, `"TEMP"` | p56 |
| config | `CONF?` | `"VOLT +2.00000000E-01"` (quoted) | p39 |
| range | `VOLT:DC:RANG 2` (also `VOLT:AC`, `CURR:DC`, `CURR:AC`, `RES`, `FRES`, `CAP`) · `…:RANG:AUTO ON\|OFF\|ONCE` · `…:RANG:AUTO?` → `0`/`1` | `+2.00000000E+00` | p61, p82-83 |
| rate | `VOLT:DC:NPLC 10` (`CURR:DC`, `RES`, `FRES`) | `+1.00000000E+01` | p84 |
| impedance | `VOLT:DC:IMP 10M\|10G` | `10M` | p84 |
| reading | `READ?`; `DATA:LAST?` → `-4.79221344E-04 VDC` | `-5.21391630E-04` | p7-11, p46-48 |
| temp | thermocouples `BITS90 EITS90 JITS90 KITS90 NITS90 RITS90 SITS90 TITS90`; RTD `PT100`; `UNIT:TEMP C\|F\|K` | | p43, p77-78 |
| overload | - | `9.9E37` / `+9.90000000E+37`; `9.91E37` = no data | p35, p40-46 |

Quirks: CONF restores defaults (auto-zero OFF, auto range), FUNC keeps settings (p37, p56);
reading memory cleared on config change (p7); manuals copy-edited between models (3065X manual
still says "SDM3055x") - trust the range tables, not the prose. sigrok drives the 3055 with
`CONF:…`, `CONF?`, `INIT`, `READ?`. Confidence medium-high (commands), low (IDN/PID). DSL fit
**OK** (USBTMC, `unquote`, `CONF?` regex for RANGE). One definition with three models and
per-model `features` (ranges, NPLC list).

### 3.3 Siglent SDM4055A / SDM4065A

Docs: `DOC/siglent_sdm4000/doc/SDM4000A_RemoteManual.pdf` (both models), `SDM4065A_UserManual_EN01A.pdf`, datasheets.
Same dialect as SDM3000. Differences: SDM4065A 6½ / 2.2 M counts, SDM4055A 5½ / 220 k.
Ranges: DCV/ACV 200m…1000/750 V; DCI/ACI 200µ…10 A (4055A AC min 20 mA); Ω 200…100 MΩ (4065A 1 MΩ, 4055A 2 MΩ step);
CAP 4065A 2n…100 mF, 4055A …10 mF (p33-38, p69). `CONF?` → `"VOLT +2.00000000E-01,+2.00000000E-08"` (p32).
NPLC: 4065A {100, 10, 1, 0.1, 0.01, 0.001}, 4055A {10, 1, 0.01} = Slow/Medium/Fast (p55, p70). `SYST:LOC` (p75).
Interfaces USBTMC, LAN (VXI-11), USB-GPIB adapter, no RS-232; PID unknown. Confidence medium-high.
DSL fit **OK** - can share the SDM3000 command block (separate definition, different ranges/NPLC).

### 3.4 Rigol DM3058 / DM3058E / DM3068

Docs: `DOC/rigol_dm3000/doc/DM3058_ProgrammingGuide_EN_2021-11.pdf` (cited "p"), `DM3058_UserGuide_EN.pdf` ("UG"),
`DM3068_ProgrammingGuide_EN.chm` (HTML help; cite = topic file; extract with `7z x`), `DM3068_UserGuide_EN_2024-01.pdf`.

| | DM3058 | DM3058E | DM3068 |
|---|---|---|---|
| Digits | 5½, dual display | 5½ | 6½ |
| Interfaces | USBTMC, RS-232, GPIB, LAN (LXI-C, VXI-11) | USBTMC, RS-232 | USBTMC, RS-232, GPIB, LAN |
| USB | `0x1AB1:0x09C4` (from memory, **unverified**) | same? | `0x1AB1:0x0C94` (UG p128) |
| ACI | 20m, 200m, 2, 10 A | same | 200µ…10 A |
| CAP | 2n…10000µ | same | 2n…100 mF |
| Temp | "Any Sensor" only | same | TC/RTD/THER (UG p75) |

- `*IDN?` → `Rigol Technologies,DM3068,DM3A020100823,01.01.00.01.01.00` (CHM IDN.htm); match case-insensitively (demo code uses `RIGOL TECHNOLOGIES`).
- RS-232: 1200-115200, default 9600, parity None (8 bit) / Odd / Even (7 bit) (UG p87-88); only one of GPIB/USB/RS-232 active - **select RS-232 on the front panel** (UG p84, p93); RS-232 commands must end `\r\n` (CHM Overview). DM3068 optional RTS/CTS.
- `CMDSET {RIGOL|AGILENT|FLUKE}` (p12). Use **RIGOL** (default). AGILENT gives the §2.1 dialect.

| Op | Wire (RIGOL set) | Reply | Page |
|---|---|---|---|
| mode | `:FUNCtion:VOLTage:DC`, `:FUNCtion:VOLTage:AC`, `:FUNCtion:CURRent:DC`, `:FUNCtion:CURRent:AC`, `:FUNCtion:RESistance`, `:FUNCtion:FRESistance`, `:FUNCtion:FREQuency`, `:FUNCtion:PERiod`, `:FUNCtion:CONTinuity`, `:FUNCtion:DIODe`, `:FUNCtion:CAPacitance` | none | p41-45 |
| function | `:FUNCtion?` | `DCV` `ACV` `DCI` `ACI` `2WR` `4WR` `FREQ` `PERI` `CONT` `DIODE` `CAP` (DM3068 also `SENSOR`) - DM3058 summary table says `RESISTANCE`/`CAPACITANCE`: map both | p42-45 |
| reading | `:MEASure:VOLTage:DC?` (`:VOLTage:AC?`, `:CURRent:DC?`, `:CURRent:AC?`, `:RESistance?`, `:FRESistance?`, `:FREQuency?`, `:PERiod?`, `:CONTinuity?`, `:DIODe?`, `:CAPacitance?`) | `8.492853e-05` (lower-case `e` on 3058) | p58-74 |
| auto | `:MEASure AUTO` / `:MEASure MANU` | none | p58 |
| range | `:MEASure:VOLTage:DC <0..4\|MIN\|MAX\|DEF>` (sets manual) | none | p59-74 |
| range query | `:MEASure:VOLTage:DC:RANGe?` | index integer | p60 |
| rate | `:RATE:VOLTage:DC F\|M\|S` (also VOLT:AC, CURR:DC/AC, RES, FRES) | 3068 query: `FAST`/`MEDIUM`/`SLOW` | p76-81 |
| impedance | `:MEASure:VOLTage:DC:IMPEdance 10M\|10G` | | p60 |

Range index tables (p59-74; DM3068 from CHM):
- DCV 0=200 mV, 1=2 V, 2=20 V, 3=200 V, 4=1000 V; ACV same with 4=750 V.
- DCI 0=200 µA, 1=2 mA, 2=20 mA, 3=200 mA, 4=2 A, 5=10 A. ACI DM3058: 0=20 mA, 1=200 mA, 2=2 A, 3=10 A; DM3068 = DCI table.
- Ω / 4WR 0=200 Ω, 1=2 k, 2=20 k, 3=200 k, 4=1 M, 5=10 M, 6=100 MΩ.
- CAP DM3058: 0=2n, 1=20n, 2=200n, 3=2µ, 4=200µ, 5=10000µ F; DM3068: 0=2n … 8=100 mF.
- Rates DM3058 F 123 / M 20 / S 2.5 rdg/s (p77); DM3068 F 0.02 / M 0.2 / S 100 NPLC.

Quirks: overload shows "OVER LOAD" (UG p25); remote form **undocumented** in the RIGOL set - assume ≥9e37, verify.
`:MEAS:<fn>?` unavailable in dual-display (p58). `:SYST:FORM:DECI COMMA` would break parsing - leave DOT.
DSL fit **OK**: reading and range query are function-dependent → computed params
(`send: ":MEASure:{fn}?"`, `fn: {type: enum, from: func, values: {DCV: "VOLTage:DC", …}}`), which the
validator allows in polls. Range values: features value = index (`"2"`, display "20 V"). Confidence high (cmds), low (3058 PID, overload).

### 3.5 Rigol DM858 / DM858E

Doc: `DOC/rigol_dm858/doc/DM858_ProgrammingGuide_EN.pdf` (158 pp.). DM858: 125 rdg/s, 10 A, CAP to 10 mF; DM858E: 80 rdg/s, 3 A, CAP to 1 mF (p12).
USBTMC + LAN only (raw socket 5025, p132-133); PID unknown. `*IDN?` → `RIGOL TECHNOLOGIES,<model>,<serial>,<fw>` (p52).

| Op | Wire | Reply | Page |
|---|---|---|---|
| mode | `CONF:VOLT:DC`, `CONF:VOLT:AC`, `CONF:CURR:DC`, `CONF:CURR:AC`, `CONF:RES`, `CONF:FRES`, `CONF:CAP`, `CONF:FREQ`, `CONF:PER`, `CONF:CONT`, `CONF:DIOD`, `CONF:TEMP <probe>[,<type>]` | none | p34-44 |
| config | `CONF?` | `VOLT:AC 1.00000000E+01,1.00000000E-03` (printed **unquoted**; verify) | p35-44 |
| function | `FUNC?` | quoted `"VOLT"`, `"VOLT:AC"`, `"CURR"`, `"CURR:AC"`, `"RES"`, `"FRES"`, `"CAP"`, `"FREQ"`, `"PER"`, `"CONT"`, `"DIOD"`, `"TEMP"` | p102-103 |
| range | `VOLT:DC:RANG <v>` / `…:RANG:AUTO ON` / `…:RANG?` | `1.00000000E+00` | p119-120 |
| rate | `VOLT:DC:NPLC 0.4\|5\|20` (Fast/Medium/Slow, default 20) | | p34, p121 |
| reading | `READ?`; `DATA:LAST?` → `-5.07000000E-01 VDC` | `+3.0153000E-09` | p20-22, p59-60 |
| temp | FRTD/RTD 385/389/391/392; FTH/THER 2200…30000; TC (default) B/E/J/K/N/R/S/T; `UNIT:TEMP C\|F\|K` | | p22-23, p41-42 |

Ranges: DCV 100m/1/10/100/1000; ACV …750; DCI/ACI 100µ/1m/10m/100m/1 A + 3 A (858E) or 10 A (858);
Ω 100…10 M, 50 MΩ; CAP 1n…10 mF (p35-43). Overload `9.9E37` (p60). `CONF` resets settings; values without unit suffix (p18).
Confidence high. DSL fit **OK**.

### 3.6 HP/Agilent 34401A

Doc: `DOC/agilent_34401a/doc/HP_34401A_User.pdf` (236 pp.), `Migrating_34401A_RS232_to_34461A_5992-1455.pdf`.
6½ digits. DCV 100m/1/10/100/1000; ACV …750; DCI 10m/100m/1/3 A; ACI 1/3 A; Ω/FRES 100…100 MΩ; FREQ/PER 3 Hz-300 kHz; CONT, DIOD, ratio. **No temperature, no capacitance** (p18-19).

- `*IDN?` → `HEWLETT-PACKARD,34401A,0,XX-XX-XX` (p90); later units possibly `Agilent Technologies,34401A,…`.
- RS-232 (p148, p151-153): DB-9 DTE; 300-9600 baud (default 9600); **None/8 or Even/7 or Odd/7, 2 stop bits fixed** → `8N2` or `7E2`. The manual contradicts itself on the default parity (p148 None/8; p94/p102 Even/7) - let the user choose. **DTR/DSR handshake**: meter stops sending unless DSR is asserted; host must not overrun the ~100-char buffer. **`SYST:REM` mandatory before any I/O**; `SYST:LOC` on disconnect. Ctrl-C = device clear. Read every reply before sending the next command.
- Terminators: TX `\n` (or `\r\n`) (p157); RX on RS-232 `SD.DDDDDDDDESDD\r\n` (p159).
- Commands = §2.1 baseline: `CONF:VOLT:DC {<range>|MIN|MAX|DEF},{<res>}`, `FUNC "VOLT:DC"`, `FUNC?` (quoted; exact short form verify), `VOLT:DC:RANG`, `…:RANG:AUTO OFF|ON`, `VOLT:DC:NPLC {0.02|0.2|1|10|100}`, `DET:BAND {3|20|200}`, `ZERO:AUTO`, `ROUT:TERM?` → `FRON`/`REAR`, `READ?`, `MEAS:…?` (p106-121). Overload `9.90000000E+37` (screen `OVLD`, p62).
- Quirks: AC reading ≈ 1.02 s (sigrok timeout 1500 ms). `MEAS` without `?` errors.
- DSL fit **OK\***: `format: 8N2` / `7E2` supported by the transport (`parse_format`); `session.on_connect: SYST:REM`, `on_disconnect: SYST:LOC`; `rx.timeout_ms ≥ 1500`. **Handshake**: DSL has `dtr: high` but no DSR wait; with a standard null-modem cable meter DTR→PC DSR/CTS, so `flow: rts_cts` may work depending on wiring; or use the manual's no-handshake wiring at ≤1200 baud. Mark as a first-contact item. Confidence high.

### 3.7 Keysight Truevolt 34460A / 34461A / 34465A / 34470A

Docs: `DOC/keysight_34400/doc/Truevolt_Operating_Service_Guide.pdf` (586 pp.), `Truevolt_Datasheet_5991-1983EN.pdf`.

| | 34460A | 34461A | 34465A | 34470A |
|---|---|---|---|---|
| Digits / rate | 6½ / 300 rdg/s | 6½ / 1 k | 6½ / 50 k | 7½ / 50 k |
| DCI ranges | 100µ…3 A | 100µ…10 A | 1µ…10 A | 1µ…10 A |
| Ω max | 100 MΩ | 100 MΩ | 1 GΩ | 1 GΩ |
| Temp | RTD, THER | RTD, THER | + TC | + TC |

- Interfaces: USBTMC, LAN (5025 SCPI socket, 5024 telnet; p416), GPIB opt. VID `0x2A8D` (Keysight fw) / `0x0957` (Agilent fw); PIDs commonly quoted 34461A `0x1301`, 34465A `0x0101` - **unverified**, match by IDN.
- `*IDN?` → `Keysight Technologies,34465A,<serial>,h.ff.ff-pp.pp-mm.mm-gg.gg-bb-fp` (fw ≥2.0) or `Agilent Technologies,…` (p270-271). `SYST:IDEN HP34401A` makes it answer as a 34401A - a definition regex for 34401A would then match a Truevolt (fine: compatible dialect).
- Commands: §2.1 exactly. `CONF:TEMP {FRTD|RTD|FTH|THER|TC},{85|5000|E|J|K|N|R|T}` (p244-255); `FUNC?` quoted short form `"CURR:AC"` (p314); `CONF?` → `"VOLT +1.00000000E+01,+3.00000000E-06"` (p246).
- Ranges: V 100m/1/10/100/1000 (AC 750); I 100µ/1m/10m/100m/1/3/10 A (+1µ, 10µ DC on 65A/70A, p326); Ω 100…100 M (+1 G); CAP 1n…100 µF (p248-253).
- NPLC: 0.02, 0.2, 1, 10, 100 (60A/61A); + 0.06 (65A/70A); + 0.001/0.002/0.006 with DIG option; default 10 (p331).
- Overload `±9.9E37`; `9.91E37` NaN; open CONT/DIOD → 9.9E37 (p79, p232, p246).
- Quirk: resolution argument with AUTO range errors; 10 NPLC reading ≈ 0.2 s. Confidence high. DSL fit **OK**.

### 3.8 Fluke 287/289 and 187/189, 87-IV/89-IV

Docs: `DOC/fluke_28x/doc/Fluke_187_189_89IV_87IV_remote_interface.pdf` (official, 187/189/89-IV/87-IV). 287/289 have no
public spec; facts from libsigrok `src/hardware/fluke-dmm/`, N0ury/dmm_util, EEVblog "Going further with the Fluke 289"
(URLs in `SOURCES.md`).

| | 287/289 | 187/189 | 87-IV / 89-IV |
|---|---|---|---|
| Cable | IR189USB (FTDI FT232R, `0403:6001`) | IR189USB / IR serial | RS-232 IR cable powered by DTR low / RTS high |
| Serial | 115200 8N1 | 9600 8N1 | 9600 8N1 |
| Identify | `ID` → `0\r` + `FLUKE 289,V1.16,<serial>\r` | `ID` → `FLUKE 189,V…` | `ID` → `FLUKE 89,V0.39,123456789` |
| Reading | `QM` → `0\r` + `+1.2345E0,VDC,NORMAL,NONE\r` (value, unit, state, attribute) | `QM` → `0\r` + `QM,+47.66 KOhms\r` | same |
| Overload | state `OL` / `OL_MINUS` / `OPEN_TC` | text `Out of Range` | same |
| Remote control | none for function/range (rotary switch); settings via `QMP`/`MP` | `SF <code>` emulates buttons (11 Hold, 13 Rel, 16 Hz, 17 Range …) | same |

- Protocol: every command is answered with an **ack line** `0\r` (OK) or `1\r` (syntax error) and then, for queries, one data line ending `\r`.
- Units seen (289): `VDC`, `VAC`, `ADC`, `AAC`, `OHM`, `F`, `HZ`, `CEL`, `FAR`, `S`, `PCT`, `DBM`, `V_AC_PLUS_DC` … → map to MODE.
- MODE comes from the unit field; RANGE not reported on 189 (`QM` only); 289 `QDDA` returns full display data incl. range (comma list; more complex).
- DSL fit **OK\***: `tx "\r"`, `rx terminator "\r"`. The ack line: `skip_leading` is a **character set**, so `skip_leading: "0\r"` would also drop a leading `0` of the data (e.g. `0.0123E0` → `.0123E0`, still parses; `0E0` would not). Proper fix: a small `ack` framing option (e.g. `rx.ack: {ok: "0", error: "1"}`) or `reply: {lines: 2}` taking the last line. Confidence high (189 official; 289 code-confirmed).
- Price: 287/289 €450-650; IR189USB €100+ (clones €20-40). Very common with professionals; MODE read-only.

### 3.9 Hantek HDM3055 / HDM3065 / HDM3075 (HDM3000)

Docs: `DOC/hantek_hdm3000/doc/HDM3000_SCPI_Programming_Reference_EN_Ver1.03.pdf` ("PR"), `HDM3000_UserGuide_EN_Ver2.6.pdf` ("UG").
5½/6½/7½ digits; suffix S/A = USB + RS232/485, B + LAN, H + GPIB (UG p22). USB = USBTMC (Keysight IO, UG p95); LAN socket **5025** (UG p97).
`*IDN?` example, VID:PID and RS-232 settings **not documented**.

Near-verbatim 34461A dialect: `CONF[:VOLT]:AC|DC [<range>|AUTO|MIN|MAX|DEF[,<res>]]`, `CONF:CURR`, `RES|FRES`, `FREQ|PER`, `CAP`,
`CONT`, `DIOD`, `TEMP FRTD|RTD|FTH|THER[,<type>]` (TC types E/J/K/N/R/T, PR p18) (PR p13-18). `CONF?` →
`"VOLT, +1.00000000E+01, + 3.00000000E-06"` (**note comma after the function and stray spaces**, PR p14). `FUNC "VOLT:AC"`; `FUNC?` → `"CURR:AC"` (p19-20).
Ranges V 100m…1000 (AC 750); I 100µ/1m/10m/100m/1/3/10 A (+1µ/10µ on 3075); Ω 100…100 M; CAP 1n…100µ (PR p8-9; UG p14).
NPLC 0.02/0.2/1/10/100, default 10 (UG p14). `READ?`/`FETC?` → `-1.33748741E-01` (PR p49-51). `UNIT:TEMP C|F|K` (p48).
Overload `9.9E37` (PR p15-19). Confidence medium-high (cmds), low (IDN). DSL fit **OK** (USBTMC). Use a lenient regex for `CONF?`.
Popular: HDM3055 €250-350, HDM3065 €400-550.

### 3.10 UNI-T bench: UT8805E/UT8806E (SCPI) and UT8802E/UT8803E/UT8804E (binary HID)

Docs: `DOC/unit_ut880x/doc/` - `UT8805E_Series_Programming_Manual_unitrend.pdf`, `UT8806E_Series_Programming_Manual_unitrend.pdf` ("PM"),
`UT8805E_Data_Sheet.pdf`, `UT8805E_User_Manual.pdf`, `UT800E_UT8803E_UT8802E_Programming_Manual.pdf` (DLL only), data sheets/user manuals.

**UT8805E/UT8806E** (5½ digits): USBTMC, LAN (VXI-11), RS-232 (baud up to 115200/128000, N/O/E, 1/1.5/2 stop; UM p30), GPIB on 8806E.
Commands end `<NL>` (PM p3). `*IDN?` → `UNI-T UT8805A, UT1A13460051200, V0.01.0000` (8806E PM p6 - note "UT8805A", spaces after commas).
`FUNC "VOLT:AC"`; `FUNC?` → `VOLT: AC` (**unquoted, space after colon**, PM p27); `CONF?` → `VOLT:DC +2.00000000E+01` (p9).
Ranges (PM p19-28): V 200m/2/20/200/1000 (AC 750); I 200µ/2m/20m/200m/2/10 A; Ω 200…100 M; CAP 2n…10000µ. Overload `+9.90000000E+37`.
Speed 2.5 / 10 / 5k rdg/s (UM p14). DSL fit **OK** (`replace: {from: " ", to: ""}` then map). Confidence medium-high.

**UT8802E/UT8803E** (and UT8802N/8803N; probably UT8804E): **USB HID via SiLabs CP2110 `10c4:ea80`**. Official manual documents
only UNI-T's `uci.dll`. Wire protocol (philpagel/ut8803e `protocol.md`): CP2110 UART 9600 8N1; frames `AB CD <len> <payload> <sum16be>`;
meter **streams ~3 readings/s unprompted**; payload `0x02`, mode byte, range digit (ASCII), 6 ASCII value chars, 7 status bytes (OL, hold,
manual range, rel, max, min, diode). Commands = button presses, e.g. Hold `AB CD 04 46 00 01 C2`, Auto `AB CD 04 4A 00 01 C6`, ID `AB CD 04 58 00 01 D4`.
DSL fit **X**: needs HID/CP2110 link + binary frame mode + stream mode (§4). Popular (€80-200).

### 3.11 GW Instek GDM-8341 / 8342 / 8351 and GDM-9060 / 9061

Docs: `DOC/gwinstek_gdm8300/doc/GDM-8342_User_Manual_EN_Ver_D_20230823.pdf` (cited, covers 8341/8342), `GDM-8351_User_Manual_EN_Ver_B1.pdf`;
`DOC/gwinstek_gdm9000/doc/GDM-906X_User_Manual_EN_Ver_G_20240318.pdf`.

**GDM-834x / 8351:**
- 8341/8342: 50 k counts (8341 no temperature, p62); USB **virtual COM only** (p103); GPIB option on 8342. 8351: 120 k counts, 4-W Ω, USB TMC or CDC selectable, RS-232 (p94-95).
- Serial: 9600-115200 8N1, no flow, default 115200 (p103, p145). VID `0x2184`, PIDs `0x001A`/`0x003B`/`0x0058` from the driver INF (model mapping unknown).
- `*IDN?` → `GWInstek,GDM8342,00000000,1.0` (p141); `GWInstek,GDM8351,…` (8351 p136).
- Terminators: accepts LF, CR or CRLF; replies end `\r\n` (p110-111).

| Op | Wire | Reply | Page |
|---|---|---|---|
| mode | `CONF:VOLT:DC`, `CONF:VOLT:AC`, `CONF:VOLT:DCAC`, `CONF:CURR:DC`, `CONF:CURR:AC`, `CONF:CURR:DCAC`, `CONF:RES`, `CONF:FREQ`, `CONF:PER`, `CONF:CONT`, `CONF:DIOD`, `CONF:TEMP:TCO K`, `CONF:CAP` (8351 + `CONF:FRES`) | none | p117-119 |
| function | `CONF:FUNC?` | `VOLT`, `VOLT:AC`, `VOLT:DCAC`, `CURR`, `CURR:AC`, `CURR:DCAC`, `RES`, `FREQ`, `PER`, `TEMP`, `DIOD`, `CONT`, `CAP` (unquoted) | p119 |
| range | `CONF:VOLT:DC <value>` (rounds up) · `CONF:AUTO ON\|OFF` · `CONF:AUTO?` → `1`/`0` · `CONF:RANG?` | e.g. `50E+1` (= 500 Ω) | p110, p120 |
| rate | `SENS:DET:RATE S\|M\|F` | query → `SLOW`/`MID`/`FAST` | p128 |
| reading | `VAL1?` | 834x: `+0.333E-4,V DC` (**unit after comma**); 8351: number only | p135; 8351 p128 |
| temp | TC only J/K/T: `SENS:TEMP:TCO:TYPE`, `SENS:TEMP:RJUN:SIM 0-50`, `SENS:UNIT C\|F` | | p128-130 |
| remote | `SYST:REM`, `SYST:LOC`, `SYST:RWL` | | p139-140 |

Ranges 834x: DCV 0.5/5/50/500/1000, ACV …750; I 0.0005/0.005/0.05/0.5/5/10 A; Ω 500…50 M; CAP 5n…50µ. 8351 (p111): V 0.1/1/10/100/1000 (AC 750); I 0.01/0.1/1/10; Ω 100…100 M; CAP 10n…100µ.
Rates S/M/F (p31): DCV/DCI/Ω 5/10/40 rdg/s; 8351 DCV 10/40/320. `SYST:SCP:MODE NORM|COMP` (GDM-8246 compatibility) - assume NORM.
Overload screen `-OL-` (p151), remote form undocumented. DSL fit **OK** (`split: {sep: ",", index: 0}` for `VAL1?`).

**GDM-9060/9061** (6½): USB CDC or TMC, RS-232 9600-115200 8N1 (flow none/RTS-CTS/DTR-DSR), LAN raw TCP + telnet 3000 (p216, p239-246).
`*IDN?` → `GWInstek,GDM9061,000000000,M0.69B_S0.25B` (p246); user-overridable by `SYST:IDNStr` (p212) - matching may fail on customised units.
Serial/CDC reply terminator selectable, default CRLF; TMC fixed LF (p224). 34461A dialect: `CONF:…` (p257, p282-285), `CONF?` →
`"VOLT +1.00000000E+01,+1.00000000E-05"`, `FUNC?` quoted (p294); ranges V 100m…1000, I 100µ…10 A, Ω 100…100 M, CAP 1n…100µ (p32-57);
NPLC 0.006/0.0083/0.0125/0.025/0.05/0.15/0.6/1/3/12 (p299); `READ?` → `-1.13148354E-04` (p274); TC J/K/N/R/S/T/B/E, RTD PT100/D100/F100/PT385/PT3916 (p285).
Overload not found in text (likely 9.9E37). DSL fit **OK**. Confidence high.

### 3.12 Keithley 2000 · 2100 · 2110 · DMM6500 (DMM7510)

Docs: `DOC/keithley_2000/doc/2000-900_J-Aug2010_User.pdf`, `DOC/keithley_2100/doc/2100-900-01F_Aug_2018_user.pdf`,
`DOC/keithley_2110/doc/2110-901-01C_Aug2013_Ref.pdf`, `DOC/keithley_dmm6500/doc/DMM6500-901-01C_April_2026_Ref.pdf`. Keithley 2001: no manual obtained.

**Keithley 2000** (6½): DCV 100m…1000, ACV …750, DCI 10m/100m/1/3 A, ACI 1/3 A, Ω/FRES 100…100 M, FREQ, PER, TC J/K/T, CONT, DIOD (p56).
- `*IDN?` → `KEITHLEY INSTRUMENTS INC., MODEL 2000, xxxxxxx, yyyyy/zzzzz` (p122).
- GPIB (default) **or** RS-232, chosen on the front panel (p81). RS-232: 8N1 fixed, 300-19200 baud, **default 4800**, flow none/XON-XOFF, meter TX terminator LF/CR/LFCR selectable; **commands execute only after `\r`** (p85-86). `SYST:REM/LOC/RWL` on RS-232 (p148).
- Commands: `:SENS:FUNC 'VOLT:DC'` (`VOLT:AC`, `RES`, `FRES`, `CURR:AC`, `CURR:DC`, `FREQ`, `TEMP`, `PER`, `DIOD`, `CONT`) (p141), `:CONF:<fn>` (p134), `FUNC?` (quoted long form `"VOLT:DC"` per community; not shown), `:VOLT:DC:RANG <0…1010>`, `:RANG:AUTO`, `:<fn>:NPLC 0.01…10` (default 1), `:<fn>:DIG 4…7` (p142-145), `:TEMP:TC:TYPE J|K|T` (p145).
- Reading: `:FETC?` (may repeat), `:READ?`, `:DATA:FRES?` (fresh only) (p134-136). `:FORM:ELEM READ` (default) → number only; with UNIT → `+1.23456789E+00VDC` (p140, p159). Overload `+9.9E37` (p159).
- DSL fit **OK**: `tx "\r"` (or `"\r\n"`), `on_connect: [":SYST:REM", ":FORM:ELEM READ"]`, `:INIT:CONT OFF` advisable for `READ?`. Used €250-450.

**Keithley 2100** (6½): USBTMC **`05E6:2100`** (p138, p144). Languages Default / `A34401` (34401A emulation, p85). §2.1 dialect: `FUNC "VOLT:DC"`, `CONF:<fn>`, `RANG`, `RANG:AUTO` → `1`/`0`, NPLC **0.02/0.1/1/10** (p59-60, p99), `UNIT Cel|Far|K`, `TEMP:RTD:TYPE PT100|D100|F100|PT385|PT3916|USER|SPRTD` (p116-117), overload `9.90000000E+37` (p107). IDN not shown (expect `KEITHLEY INSTRUMENTS INC.,MODEL 2100,…`). DSL fit **OK**.

**Keithley 2110** (5½, dual display): VID `05E6`, PID `0x2110` by analogy (unconfirmed, p217). `*IDN?` → `KEITHLEY INSTRUMENTS INC.,MODEL 2110,1311126,01.00-01-01` (p113).
`FUNC "VOLTage[:DC]"` … `"CAPacitance"`, `"TCOuple"`, `"TEMPerature"` (p152; `FUNC2` secondary). NPLC 0.001/0.006/0.02/0.06/1/2/10/100 (p129). TC B/C/E/J/K/N/R/S/T, RTD, NTC (p129-131). Overload `9.90000000E+37` (p181). DSL fit **OK**.

**DMM6500** (6½, touchscreen; DMM7510 7½ same model, manual not downloaded): USBTMC **`05E6:6500`** (p55), LAN 5025 (p50), GPIB/RS-232 only via KTTI cards.
`*IDN?` → `KEITHLEY INSTRUMENTS,MODEL DMM6500,04089762,1.6.3d` (p1242). **`*LANG {TSP|SCPI|SCPI2000|SCPI34401}`**, reboot required (p1254-1255); factory SCPI; definitions must check `*LANG?` on connect (`session.on_connect` + `requires`, or refuse) because SCPI fails in TSP mode.
`:SENS:FUNC "VOLT:DC"` … (p599), `:<fn>:RANG <n>|DEF|MIN|MAX` (p564), `:<fn>:NPLC 0.0005…15` (p559), `:TEMP:TRAN TC|RTD|FRTD|THER` (p562), `:MEAS?` = `:READ?` (p459), overload `9.9e+37` (p565).
Ranges: DCV 100m…1000, DCI 10µ…3 A (10 A rear), Ω 10…100 M (4W 1 Ω…), CAP 1n…1 mF (p565). DSL fit **OK** (SCPI mode). Confidence high.

### 3.13 Brymen BM867s/BM869s and BM257s

Docs: `DOC/brymen_bm86x/doc/BM860s_protocol.pdf`, `DOC/brymen_bm25x/doc/BM250s_protocol.pdf` (official).
- **BM86x**: BU-86X kit (BC-86X IR + Cypress HID `0820:0001`); request = HID report `00 86 66`; reply 27 bytes (24 data) in three 8-byte reports; **raw LCD segment bytes** (segment order bits 7:1, decimal point in next byte bit 0); bytes 16-18 = 0x86 as sync; dual display; overload = "L" segment pattern. No remote control.
- **BM257s/BM25x**: BRUA-20X IR (PL2303), IR receiver powered by DTR=1, RTS=1; 9600 8N1; meter **streams** 15-byte frames after power-on with HOLD pressed; byte 1 = 0x02; high nibble = byte position; low nibble = segments. No terminator. Manual: "clear buffer, read 15 bytes".
- DSL fit **X**: HID link (BU-86X), stream mode, segment decoder. Confidence high (official docs).

### 3.14 BK Precision 2831E/5491B, 5492B, 5492C/5493C

Docs: `DOC/bk_2831e/doc/2831E_5491B_manual.pdf`, `2831E_5491B_added_commands.pdf`; `DOC/bk_549x/doc/5492B_manual.pdf`, `5490C_Series_programming_manual.pdf`, `5490C_Series_manual.pdf`.

- **2831E (4½) / 5491B (50 k)**: USB virtual COM (+RS-232 on 5491B), 600-38400, default 9600 8N1, TX terminator LF/CR menu (p39-42). `*IDN?` → `2831E Multimeter,Ver1.0.09.12.03` - **no vendor field** (p64). **Character echo** (RETURN menu; p11, p42): every received char is sent back and the host is expected to wait for it - turn echo **off** on the meter. `FUNC VOLT:AC|VOLT:DC|CURR:AC|CURR:DC|RES|FREQ|PER|DIOD|CONT` (p48); `:VOLT:DC:RANG[:UPP] <n>`, `:RANG:AUTO <b>` (p49-51); `:NPLC 0.1|1|10` (25/10/5 rdg/s, p49, p65); `FETC?` → `+1.2345E+0` (p42, p63). Ranges 2831E V 0.2/2/20/200/1000 (AC 750), I 2m/20m/200m/2/20 A, Ω 200…20 M; 5491B V 0.5/5/50/500/1000, I 5m…20 A, Ω 500…50 M (p18-23). No CAP/TEMP. Overload undocumented.
- **5492B** (discontinued): Keithley-2000-like: `SENS:FUNC 'VOLT:DC'`, `CONF?` → `volt:dc` (p68), `FETC?`/`READ?` (p69), NPLC 0.1-10; ranges V 120m/1.2/12/120/1000, I 12m…12 A, Ω 120…120 M (p22-26); baud 2400-115200 (57600+ unstable), echo option (p57-59); `*IDN?` → `5492B Digital Multimeter, Ver1.0.00.00.01,123A45678` (p99).
- **5492C/5493C**: USBTMC (UM p45), RS-232 4800-115200 (echo handshake again, p43-44), LAN port configurable (example 45454). `*IDN?` → `BK Precision,549XC,<serial>,5.0.1.3.9R3` (PM p7). `CONF[:VOLT]:DC 100m|1|10|100|1000|AUTO`, `CONF:CURR:DC|AC 100uA…3A,10A`, `CONF:RES|FRES 10…100M`, `CONF:CAP 1n…10m`, `CONF:TEMP FRTD|RTD|FTH|THER` (p10-12); `CONF?` → `DCI, 1.00000000E-04, 1.00000000E-09` (own labels, no `FUNC?`, p9); NPLC 0.02/0.2/1/10/100 (UM p17); `READ?` (p17-18). PDF uses `-` for `|` in syntax.
- DSL fit **OK\*** with echo off (or USBTMC on 549xC). An "echo" serial mode would be needed to support echo-on units. Confidence medium.

### 3.15 Keysight U1250 handhelds (U1231A…U1253B, U1241C, U127x, U128x) + U1173A IR cable

No official remote document; source libsigrok `src/hardware/agilent-dmm/{api.c,protocol.c}` (URL in `DOC/keysight_u1200/doc/SOURCES.md`).
- Link: U1173A IR-USB serial, **9600 8N1** (U127x also 19200). sigrok sends `*IDN?\r\n` and other commands as `cmd\n\r\n` (workaround); replies end `\n`.
- `*IDN?` → `Agilent Technologies,U1253B,<serial>,<fw>` or `Keysight Technologies,…`.
- Read-only meter (rotary switch). `FETC?` (`FETC? @2`/`@3` secondary) → `+1.23456789E+00`; overload `±9.90000000E+37`. `CONF?` → U123x `"V,<res>,AC"` / `"RES,<n>"`; U124x/U125x/U128x `"VOLT:AC +5.00000000E+00,+1.00000000E-04"`, `"TEMP:K CEL"`. `STAT?` → quoted 21-char bit field.
- Unsolicited `*<digit>` line when the rotary switch moves; `*E` error.
- DSL fit **OK\***: MODE from `CONF?` regex; `mode`/`range` operations cannot be bound (class requires `mode.set` - **class contract conflict**: read-only meters need `mode` set to be optional or a `requires: false` refusal). Unsolicited lines would desync replies → validate replies with a regex and drain (needs a "discard lines matching" rx option). Confidence medium.

### 3.16 Keysight U3606A/B (DMM + 30 W PSU)

Doc: `DOC/keysight_u3606/doc/U3606B_Programming_Guide_U3606-90068.pdf` (printed page + 12 = PDF page), app note 5991-4383EN.
5½ digits. USBTMC + GPIB (p356), PID unknown. `*IDN?` → `Agilent Technologies,U3606B,KS08080027,00.12-00.42-00.20` (p350).
`CONF[:VOLT][:DC] [<range>|AUTO|MAX|MIN|DEF[,<res>]]`, `CONF:VOLT:AC`, `CONF:VOLT:ACDC`, `CONF:CURR[:DC]`, `CONF:CURR:AC`, `CONF:RES`, `CONF:LRES`, `CONF:CAP`, `CONF:DIOD`, `CONF:CONT`, `CONF:FREQ`, `CONF:PWID`, `CONF:DCYC` (p93-111, p183);
`FUNC?` → `VOLT:AC` (example unquoted - handle both). DCV 20m/100m/1/10/100/1000; DCI 10m/100m/1000m/3 A; Ω 100…100 M.
`READ?` → `+9.985308E+00`; overload `±9.900000E+37`, **`+9.910000E+37` = range change in progress** → MEAS null (p94-95). ~1 ms command gap (p13).
PSU part: `SOUR:VOLT:RANG {30 V|8 V|1 V|AUTO}`, `VOLT <v>`, `CURR <a>`, `SENS:VOLT?`/`CURR?`, `OUTP ON|OFF` (p180-272) → second class binding (PSU). DSL fit **OK**.

### 3.17 Tektronix DMM4040/4050 and DMM4020

Docs: `DOC/tektronix_dmm4000/doc/DMM4040_DMM4050_Programmer_077036300.pdf`, `DMM4020_Users_077036400.pdf`.
- **DMM4040/4050** (6½, Fluke 8845A/8846A rebadge): RS-232 default 9600 8N1 no flow (1200-230400, N8/O7/E7, 1/2 stop, XON or RTS/CTS), EOL CR/LF/CRLF (p20-21). **RS-232 mode must be COMPUTER** (TERMINAL echoes and prompts `3>`) (p22). LAN socket **3490** (p29) → TCP. `SYST:REM` on RS-232 (p81). `*IDN?` → `TEKTRONIX, DMM4040, 9963001, 05/19/09-13:59` (p84). `FUNC "VOLTage:DC"` etc., CONF/MEAS/READ? as 34401A (p82); NPLC 0.02/0.2/1/10/100 (p46-48); overload `9.90000000E+37` (p53). Fluke 45 language adds prompts - keep SCPI. DSL fit **OK** (serial).
- **DMM4020** (5½, RS-232 only): Fluke-45-style commands: `VDC`, `VAC`, `VACDC`, `ADC`, `AAC`, `AACDC`, `OHMS`, `WIRE2`/`WIRE4`, `FREQ`, `CONT`, `DIODE`; `FUNC1?` → `FREQ` (p85-86); `RANGE <n>` (V 1-5 = 200m/2/20/200/1000; Ω 1-7 = 200…100 M; DCI 1-6 = 200µ…10 A; ACI 1-4 = 20m…10 A), `RANGE1?`, `AUTO`, `AUTO?` (p89-90); `RATE S|M|F` = 2.5/20/100 rdg/s (p90); `VAL1?` → `+1.2345E+0`; **overload `±1.0E+9`** (p77, p91). 9600 8N1, echo off (p72). `*IDN?` → `TEKTRONIX, DMM4020, nnnnnnn, n.n Dn.n` (p74). **Every command is followed by a prompt line** `=>` (ok), `?>` (command error), `!>` (execution error) (p76).
  DSL fit **X**: needs "reply = data line(s) + prompt line" framing (`rx.prompt: {ok: "=>", error: ["?>", "!>"]}`). Workaround today: none clean (prompt arrives as a separate line and would shift replies).

### 3.18 Chip-protocol and binary handhelds (need new DSL features)

Docs: `DOC/cyrustek_es519xx/doc/ES51922.pdf` (serial output §4, p21-25), `ES51978.pdf`; `DOC/fortune_fs9721/doc/FS9922-DMM4-DS-13_EN.pdf`;
`DOC/unit_ut181a/doc/antage_ut181a_Protocol.md`; code: libsigrok `src/dmm/{es519xx,fs9721,fs9922,ut71x,bm86x}.c`, `src/serial_hid_{cp2110,ch9325,victor}.c`, `src/hardware/serial-dmm/api.c`, ljakob/unit_ut61eplus (URLs in each `SOURCES.md`).

| Meter (examples) | Cable / chip | Serial | Flow | Frame | Overload | Remote setters |
|---|---|---|---|---|---|---|
| UT61E (ES51922) | UT-D02 RS-232 (DTR=1, RTS=0 power) or UT-D04 HID (HE2325U `04fa:2490` / CH9325 `1a86:e008`) | 19230 7O1 | stream ~2/s | 14 bytes `0x30-0x3F` + CRLF: b0 range, b1-5 digits, b6 function, b7 status, b8-11 option flags | status bit 0 | none |
| UT61E+ / B+ / D+, UT161x | UT-D09 CP2110 HID `10c4:ea80` | 9600 8N1 | request `AB CD 03 5E 01 D9` | `AB CD <len>` mode, range, 7 ASCII display chars, flags, sum16be | display `OL.` | button codes `AB CD 03 <cmd> <sum16be>` (70 range, 71 auto, 72 rel, 74 hold, 76 select) |
| UT181A | UT-D09 CP2110 (needs SETUP→Communication ON) | 9600 8N1 | request + monitor stream | `AB CD <len16le> payload <sum16le>`, f32le values, variable layout | flag | set mode / range / record |
| UT71A-E, Voltcraft VC-920/940/960 | UT-D02 / UT-D04 | 2400 7O1 | stream | 11 bytes, b0-4 ASCII digits (`:` = OL), b5 range, b6 function, b7-8 flags, CRLF | `:` | none |
| UT61B/C/D, Victor 70C, PeakTech 2025 (FS9922) | UT-D02 / UT-D04 | 2400 8N1 | stream | 14 bytes: sign, 4 ASCII digits, space, DP, 4 flag bytes (may contain 0x0D/0x0A!), bargraph, CRLF | `?0:?` | none |
| VC-820/840, UT60A/E, MS8250B, TP4000ZC (FS9721) | RS-232 opto (DTR=1, RTS=0) / UT-D04 | 2400 8N1 | stream | 14 bytes, high nibble = position 1-14, low nibble = LCD segments | segments "0L" | none |
| Victor 86C / 70C USB | HID `1244:d237`, scrambled (subtract "jodenxunickxia", permute, bit-reverse) → FS9922 | - | stream | FS9922 | - | none |
| Owon B35T / B41T+ | BLE only (notify `fff4`, write `fff3`) | - | notify | FS9922 (old) / 3×u16 (new) | decimals `111` | new: button codes |

Unknown / not covered: Mastech MS8040 (no doc, not in sigrok), UNI-T UT171A/B/C (probably CP2110 `AB CD`, unverified),
Brymen BM235 (no PC interface known), Fluke 87V (no interface).

---

## 4. DSL features that blocked families

| Feature | Blocks | Minimal design |
|---|---|---|
| **Stream rx** (no command, newest complete frame) | ES51922/UT61E, FS9922, FS9721, UT71x, BM25x, UT8802E/8803E, UT181A monitor, Victor | `reply: stream` (or empty `send` + `rx.mode: stream`): flush input, read until a frame `{bytes: N, start: <byte/regex>, end: "\r\n"}` or nibble-position sync; return the newest |
| **Byte-field parse step → map** | all binary meters | `{bytes: {range: {at: 0}, digits: {at: 1, len: 5, as: ascii}, flags: {at: 7}, v: {at: 5, as: f32le}}}` then `bits` per field; or CEL `byte(raw, i)` |
| **HID link with UART-bridge adapters** | UT-D04 (CH9325/HE2325U), UT-D09 (CP2110), BU-86X, Victor | `hid: {vid, pid, bridge: cp2110\|ch9325\|bu86x\|victor, baud, format}` → exposes a byte stream so existing framing works |
| **Binary frame + checksum** | UT61E+, UT181A, UT8802E/8803E, (UT171) | `rx.mode: frame {magic: "ABCD", length: {at: 2, size: u8\|u16le}, checksum: sum16be\|sum16le}` + hex `send` with computed checksum |
| **7-segment decode step** | FS9721, BM25x, BM86x | `{segments: {table: <chip>, dp_bit: …}}` |
| **Ack / prompt lines** | Fluke 28x/18x (`0\r` ack), DMM4020 and Fluke-45 modes (`=>`/`?>`/`!>`) | `rx.ack: {ok: "0", error: ["1"]}` / `rx.prompt: {ok: "=>", error: ["?>", "!>"]}` consumed after each exchange |
| **Discard unsolicited lines** | Keysight U1250 (`*<n>`) | `rx.discard: "^\\*[0-9E]$"` |
| **Serial echo mode** | BK 2831E/5491B/5492B/549xC serial with echo on | `framing.echo: true` (read back each sent byte) - or document "turn echo off" |
| **TCP link** (declared, not implemented) | LAN-only use of every SCPI bench meter (ports 5025; DMM4040 3490; GDM-906x 3000; BK 549xC configurable); XDM30x1 LAN | none of the above families is LAN-only except DM858/SDM via LAN when no USB; all have USBTMC, so not blocking |
| **DSR wait** | 34401A RS-232 hardware handshake | `flow: dtr_dsr` (serialport supports only RTS/CTS natively; may need a cable workaround) |
| **Read-only meters vs class contract** | Fluke 28x, U1250, all chip-protocol handhelds | `DMM.yaml` makes `mode.set` required; needs `required: []` for meters with a rotary switch, or a `capabilities: read_only` flag |
| **BLE** | Owon B35T family | out of scope |

`extends` (shared `scpi_dmm` base) is not in the DSL yet; §2.1 lists its content.

---

## 5. Recommended definition order

1. `siglent_sdm3000` (3 models), `rigol_dm858`, `keysight_truevolt` (4 models), `hantek_hdm3000`, `unit_ut8805e` - all §2.1 over USBTMC; one shared command block; differences only in ranges/NPLC and `FUNC?` reply quirks.
2. `owon_xdm3000` (copy of `owon_xdm` + `RANGE1?`) and XDM2041 model entry in `owon_xdm`.
3. `rigol_dm3000` (RIGOL set, index ranges via computed params), `gwinstek_gdm8300`, `gwinstek_gdm9000`, `keithley_2100`, `keithley_2110`, `keithley_dmm6500` (SCPI).
4. Serial SCPI with session setup: `agilent_34401a`, `keithley_2000`, `tektronix_dmm4040`, `bk_2831e` (echo off).
5. After small DSL additions: `fluke_28x` (ack option; plus read-only class contract), `keysight_u1250` (discard rule).
6. After stream + byte-field + HID: UT61E (ES51922) first - largest community - then FS9922/FS9721 meters, UT61E+ (CP2110 + frame), Brymen.

Every first-contact check must confirm: `*IDN?` string, USB VID:PID, `FUNC?` quoting, the remote overload value,
settle time after a function change (OWON needed 1.5 s; CONF on others resets trigger), and range query format.
