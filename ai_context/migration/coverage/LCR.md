# LCR - LCR / impedance meters and component testers

Research 2026-10-18 for definitions written before we own the hardware (status **Awaiting
confirmation**). LCR has **no class contract yet**; section 4 proposes one (`devices/classes/LCR.yaml`
is deliberately not created) and the panel.

Manuals live outside the repo in `/home/marek/project/BenchMeshDOC/<vendor>_<family>/doc/`
(each with `SOURCES.md`). Page numbers below are the **printed** page numbers of the cited
manual (`PM` = programming manual, `UM` = user manual). Sources that are only readable online
(ManualsLib page viewer) are marked *online* and should be re-checked against the PDF before a
definition is marked anything but "Awaiting confirmation".

Not covered (checked): **Siglent** and **Rigol** sell no LCR meter (only DMMs with a capacitance
function). **Peak Atlas LCR45** has no data interface. **Keysight U1731C/U1732C/U1733C**,
**Hioki IM3523/IM3536**, **GW Instek LCR-800/LCR-8000G**, **IET/QuadTech 1900/7600**: no freely
downloadable command reference found this session (web search budget ran out); listed in the
ranking with what is known, to be researched later.

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Typical models | Price band | Interface → protocol | DSL fit today |
|---|---|---|---|---|---|
| 1 | **Cyrustek ES51919 handhelds** | DER EE DE-5000, UNI-T UT612, Mastech MS5308, PeakTech 2170 | €70-150 | IR/USB cable, one-way **binary 17-byte stream** | **No** (stream + binary framing) |
| 2 | **East Tester ET44/ET45** (+ RuoShui/Victor 4090/4091, Mustool ET4410) | ET4401, ET4402, ET4410, ET4501, ET4502, ET4510 | €100-250 | USB-CDC / RS-232, text SCPI-like | **Yes** |
| 3 | **Tonghui TH28xx** (+ OEM B&K 894/895) | TH2810B+, TH2817B+, TH2830, TH2832, TH2838; BK 894, 895 | €250-2000 | RS-232, USB-CDC, USBTMC, (LAN/GPIB), SCPI "E4980 dialect" | **Yes** (serial, USBTMC) |
| 4 | **B&K Precision 878B/879B/880 handhelds** | 880 (879B/878B believed same) | €200-350 | mini-USB virtual COM, text SCPI-like | **Yes** |
| 5 | **UNI-T UTR28xx benchtop** | UTR2810E+ (UTR2830/UTR2832 not verified) | €150-400 | RS-232/USB-CDC, own SCPI-like dialect | **Yes** |
| 6 | **Applent AT28xx / AT810A** (+ OEM GW Instek LCR-6000, RS PRO LCR-6000) | AT810A, AT2816A/B, AT2817(A), AT2818; LCR-6002/6020/6100/6200/6300 | €300-1500 | RS-232 / USB virtual COM (FTDI), SCPI "Applent dialect" | **Yes** (serial) |
| 7 | **B&K Precision 891** | 891 | €1400 | USB virtual COM, RS-232, GPIB, LAN, own SCPI dialect | **Yes** (serial); LAN needs TCP |
| 8 | **Keysight E4980A/AL** (+ HP 4284A, reference dialect) | E4980A, E4980AL | €8k+ (used €3k) | USBTMC, LAN, GPIB, SCPI | **Yes** (USBTMC); LAN needs TCP |
| 9 | **B&K Precision 889B** (legacy) | 889B (also "Model 4090" text in manual) | €600 (used) | USB virtual COM, own ASCII commands | Partly (values in display units) |
| 10 | **Voltcraft 4080 / PeakTech 2165** | LCR 4080, PeakTech 2165 | €80 (used) | RS-232/USB 1200 baud, polled **fixed 39-char ASCII** packet | Yes, awkward (range tables in CEL) |
| - | Keysight U1731C/U1733C | handheld, IR-USB cable | €300-500 | text over IR serial (not researched) | unknown |
| - | Hioki IM3523/IM3536 | benchtop | €3k+ | USB/LAN/RS-232/GPIB, Hioki command set (not researched) | unknown |

---

## 2. Families

### 2.1 Cyrustek ES51919 handhelds - DER EE DE-5000, UNI-T UT612, Mastech MS5308, PeakTech 2170

- **Models / clones**: libsigrok `serial-lcr` registers exactly these four on the ES51919 parser
  (`cyrustek_es51919/doc/serial-lcr-api.c` l.395-399). DE-5000 is by far the most popular hobby
  LCR meter (EEVblog favourite). Test frequencies 100 Hz, 120 Hz, 1 kHz, 10 kHz, 100 kHz (+ DC for
  DCR); 20000-count primary; secondary D, Q, θ, ESR/Rp.
- **Interface**: optical IR port + DER EE IR-USB cable (DE-5000), USB on UT612. USB chip / vid:pid
  **not confirmed** (sigrok wiki page has no IDs). Serial `9600/8n1/rts=1/dtr=1`
  (`libsigrok-internal.h` `ES51919_COMM_PARAM`; RTS/DTR high power the IR receiver).
- **Protocol**: **one-way**. The meter streams a 17-byte binary packet per measurement; there is
  no request, no identify ("No supported device provides a means to identify yet … they all just
  send", `serial-lcr-api.c` l.103-104). **No remote control at all** - read-only.
- **Packet** (`cyrustek_es51919/doc/es51919.c` l.33-124, reverse-engineered):

| Byte | Meaning |
|---|---|
| 0, 1 | header `0x00 0x0D` |
| 2 | flags: b0 HOLD, b1 reference shown, b2 delta (REL), b3 calibration, b4 sorting, b5 LCR-auto, b6 auto range, b7 **parallel** (else series) |
| 3 | b5-7 test frequency code: 0=100 Hz, 1=120 Hz, 2=1 kHz, 3=10 kHz, 4=100 kHz, 5=DC |
| 4 | sorting tolerance code (0 none, 3 ±0.25 %, 4 ±0.5 %, 5 ±1 %, 6 ±2 %, 7 ±5 %, 8 ±10 %, 9 ±20 %, 10 −20+80 %) |
| 5 | primary quantity: 1 L, 2 C, 3 R, 4 DCR |
| 6, 7 | primary value, **int16 big-endian** (0x4E20 = 20000 = outside limits) |
| 8 | b0-2 decimal exponent (value × 10^-n); b3-7 unit: 0 none, 1 Ω, 2 kΩ, 3 MΩ, 5 µH, 6 mH, 7 H, 8 kH, 9 pF, 10 nF, 11 µF, 12 mF, 13 %, 14 ° |
| 9 | b0-3 display status: 0 normal, 1 blank, 2 "----", 3 OL, 7 PASS, 8 FAIL, 9 OPEn, 10 Srt |
| 10 | secondary quantity: 0 none, 1 D, 2 Q, 3 ESR/Rp, 4 θ |
| 11-14 | secondary value/unit/status, same layout as 6-9 |
| 15, 16 | trailer `0x0D 0x0A` |

- **Quirks**: the trailer `0D 0A` can also occur inside value bytes, so line framing on `\n`
  mis-syncs; sync on header **and** trailer at a fixed 17-byte distance. Calibration/sorting mode
  packets carry no measurement (sigrok drops them, l.250-253).
- **Front panel (DE-5000)**: dual LCD (primary L/C/R + unit, secondary D/Q/θ/ESR); keys power,
  L/C/R (auto), D/Q/θ/ESR, FREQ, SER/PAR, RANGE/AUTO, HOLD, REL (Δ), SORT (tolerance), CAL
  (open/short), backlight, PC-link (IR).
- **Docs**: no official protocol; libsigrok source (GPL) - `cyrustek_es51919/doc/`. **Confidence:
  high** for the packet (sigrok ships it), low for USB IDs.
- **DSL fit: blocked.** What the DSL would need - concrete:
  1. **Unsolicited stream reading**: a poll command with no `send` (or `send: ""` explicitly
     allowed with `reply` only), where the engine reads the *latest complete* packet and discards
     older buffered ones (today the engine drains stray input as "late replies").
  2. **Packet framing**: `rx.mode: packet` with `header: "\x00\r"`, `length: 17`,
     `trailer: "\r\n"` - resync by sliding until header+trailer match at the right distance.
  3. **Byte-level parse** after framing: `{samples: u8}` already turns a raw reply into a list
     and CEL can index it, but CEL has no bitwise operators; either add `bits` over an indexed
     byte (`{bits: {byte: 2, b7: "7", freq: "5..7"}}`) or CEL helpers `bit(x, n)`,
     `field(x, lo, hi)`, plus `i16be(list, offset)`. Scaling needs a unit table (map step or CEL
     list literal, available today) and `10^-n` (CEL list literal `[1.0, 0.1, …][n]` works).
  4. A **read-only binding** of the LCR contract (no setters; `function`/`frequency` setters are
     `required` in the proposed contract → the contract must allow read-only models, or those
     operations become optional with features telling the panel to hide controls).

### 2.2 East Tester ET44 / ET45 (+ RuoShui / Victor 4090/4091, Mustool ET4410)

- **Models** (`easttester_et44/doc`; frequencies from ET44.py `instrument.py` l.10-21, levels from
  README "Voltage level"):

| Model | Frequencies (Hz) | Level (mV) |
|---|---|---|
| ET4401 / 4090A | 100, 120, 200, 400, 800, 1k, 2k, 4k, 8k, 10k | 100, 300, 600, 1000, 1500, 2000 |
| ET4402 / 4090B | as 4401 + 15k, 20k | same |
| ET4410 / 4090C | as 4402 + 40k, 50k, 80k, 100k | same |
| ET4501 / 4091A | 10 - 10 000, any integer | 10 - 2000 any integer |
| ET4502 / 4091B | 10 - 20 000 | same |
| ET4510 / 4091C | 10 - 100 000 | same |

  Primary L/C/R/Z/DCR/ECAP, secondary X/D/Q/θ/ESR, 5-digit both (UM p.2); DC bias 0-1500 mV; source
  resistance 30/100 Ω; open/short correction; comparator; list sweep (7 points).
- **Interfaces**: USB = **CDC-ACM** (`/dev/ttyACM0` in ET44.py examples, vid:pid unknown);
  RS-232 DB9 (UM p.17; firmware updates go over RS-232 with a PL2303 cable). **9600 8N1**, no
  flow control (PM p.2).
- **Identify**: `*IDN?` → `<manufacturer>,<model>,<firmware>,<hardware>,<SN>` (PM p.3); model
  field is space-padded, manufacturer varies by brand: `ELEC,ET4410         ,V6.00.2232.019,V1.12.2103.008,…`
  (EEVblog, Mustool unit). Match on the model field: `^[^,]*,\s*ET4410\b`.
- **Commands** (PM = `ET44_ET45_SCPI_20230306.pdf`):

| Operation | Set | Query → reply | Page |
|---|---|---|---|
| primary parameter | `FUNC:IMP:A {AUTO\|R\|C\|L\|Z\|DCR\|ECAP}` | `FUNC:IMP:A?` → token | 6 |
| secondary parameter | `FUNC:IMP:B {X\|D\|Q\|THR\|ESR}` | `FUNC:IMP:B?` → token | 6 |
| equivalent circuit | `FUNC:IMP:EQU {SER\|PAL}` | `FUNC:IMP:EQU?` → `SERIAL` / `PALLEL` (ET44.py l.245-262) | 6 |
| frequency | `FREQ <Hz>` (PM: `FREQuency:CW`) | `FREQ?` → NR3 Hz | 5 |
| level | `VOLT <mV>` (PM: `VOLTage:LEVel`, unit **mV**) | `VOLT?` → NR3 mV | 5 |
| DC bias | `BIAS:VOLT <mV>` 0-1500 | `BIAS:VOLT?` → NR3 mV | 4 |
| source resistance | `OUT:RES {0\|1}` (0=100 Ω, 1=30 Ω) | `OUT:RES?` | 4 |
| speed | `APER {FAST\|MEDium\|SLOW}` | `APER?` → token | 3-4 |
| range | `FUNC:IMP:RANG:AUTO {ON\|OFF}` (OFF = hold current range; no manual range) | `…AUTO?` → NR1 | 7 |
| relative (Δ) | `FUNC:DEV:MODE {ON\|OFF}` | `FUNC:DEV:MODE?` | 5-6 |
| min/max/avg | `FUNC:COM {MAX\|MIN\|AVG\|OFF}`; `FUNC:COMV?` → value or `OFF` | | 7 |
| trigger source | `SYST:SOUR {INT\|MAN\|EXT}` | `SYST:SOUR?` | 11 |
| trigger | `*TRG` | | 3 |
| correction | `CORR:EXEC` (open **or** short decided by the hardware; countdown on screen) | | 7 |
| read | - | `FETC?` → `<NR3>,<NR3>` e.g. `1e-3, 0.1025` | 8 |
| comparator | `COMP:STAT {ON\|OFF}`, `COMP:TOL:NOM <v>`, `COMP:TOL:RANG:1L -0.1` | `COMP:TOL:BIN?` → NR1 | 8-9 |
| remote/local | `SYST:REM` / `SYST:LOC` | | 11 |

- **Quirks** (from ET44.py, verified by its author on an ET4410):
  - **Every setter replies a line**: `exec success`, `cmd err` (unknown command) or `execu err`
    (failed) - `instrument.py` `write()` l.122-136. Setters must be `reply: line` with
    `parse: [trim, {map: {"exec success": OK}}]` so errors surface (unmapped → error).
  - Failed query replies `Rcmd err`.
  - Terminators: write `\r\n`, replies end `\r\n` (ET44.py defaults; PM says "0A frame tail",
    "carriage return to return").
  - Primary `AUTO` refuses B and EQU setters; `ECAP` resets level and bias to its own defaults
    (README "Primary mode"). DCR secondary behaviour undocumented.
  - Display still says "Local" while remote (EEVblog #4). Several firmware variants exist;
    TestController ships two different ET4410 definitions (EEVblog #19).
- **Docs**: official English SCPI PDF (13 p., machine-translated but complete) + real-device
  Python driver. **Confidence: high** (protocol and quirks seen on hardware by a third party).
- **DSL fit: yes.** `serial {baud: 9600, format: 8N1}`, `tx.terminator "\r\n"`, `rx terminator
  "\n" strip "\r"`. The pair-based `function` operation maps onto three setters with a step list
  (sketch in 4.4). `VOLT`/`BIAS` in mV → definition converts (`from:` or CEL `/1000`).

### 2.3 Tonghui TH28xx (+ OEM B&K Precision 894/895) - "E4980 dialect"

- **Models**: TH2810B+ (≤10 kHz), TH2817B+ (50 Hz-100 kHz) documented
  (`tonghui_th28xx/doc/TH2810B+_TH2817B+_operation_manual_cn_v1.1.pdf`); B&K 894 (20 Hz-500 kHz)
  and 895 (-1 MHz) documented (`bk_894/doc/894_895_programming_manual.pdf`) - same command tree,
  manual wording and FETC format as Tonghui, i.e. a Tonghui OEM. TH2830 (50 Hz-100 kHz,
  RS232/RS485/USB/HANDLER/GPIB per DirectIndustry), TH2832, TH2838 **not documented here** -
  same dialect assumed (Tonghui uses one SCPI tree across its 28xx line; TH2851 is advertised as
  E4980A/4284A command compatible).
- **Interfaces**: TH2817B+: bus selectable RS232C / USBTMC / USBCDC / GPIB / RS485, baud
  9 600-115 200, protocol selectable **SCPI / MODBUS / TH2810B** (legacy, fixed 19 200) - must
  be SCPI (UM p.60-62). BK 894/895: RS-232 + USBCDC + USBTMC + LAN (+GPIB 895), baud 9600, 19200,
  28800, 38400, 48000, 57600, 115200, 8N1, **no flow control - pace commands** (PM p.6-7).
  USBTMC vid:pid not documented.
- **Identify** `*IDN?`: Tonghui `<manufacturer>,<model>,<firmware>,<HW_version>` e.g.
  `Tonghui,TH2817B+,VER1.0.0,HardWare Ver A5.0` (UM p.115); BK `B&K Precision,895,<SN>,VER1.0.0,Hardware Ver XX.X` (PM p.9).
- **Commands** (TH = TH2817B+ UM, BK = 894/895 PM):

| Operation | Command | Reply | Page |
|---|---|---|---|
| function (pair) | `FUNC:IMP {CPD\|CPQ\|CPG\|CPRP\|CSD\|CSQ\|CSRS\|LPQ\|LPD\|LPG\|LPRP\|LSD\|LSQ\|LSRS\|RX\|ZTD\|ZTR\|GB\|YTD\|YTR}` (TH adds `RPQ RSQ ZQ`) | `FUNC:IMP?` → token | TH 92-94, BK 21 |
| range | `FUNC:IMP:RANG <Ω>` (turns AUTO off), values 3/10/30/100/300/1k/3k/10k/30k/100k (TH; BK from 10), suffix `OHM`/`KOHM` | `FUNC:IMP:RANG?` → value | TH 94, BK 21 |
| auto range | `FUNC:IMP:RANG:AUTO {ON\|OFF}` | NR1 | TH 94, BK 22 |
| frequency | `FREQ <v>[HZ\|KHZ\|MHZ]`, `MIN`/`MAX` (TH 50 Hz-100 kHz; BK 20 Hz-500 kHz/1 MHz) | NR3 | TH 90-91, BK 15 |
| level | `VOLT <v>[V\|MV]` (TH 100 mV-1 V; BK 5 mV-2 V); current level `CURR <v>MA` (BK 50 µA-) | NR3 | TH 91, BK 16 |
| source resistance | `ORES {10\|100}` (TH), `{30\|50\|100}` (BK) | NR1 | TH 91, BK 17 |
| speed + averaging | `APER {FAST\|MED\|SLOW}[,<1-255>]` (TH: 53/12/3 meas/s) | `FAST,<NR1>` | TH 100, BK 28 |
| trigger source | `TRIG:SOUR {INT\|EXT\|BUS\|HOLD}` | token | TH 101, BK 29 |
| trigger | `TRIG` / `*TRG` | | TH 101, BK 9/28 |
| trigger delay | `TRIG:DEL <s>` 0-60 s | NR3 | TH 102, BK 29 |
| read | `FETC?` → `SN.NNNNNESNN,SN.NNNNNESNN,SN[,SN]` = A, B, status, [bin if comparator on] | | TH 102-103, BK 30 |
| correction | `CORR:OPEN`, `CORR:SHOR` (sweep all preset points), `CORR:OPEN:STAT {ON\|OFF}`, `CORR:SHOR:STAT …`, `CORR:LOAD:…`, `CORR:SPOT<n>:…`, `CORR:LENG 0M` | | TH 104-106, BK 32-39 |
| DC bias (BK) | `BIAS:STAT {ON\|OFF}`, `BIAS:VOLT <v>` ±2 V, `BIAS:CURR <v>MA` ±50 mA | | BK 17-19 |
| deviation | `FUNC:DEV<n>:MODE {ABS\|PERC\|OFF}`, `FUNC:DEV<n>:REF <v>`, `…:REF:FILL` | | TH 95, BK 22-23 |
| monitors | `FUNC:SMON:VIAC {ON\|OFF}` (TH), `FUNC:SMON:VAC`/`IAC` (BK); `FETC:SMON:VAC?`, `IAC?` | | TH 95/102, BK 22 |
| comparator | `COMP[:STAT]`, `COMP:MODE`, `COMP:TOL:NOM`, `COMP:TOL:BIN<n>`, `COMP:SLIM`, `COMP:BIN:COUN:DATA?` | | BK 39-43 |
| page | `DISP:PAGE MEAS` (measurement display needed for FETC format 1) | | TH 89-90, BK 14 |

- **FETC status** (TH p.103, BK p.30): `-1` no data, `0` normal, `+1` analog bridge unbalanced,
  `+2` A/D not working, `+3` signal source overload, `+4` constant voltage not adjustable; with
  status −1/1/2 the data are `+9.99999E+37`. Bin: `0` out, `+1…+9` BIN1-9, `+10` AUX.
- **Quirks**: no setter replies documented → `reply: none`; buffer overruns without pacing (BK
  PM p.6) → `timing.gap_after_write_ms ≥ 50`. Measurement page must be shown or FETC returns the
  list-sweep format. TH2810B+ correction covers 4 preset points, TH2817B+ 11.
- **Front panel** (TH2817B+/TH2830 class): 3.5-4.3" LCD with soft keys; MEAS DISPLAY page shows
  FUNC, FREQ, LEVEL, RANGE, SPEED, BIAS fields, large A/B readouts, Vm/Im monitor, comparator
  BIN; keys MEAS, SETUP, SYSTEM, FILE, KEYLOCK (lit in remote), TRIGGER, RESET, numeric pad.
- **Docs**: TH manual Chinese (complete, 130 p.); BK programming manual English (complete).
  **Confidence: high** for TH2810B+/TH2817B+ and BK 894/895, **medium** for TH2830/2832/2838.
- **DSL fit: yes** (serial, USBTMC). LAN on BK 894 needs TCP.

### 2.4 B&K Precision 880 (878B/879B believed same) - handheld "impa/impb dialect"

- **Model**: 880: 100 Hz, 120 Hz, 1 kHz, 10 kHz, 100 kHz; level 0.3/0.6/1 V; primary L/C/R/Z/DCR,
  secondary D/Q/θ/ESR, SER/PAL, AUTO-LCR, tolerance 1/5/10/20 %, recording MIN/MAX/AVG
  (`bk_880/doc/880_manual.pdf` UM p.28-29). 878B/879B (more common, older) not documented here;
  same dialect expected - **verify** before reusing.
- **Interface**: mini-USB virtual COM, **9600 8N1**, no flow control (UM p.46); chip/vid:pid not
  stated (BK supplies its own driver installer, p.45).
- **Identify**: `*IDN?` → `<model>,<firmware>,<serial number>` (UM p.49) - no manufacturer field.
- **Commands** (UM p.49-53):

| Operation | Command | Reply |
|---|---|---|
| frequency | `FREQ {100\|120\|1000\|10000\|100000}` (or `1kHz` …) | `FREQ?` → `100Hz\|120Hz\|1kHz\|10kHz\|100kHz` (text with unit) |
| level | `VOLT {0.3\|0.6\|1}` (L/C/R/Z only) | `VOLT?` → `0.3V\|0.6V\|1V` |
| primary | `FUNC:impa {L\|C\|R\|Z\|DCR}` | `FUNC:impa?` → `L … DCR\|NULL` |
| secondary | `FUNC:impb {D\|Q\|THETA\|ESR}` | `FUNC:impb?` → `D\|Q\|THETA\|ESR\|NULL` |
| circuit | `FUNC:EQU {SER\|PAL}` | `FUNC:EQU?` → `SER\|PAL` |
| tolerance | `CALC:TOL:STAT {ON\|OFF}`, `CALC:TOL:RANG {1\|5\|10\|20}`; `CALC:TOL:NOM?`, `CALC:TOL:VALU?` (`-----` out of range) | |
| recording | `CALC:REC:STAT {ON\|OFF}`; `CALC:REC:{MAX\|MIN\|AVER\|PRES}?` → `NR3,NR3` | |
| read | `FETC?` → `NR3,NR3,NR1` (A, B, bin) or `NR3,NR1` for DCR | |
| lock | `*LLO` / `*GTL`; `*TRG` exists but is useless (continuous) | |

- **Quirks**: any command switches to remote (RMT) and **disables auto-fetch** (meter otherwise
  streams `A,B,bin\r\n` after every measurement when auto-fetch is on, p.46) - send a harmless
  query in `session.on_connect`. Terminator CR, LF or CRLF accepted; replies `…\r\n` (p.47-48).
  Errors only shown on LCD (E10/E11/E12, p.54) → setters `reply: none`. No speed command
  (FAST/SLOW only on the keypad), no open/short command.
- **Front panel / LCD** (p.14-17): dual display, keys FREQ/record, D/Q/θ/ESR (+level), L/C/R/Z/DCR
  (+auto), RATE/SER-PAL, HOLD, TOL, CAL (open/short); annunciators MAX/MIN/AVG, AUTO, D, Q, θ, ESR,
  deg, Ω, %, kHz, PAL/SER, unit glyphs, RMT, DH (hold), SLOW/FAST, 1V/0.6V/0.3V, TOL.
- **Docs**: English, complete for what exists. **Confidence: high** (880), low (878B/879B).
- **DSL fit: yes.** Frequency/level replies need a `map` parse to numbers.

### 2.5 UNI-T UTR2810E+ - own SCPI-like dialect

- **Model**: UTR2810E+: 100 Hz, 120 Hz, 1 kHz, 10 kHz; 0.1/0.3/1.0 V; source 30/100 Ω; ranges
  3 Ω-300 kΩ; 42 parameter combinations (`unit_utr2810e/doc/…programming_manual_cn_v1.0.pdf`).
  UTR2830/UTR2832 (100 kHz) not documented here.
- **Interface**: "RS232C commands" (p.3); baud not stated in the programming manual.
- **Identify**: `*IDN?` → `UNIT,UTR2810E+, CDB2024140001,REVA2.7` (p.12; manufacturer is `UNIT`).
- **Commands** (p.5-12):

| Operation | Command | Reply |
|---|---|---|
| function (pair) | `FUNC {L_Q\|C_D\|R_X\|Z_RAD\|G_B\|Y_R\|L_r…}` (only 7 of 42 tokens listed) | `FUNC?` → token |
| circuit | `MODE {SER\|PAR}` | `MODE?` |
| auto range | `FUNC:IMP:AUTO {ON\|OFF}` | `{ON,OFF}` |
| range | `FUNC:IMP:RANG {3\|10\|30\|100\|300\|1k\|3k\|10k\|30k\|100k\|300k}` | token |
| frequency | `FREQ {100\|120\|1k\|10k}` | token |
| level | `LEV:VOLT {1.0V\|0.3V\|0.1V}` | token |
| source R | `LEV:SRES {30\|100}` | token |
| speed | `SPEED {SLOW\|MEDium\|FAST}` (3 / 6.25 / 20 per s) | `SLOW\|MEDIUM\|FAST` |
| correction | `CORR:OPEN:STAT ON` then `CORR:OPEN` / `CORR:SHOR` (only with state ON) | Chinese text "开路清零成功" |
| trigger | `TRIG:SOUR {INT\|BUS\|MAN\|EXT}`; `TRIG` / `*TRG` → `TRIGger start` + one result | |
| read | `FETC?` → `<primary>,<secondary>` | |
| comparator | `COMP:STATA`, `COMP:STATB`, `COMP:MODE {ABS\|PER\|SEQ}`, `COMP:TOL:NOM`, `COMP:TOL:BIN<n>` | |

- **Quirks**: some setters reply text (correction, trigger) → those need `reply: line`;
  others undocumented → assume none. The token list is incomplete (42 pairs, 7 named).
- **Docs**: official Chinese, short (12 p.), incomplete. **Confidence: medium-low.**
- **DSL fit: yes.**

### 2.6 Applent AT28xx / AT810A (+ OEM GW Instek LCR-6000, RS PRO LCR-6000) - "Applent dialect"

- **Models**: Applent AT810A, AT2816A/B, AT2817(A), AT2818 share one user's guide
  (ManualsLib 1767875); GW Instek LCR-6002/6020/6100/6200/6300 are documented by the RS PRO
  LCR-6000 manual (ManualsLib 2964523) whose command reference is the Applent one (same
  subsystems, page names, FETC format, error codes). Model number = max frequency
  (6002 = 2 kHz … 6300 = 300 kHz) - **not verified**.
- **Interface**: RS-232 DB9 or USB virtual COM (Applent installs an **FTDI** driver, AT2818 UM
  p.60-61 *online*); baud 1200, 9600, 38400, 57600, 115200, 8N1 (LCR-6000 UM p.72 *online*);
  terminator LF (p.73, 99).
- **Identify**: `*IDN?` → `<model>,<firmware>,<serial no.>,<manufacturer>` - **manufacturer
  last** (LCR-6000 UM p.140 *online*).
- **Commands** (LCR-6000 UM *online*; AT2818 UM *online* where different):

| Operation | Command | Reply | Page |
|---|---|---|---|
| function | `FUNC {Cs-Rs\|Cs-D\|Cp-Rp\|Cp-D\|Lp-Rp\|Lp-Q\|Ls-Rs\|Ls-Q\|Rs-Q\|Rp-Q\|R-X\|DCR\|Z-thr\|Z-thd\|Z-D\|Z-Q}` (θ may be byte 0xE9) | `FUNC?` → `Cp-D` | 109 (AT2818: `FUNC[:IMP][:TYPE]` p.71) |
| auto range | `FUNC:IMP:AUTO {ON\|OFF}` (AT2818: `FUNC:IMP:RANG:AUTO {HOLD\|AUTO\|NOM}` p.72) | `on`/`off` lower case | 109 |
| range | `FUNC:IMP:RANG {0-8\|MIN\|MAX}` (range **number**, e.g. 2 = 10 kΩ) | NR1 | 109 |
| frequency | `FREQ <v>` (multiplier `k` ok, unit `Hz` **not** allowed) | `1.000000E+03` | 112 |
| level | `LEV:VOLT <v>` / `VOLT:LEV <v>` (no `V` suffix) | NR3 | 113 |
| source R | `VOLT:SRES {30\|50\|100}` (AT2818) | NR1 | AT2818 74 |
| speed | `APER {SLOW\|MED\|FAST}` (AT2818: `SLOW\|MED1\|MED2\|FAST`), `APER <avg 0-256>` | `APER?` → `slow,0`; `APER:RATE?` | 116 / AT2818 75 |
| monitors | `FUNC:MON1 {off\|Z\|D\|Q\|THR\|THD\|R\|X\|G\|B\|Y\|ABS\|PER\|VAC\|IAC}` | `FETC:MON1?` | 111, 120 |
| read | `FETC?` → `<A>,<B>,<compare>` e.g. `+2.61788e-11,+5.45442e-01,BIN1`; `FETC:MAIN?` → `<A>,<B>` | | 118 / AT2818 76 |
| trigger | `TRIG:SOUR {INT\|MAN\|EXT\|BUS}`; `TRIG` (BUS only); `*TRG` = `TRIG;:FETC?` → result | | 132, 140 |
| correction | `CORR:OPEN`, `CORR:SHOR`, `CORR:OPEN:STAT {ON\|OFF}` … (state syntax inferred) | | 129 |
| page | `DISP:PAGE {MEAS\|…}`; FREQ/LEV refused on LIST and CORRECTION pages | | 106, 112 |

- **Quirks**: SYSTEM CONFIG "RESULT" must be **FETCH**, not AUTO (AUTO streams a result after each
  measurement, p.75); "ERROR CODE" ON makes the meter answer `*E00`…`*E11` (p.74) - choose OFF
  with `reply: none`, or ON and parse (needs a capture). Replies lower case (`on`, `slow`).
  Comparator token in FETC (`BIN1`, `AUX`, `OUT`, `NG`) must be split off.
- **Docs**: no freely downloadable command reference (applent.com blocked, ICP filing); ManualsLib
  page viewer only (summaries, partly garbled). **Confidence: medium** (structure solid, exact
  tokens to be confirmed against a PDF/capture).
- **DSL fit: yes.**

### 2.7 B&K Precision 891 - own SCPI dialect

- **Model**: 20 Hz-300 kHz (resolution 0.01 Hz <100 Hz … 100 Hz ≥100 kHz), AC level 0.5 or 1 Vrms,
  DCR 1 VDC, ranges AUTO/HOLD only, speed SLOW/FAST, 2-4 display digits (`bk_891/doc/891_manual.pdf`
  UM p.21-24, 75-76). Pairs Cs-Q/D/R, Cp-Q/D/R/G, Ls-Q/D/R, Lp-Q/D/R/G, Z-θ, Y-θ, R-X, G-B, DCR (p.23).
- **Interface**: USB virtual COM (9600/19200/38400/57600, N/E/O 8, 1/2 stop, no flow - **add
  delays**, p.58-59), RS-232, GPIB, LAN (web UI, telnet 5024, raw socket **5025**, ≥100 ms between
  writes, p.64).
- **Identify**: `*IDN?` → `<Manufacturer>,<model>,<serial number>,<firmware>` (p.66).
- **Commands** (p.66-73): `MEAS:FUNC {CSQ|CSD|CSR|CPQ|CPD|CPR|CPG|LSQ|LSD|LSR|LPQ|LPD|LPR|LPG|ZTH|YTH|RX|GB|DCR}`
  (or index 0-18) + `?`; `FREQ <NRf>` + `?`; `LEV:AC {0.5|1}` + `?`; `MEAS:RANG {HOLD|AUTO}` + `?`;
  `MEAS:SPEE {SLOW|FAST}` + `?`; `FETC?` → displayed `<A>,<B>`; `MEAS:RESU?` → raw `<A>,<B>`;
  `CAL:OPEN`, `CAL:SHOR`, `CAL:BUSY?` → 0 done / 1 busy / −1 fail; `FORM {ASC|REAL}` (keep ASCII);
  `SYST:ERR?`; `DISP:PAGE MEAS`.
- **Quirks**: "All commands should be ended with `<Rtn><NL>`" and a space before parameters
  (p.64); RMT indicator + front panel locked until LOCAL key (p.58).
- **Docs**: English, complete. **Confidence: high.** **DSL fit: yes** (serial); LAN → TCP.

### 2.8 Keysight E4980A / E4980AL (reference "E4980 dialect", also HP 4284A)

- **Model**: 20 Hz-2 MHz (E4980AL: to 1 MHz / 300 kHz options), level 0-2 V / 0-20 mA (option 001:
  0-20 V / 0-100 mA), ranges 1, 10, 100, 300, 1k, 3k, 10k, 30k, 100k Ω (opt 001 adds 0.1), DC bias
  0/1.5/2 V (opt 001 ±40 V) - `keysight_e4980a/doc/qcodes_e4980a.py` l.218-242 (QCoDeS, citing
  User's Guide E4980-90230 p.353; the Keysight PDF is behind a bot wall).
- **Interface**: USBTMC (vid 0x0957; pid **not verified**, commonly reported 0x0909), LAN (raw
  SCPI 5025), GPIB.
- **Commands** (QCoDeS l.158-461): `:FUNC:IMP {CPD|CPQ|CPG|CPRP|CSD|CSQ|CSRS|LPD|LPQ|LPG|LPRP|LPRD|LSD|LSQ|LSRS|LSRD|RX|ZTD|ZTR|GB|YTD|YTR|VDID}`;
  `:FUNC:IMP:RANG <Ω>`, `:FUNC:IMP:RANG:AUTO {1|0}`; `:FREQ <Hz>`; `:VOLT:LEV <V>` / `:CURR:LEV <A>`;
  `:APER {SHOR|MED|LONG},<avg 1-256>`; `:BIAS:STAT {1|0}`, `:BIAS:VOLT:LEV <V>`;
  `:CORR:OPEN`, `:CORR:OPEN:STAT {1|0}`, `:CORR:SHOR`, `:CORR:SHOR:STAT`; `:FETC:IMP:FORM?` →
  `<A>,<B>,<status>`; `:FETC:IMP:CORR?` → `<R>,<X>`; `:TRIG:SOUR {INT|EXT|BUS|HOLD}`,
  `:TRIG:IMM`, `:INIT:CONT ON`; `*OPT?`, `SYST:ERR?`.
- **Confidence: medium-high** (secondary source, but a heavily used driver). **DSL fit: yes**
  (USBTMC, `rx.mode: message`); LAN → TCP.
- The Tonghui tree (2.3) is this tree without the leading colon, with `FAST|MED|SLOW` speed tokens
  and Tonghui-specific extras → shared baseline A (section 3).

### 2.9 B&K Precision 889B (legacy) - own ASCII commands

- **Model**: 100 Hz-200 kHz, 1 Vrms / 250 mVrms / 50 mVrms / 1 VDC, plus DCV/ACV/DCA/ACA
  (`bk_889b/doc/889B_manual.pdf` UM p.17-24). The manual's syntax section says "Models 4090".
- **Interface**: USB, 9600 8N1, half duplex, no handshake (p.20). Terminator CR or LF (p.20).
- **Identify**: `*IDN?` → `B&K PRECISION CORP. MODEL889B,123456789,4.096` (p.21).
- **Commands**: function setters `CpD`, `CpQ`, `CpRp`, `CsD`, `CsQ`, `CsRs`, `LpD`, `LpQ`, `LpRp`,
  `LsD`, `LsQ`, `LsRs`, `RsXs`, `RpXp`, `ZTD`, `ZTR`, `DCR` → reply `OK`; the same with `?` sets
  **and measures**: `CPD?` → `0.22724 0.12840` (space separated); `FREQ {100Hz|120Hz|1KHz|10KHz|100KHz|200KHz}`
  → `OK`, `FREQ?` → `1KHz` (with `ASC ON`); `LEV {1VDC|1Vrms|250mVrms|50mVrms}`; `CORR OPEN` /
  `CORR SHORT` → `OK` after completion; `*RST` replies the IDN (p.20-23).
- **Quirks**: readings are in the **display unit** (default `CpD uF`, p.21) with no unit in the
  reply - the range/unit must be tracked; the binary "Remote Binning" mode (`MOD <24-bit code>`,
  11-byte `02 09 <float32 LE ×2> <checksum>` frames, p.17-19) must be avoided.
- **Confidence: medium** (English manual, but unit handling unclear). **DSL fit: partly** (text is
  fine; scaling needs a unit query that the manual does not document).

### 2.10 Voltcraft LCR 4080 / PeakTech 2165 - polled fixed ASCII packet

- **Protocol** (`voltcraft_4080/doc/vc4080.c` l.37-118): request `N` (single char, l.635-641) →
  39-char ASCII packet ending `\r\n`: `[0]` L/C/R, `[1]` Q/D/R, `[2]` freq A=1 kHz/B=120 Hz,
  `[3]` P/S circuit, `[4]` A/M auto/manual, `[5:9]` primary digits (`8…` range switching, `9…` OL),
  `[10]` primary range code, `[11:14]` secondary, `[15]` secondary range, `[16]` sequence counter,
  `[17:21]` D, `[22:26]` Q, `[27:36]` flags (SETUP, FUSE, HOLD, MAX/MIN/AVG, REL, LIMITS, TOL,
  backlight, adapter, low battery). Serial 1200 baud; manual says 7E1, sigrok uses 8N1 and strips
  the parity bit (l.76-79).
- **DSL fit: yes but awkward** - `rx terminator "\n"`, `format: 7E1` is supported by the transport;
  value = digits × a range-code table that depends on L/C/R, frequency and circuit (CEL map
  literal). **Confidence: medium** (sigrok, two vendors' docs referenced in the source).

---

## 3. Shared command baselines (input for `extends`)

| Baseline | Members | Core | Differences to parameterise |
|---|---|---|---|
| **A. E4980 / Tonghui** | Keysight E4980A/AL, HP 4284A, Tonghui TH2810B+/TH2817B+ (TH2830/2832/2838 assumed), B&K 894/895 | `FUNC:IMP <CPD…>`, `FUNC:IMP?`, `FUNC:IMP:RANG[:AUTO]`, `FREQ`, `VOLT[:LEV]`, `CURR[:LEV]`, `APER <spd>[,avg]`, `TRIG:SOUR`, `TRIG`, `FETC?` → `A,B,status[,bin]`, `CORR:OPEN/SHOR[:STAT]`, `BIAS:STAT/VOLT`, `*IDN?` | speed tokens (`SHOR/MED/LONG` vs `FAST/MED/SLOW`), leading colon, limits, FETC subnode (`:FETC:IMP:FORM?` vs `FETC?`), extra pairs (RPQ, RSQ, ZQ, LPRD, VDID) |
| **B. Applent** | Applent AT810A/AT2816/AT2817/AT2818, GW Instek LCR-6000, RS PRO LCR-6000 | `FUNC <Cp-D…>`, `FUNC:IMP:RANG <0-8>`, `FREQ` (no Hz), `LEV:VOLT` (no V), `APER {SLOW,MED,FAST}`, `FETC?` → `A,B,compare`, `TRIG:SOUR {INT,MAN,EXT,BUS}`, `*IDN?` (manufacturer last) | auto-range syntax, MED1/MED2, error-code/result-mode settings |
| **C. A/B parameter handhelds** | East Tester ET44/ET45 (+RuoShui/Victor), B&K 880 (878B/879B?) | primary and secondary set separately (`FUNC:IMP:A`/`FUNC:impa`, `…:B`/`impb`), `…:EQU {SER\|PAL}`, `FREQ`, `VOLT`, `FETC?` → `A,B[,bin]` | ET44 setters reply `exec success`; BK 880 replies unit text (`1kHz`, `0.6V`); ET44 level in mV |
| D. own dialects | B&K 891 (`MEAS:FUNC CSQ`, `LEV:AC`), UNI-T UTR2810E+ (`FUNC C_D`, `MODE SER`, `SPEED`), B&K 889B (`CpD?`) | - | - |
| E. one-way streams | ES51919 (binary), VC4080 (ASCII, polled) | - | needs DSL stream/packet support (ES51919) |

The canonical **function keys** of the proposed contract are the baseline-A tokens (CPD, CSRS,
ZTD …): they are the de-facto standard and map 1:1 to B (`Cp-D`), to D (`CSQ`, `C_D`, `CpD`) and,
via three setters, to C.

---

## 4. Proposed LCR class contract and panel

### 4.1 Contract (proposal - not created as a file)

```yaml
schema: benchmesh.class/v1
class: LCR
description: >
  LCR / impedance meter. A measurement is a parameter pair (primary, secondary) of the DUT at the
  test frequency and level. FUNC values are keys of the definition's `features.functions`
  (canonical tokens of the E4980/Tonghui dialect: CPD, CPQ, CPG, CPRP, CSD, CSQ, CSRS, LPD, LPQ,
  LPG, LPRP, LSD, LSQ, LSRS, RX, ZTD, ZTR, GB, YTD, YTR, DCR, plus vendor extras such as RPQ, RSQ,
  ZQ, CSESR). Units of PRIMARY/SECONDARY come from features.functions[FUNC].units.
status:
  PRIMARY:   {type: si, required: true, description: "Primary reading in the base unit (F, H, Ω, S); null while settling, without data or on overload"}
  SECONDARY: {type: si, required: true, description: "Secondary reading (D and Q unitless, θ in ° or rad, Rs/Rp/ESR/X Ω, G/B S); null for DCR"}
  FUNC:      {type: string, required: true, description: "Active parameter pair (a features.functions key)"}
  FREQ:      {type: si, required: true, description: "Test frequency (Hz); null for DCR"}
  LEVEL:     {type: si, description: "Test signal level (V rms)"}
  CIRCUIT:   {type: enum, values: [SER, PAR], description: "Equivalent circuit; null where it does not apply (Z-θ, R-X, DCR)"}
  SPEED:     {type: string, description: "Measurement speed (a features.speeds value)"}
  RANGE:     {type: string, description: "AUTO, HOLD or a features.ranges value"}
  OVERLOAD:  {type: bool, description: "Out of range / bridge unbalanced (OL, FETC status ≠ 0, 9.9E37)"}
  BIN:       {type: string, description: "Comparator result (BIN1..BIN9, AUX, OUT, PASS, FAIL); null when the comparator is off"}
  BIAS:      {type: si, description: "DC bias (V)"}
  BIAS_ON:   {type: enum, values: [ON, OFF], description: "DC bias applied to the DUT"}
  REMOTE:    {type: enum, values: [ON, OFF], description: "Front panel locked by remote control"}
operations:
  function:      {get: true, set: true, value: string, required: [set], description: "Select the parameter pair (features.functions key); A/B instruments bind it to several setters"}
  frequency:     {get: true, set: true, value: si, required: [set], description: "Test frequency (Hz): a features.frequency.list value or within min..max"}
  level:         {get: true, set: true, value: si, description: "Test level (V rms)"}
  speed:         {get: true, set: true, value: string, description: "features.speeds value"}
  range:         {set: true, value: string, description: "AUTO, HOLD or a features.ranges value"}
  circuit:       {set: true, value: enum, values: [SER, PAR], description: "Only where the circuit is separate from the function (A/B instruments)"}
  measurement:   {get: true, value: si, description: "Fresh primary reading"}
  secondary:     {get: true, value: si, description: "Fresh secondary reading"}
  trigger_source:{get: true, set: true, value: string, description: "features.trigger_sources value (INT, BUS/MAN, EXT, HOLD)"}
  trigger:       {set: true, value: enum, values: [ONCE], description: "One measurement (BUS/MAN source)"}
  correction:    {set: true, value: enum, values: [OPEN, SHORT], description: "Run open or short correction over the instrument's correction points (seconds to minutes; fixture must be open/shorted first)"}
  open_correction:  {set: true, value: enum, values: [ON, OFF], description: "Apply stored open correction"}
  short_correction: {set: true, value: enum, values: [ON, OFF], description: "Apply stored short correction"}
  bias:          {get: true, set: true, value: si, description: "DC bias level (V)"}
  bias_output:   {set: true, value: enum, values: [ON, OFF], description: "Apply DC bias (definition marks it safety: energize)"}
  remote:        {set: true, value: enum, values: [ON, OFF]}
```

Open point for read-only meters (ES51919): `function`/`frequency` setters are `required` above;
either make the contract allow a read-only binding (e.g. `features.read_only: true` exempts
required setters) or drop `required` and let the panel hide missing controls. Recommended:
required **get-side** status only (PRIMARY, SECONDARY, FUNC, FREQ), setters optional - the UI
already builds controls from `/methods`.

### 4.2 Features (what the panel is driven by)

```yaml
features:
  functions:                       # allowed pairs, in front-panel order
    CPD:  {primary: Cp, secondary: D,   circuit: PAR, units: [F, ""],  ui_name: "Cp-D"}
    CSD:  {primary: Cs, secondary: D,   circuit: SER, units: [F, ""],  ui_name: "Cs-D"}
    CSRS: {primary: Cs, secondary: Rs,  circuit: SER, units: [F, "Ω"], ui_name: "Cs-Rs"}
    LSQ:  {primary: Ls, secondary: Q,   circuit: SER, units: [H, ""],  ui_name: "Ls-Q"}
    RX:   {primary: R,  secondary: X,                 units: ["Ω", "Ω"], ui_name: "R-X"}
    ZTD:  {primary: Z,  secondary: θ,                 units: ["Ω", "°"], ui_name: "Z-θ"}
    DCR:  {primary: DCR,                              units: ["Ω"],      ui_name: "DCR"}
  frequency: {list: [100, 120, 1000, 10000, 100000]}   # or {min: 20, max: 300000} (+ optional list of presets)
  level:     {list: [0.3, 0.6, 1.0]}                   # or {min: 0.005, max: 2.0}; unit V
  speeds:    [{value: FAST, display: "fast"}, {value: MED, display: "medium"}, {value: SLOW, display: "slow"}]
  ranges:    [{value: AUTO, display: AUTO}, {value: "100", display: "100 Ω"}, {value: "1000", display: "1 kΩ"}]
  trigger_sources: [INT, BUS, EXT]
  bias:      {min: 0, max: 1.5, unit: V}               # absent = no bias
  correction: [OPEN, SHORT]                             # ET44: [AUTO] (hardware decides)
  comparator: true
  counts: 99999                                         # display resolution
```

### 4.3 Panel (modelled on the front panels)

Every instrument above - handheld or benchtop - shows the same thing: a **large primary readout**
with its parameter letter and unit, a **smaller secondary readout** below, and a strip of setup
annunciators (frequency, level, SER/PAR, AUTO/HOLD, speed). Benchtops add a setup field row
(FUNC · FREQ · LEVEL · RANGE · SPEED · BIAS) and a comparator BIN result. Proposed `LcrPanel`
(kit components, one panel for all LCR meters):

- **Display (kit-glass)**
  - Primary `Readout` - tag = primary glyph (Cs, Lp, R, Z, DCR), value with SI prefix from
    `features.counts`, unit from `functions[FUNC].units[0]`; `Recordable field="PRIMARY"`.
  - Secondary `Readout` (smaller) - tag D/Q/θ/Rs/ESR/X/G/B; D and Q shown without unit, θ in °;
    `Recordable field="SECONDARY"`. Hidden for DCR.
  - Annunciators: `SER`/`PAR` (from CIRCUIT), `AUTO`/`HOLD` (RANGE), frequency (`1 kHz`), level
    (`0.6 V`), speed (`SLOW`), `BIAS` (BIAS_ON, alert colour), `OL` (alert), BIN result,
    client-side `HOLD`, `REL Δ`, `RMT`.
  - Statistics row MIN/MAX/AVG/N (reuse `useStats` from `DmmPanel`) and a sparkline of PRIMARY.
  - Optional "component view": derived values the UI can compute from the pair (e.g. ESR = D/(2πfC)
    for Cs-D, Q = 1/D) - only display, never sent.
- **Controls** (each waits for confirmation through `kit/confirm.ts` against the reported status)
  - **Function**: two chip rows built from `features.functions` - primary L / C / R / Z / DCR,
    then secondary filtered to the pairs allowed for that primary - plus a `SER | PAR` toggle when
    both circuits exist for the pair; the chosen combination resolves to one `function` key
    (`api.set('function', 'CSD')`). This mirrors the handheld keys (L/C/R, D/Q/θ/ESR, SER/PAR) and
    the benchtop FUNC soft-key list at once.
  - **Frequency**: chips when `frequency.list` exists (handhelds, ET44, UTR2810E+); otherwise the
    kit digit-wise setpoint editor with min/max from `/methods` plus 1-2-5 preset chips (benchtops).
  - **Level**: chips (list) or setpoint editor (range).
  - **Range**: `Selector` AUTO + `features.ranges` (HOLD-only meters show AUTO/HOLD).
  - **Speed**: `Selector` from `features.speeds`.
  - **Correction**: OPEN and SHORT buttons with an instruction dialog ("open the clips", "short the
    clips") - long-running, so the button pulses until the next normal reading; ET44 shows one
    "Correct" button (hardware decides).
  - **Trigger**: source selector + a "Single" button when the source is BUS/MAN; HOLD (client-side
    freeze) and REL Δ micro switches as on the DMM panel.
  - **Bias**: setpoint editor + arm-to-enable switch (`bias_output` energizes: two presses).
  - **Comparator** (later): nominal + tolerance → BIN annunciator; not needed for a first version.

### 4.4 Binding sketch for an A/B instrument (ET44) - shows the contract fits

```yaml
commands:
  imp_a_set: {send: "FUNC:IMP:A {f}", params: {f: {type: enum, values: {CPD: C, CSD: C, CSQ: C, CPQ: C, CSESR: C, LSQ: L, LPQ: L, LSD: L, RX: R, ZTD: Z, DCR: DCR}}},
              parse: [trim, {map: {"exec success": OK}}], safety: configure, source: PM p.6}
  imp_b_set: {send: "FUNC:IMP:B {f}", params: {f: {type: enum, values: {CPD: D, CSD: D, CSQ: Q, CPQ: Q, CSESR: ESR, LSQ: Q, LPQ: Q, LSD: D, RX: X, ZTD: THR}}},
              parse: [trim, {map: {"exec success": OK}}], safety: configure, source: PM p.6}
  equ_set:   {send: "FUNC:IMP:EQU {f}", params: {f: {type: enum, values: {CPD: PAL, CPQ: PAL, LPQ: PAL, CSD: SER, CSQ: SER, CSESR: SER, LSQ: SER, LSD: SER}}},
              parse: [trim, {map: {"exec success": OK}}], safety: configure, source: PM p.6}
  fetch:     {send: "FETC?", parse: [trim, {fields: {sep: ",", names: [a, b]}}], source: PM p.8}
classes:
  LCR:
    operations:
      function: {set: [{cmd: imp_a_set, args: {f: "{value}"}}, {cmd: imp_b_set, args: {f: "{value}"}}, {cmd: equ_set, args: {f: "{value}"}}]}
    status:
      PRIMARY:   {expr: 'num(fetch.a)', as: si}
      SECONDARY: {expr: 'num(fetch.b)', as: si}
      LEVEL:     {expr: 'num(volt_q) / 1000.0', as: si}     # ET44 reports mV
```

Gaps found while sketching (worth a DSL decision before writing the definitions):
1. A step list cannot skip a step: RX/ZTD/DCR have no circuit and DCR no secondary, so `equ_set` /
   `imp_b_set` have no mapping for them. Needs either a per-step condition (`when: CEL`) or an enum
   value meaning "skip this step".
2. Reading FUNC back from three queries works with a CEL map literal over `a + b + equ`.
3. `features.functions` units drive both the UI and SI display - same pattern as DMM `modes`.
</content>
</invoke>
