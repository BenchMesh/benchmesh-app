# ELL coverage: DC electronic loads

Research date 2026-10-18. Class contract: `devices/classes/ELL.yaml`. Reference definition:
`devices/owon_oel.yaml` (OWON OEL15/30, bench-tested). All new models start as
`support: awaiting`.

**Citation keys.** Pages are **PDF pages** of the local files, not printed page numbers.
Local docs live in `/home/marek/project/BenchMeshDOC/<folder>/doc/`, and each folder has a
`SOURCES.md` with the URL. Open-source protocol sources (sigrok, GitHub) are listed per family.

**Research gap.** The shared web-search budget ran out after the first group of families.
East Tester ET54xx, UNI-T UTL8200, GW Instek PEL, Keysight EL30000 and Array 3721A have
**no downloaded manual**. Their entries (section 2.10) give only what is known with low
confidence, plus the next step to take.

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Price band (new) | Protocol | Manual | Confidence | DSL today |
|---|---|---|---|---|---|---|
| 1 | **Korad KEL102/KEL103** (= Tenma 72-13200/72-13210, RND 320-KEL102/103) | €250-400 | ASCII SCPI-like with **unit suffixes**, serial/USB-CDC; LAN = **UDP** 18190 | Vendor 8-page table plus a corrected community spec | medium-high | ✅ serial (LAN needs UDP) |
| 2 | **Rigol DL3021/DL3021A/DL3031/DL3031A** | €500-900 | SCPI over USBTMC / RS232 / LAN / GPIB | Full programming guide (96 pp) | high | ✅ USBTMC + serial |
| 3 | **Siglent SDL1020X/SDL1030X (-E)** | €450-800 | SCPI over USBTMC / LAN | Full programming guide (87 pp) + sigrok driver | high | ✅ USBTMC (LAN needs TCP) |
| 4 | **Atorch DL24 / DL24P / DL24M** (and Atorch meters) | €25-60 | Binary push stream `FF 55…`, BLE/serial; **no setpoint commands** (only key presses) | Community reverse-engineering | medium | ❌ binary + push |
| 5 | **OWON OEL15xx/30xx** (+ OEL85xx) | €200-350 | SCPI over USB-CDC/RS232 (or Modbus RTU) | Full, already defined | tested | ✅ `owon_oel.yaml` |
| 6 | **East Tester ET5410/ET5411/ET5420** (and rebrands) | €150-300 | SCPI-like ASCII over USB-CDC | not retrieved | low | probably ✅ |
| 7 | **ITECH IT8511+/IT8512+/IT8513+ … / B&K 8500 series** (classic ITECH OEM) | used €150-400 | **26-byte binary frame**, TTL serial (USB via a PL2303 cable); IT8500+ also SCPI | Full (BK manual + ITECH frame guide + ITECH SCPI guide) + sigrok | high | ❌ binary frame; ✅ SCPI on IT8500+ |
| 8 | **B&K 8600 series** (= ITECH IT8800 OEM) | €900-3000 | SCPI over USBTMC / RS232 / GPIB | Full (83 pp) | high | ✅ |
| 9 | **Array 3710A/3711A** (binary) / 3720A/3721A (SCPI?) | used / €300-600 | 3710A: ITECH-like 26-byte binary (reported); 372xA: SCPI (reported) | not retrieved | low | ❌ / ? |
| 10 | **Maynuo M9711/M9712/M97xx** | €400-900 | **Modbus RTU**, float32 BE registers | sigrok register map | medium | ❌ Modbus |
| 11 | **UNI-T UTL8211+/UTL8212+** (UTL8200+) | €250-450 | SCPI over USB-CDC / RS232 (reported) | not retrieved | low | probably ✅ |
| 12 | **ZKETECH EBD-USB / EBD-A20H**, **Arachnid Re:load Pro** | €20-80 | EBD: binary `FA…F8` frames, XOR checksum, base-240 values; Re:load: plain ASCII | sigrok drivers | medium | EBD ❌ / Re:load ✅ |
| 13 | **GW Instek PEL-3000/PEL-2000**, **Keysight EL30000** | €2000+ | SCPI (USB-CDC/USBTMC/LAN/GPIB) | not retrieved | low | ✅ expected |
| - | West Mountain Radio CBA | - | Closed USB HID protocol, Windows software only | none public | - | out of scope |

---

## 2. Families

### 2.1 Korad KEL102 / KEL103 (+ Tenma 72-13200/72-13210, RND 320-KEL102/KEL103)

**Docs.** `korad_kel10x/doc/KEL10x_Communication_Commands_with_Computer.pdf` (CC, 8 pp) and
`KEL10x_IP_LAN_Communication_Protocol.pdf` (LAN, 6 pp). The best source is
the community spec **kelctl `KEL103-protocol.md`** (KP; https://github.com/vorbeiei/kelctl).
It corrects the vendor table using trial-and-error on hardware. Also used: **py-korad** (PK;
https://github.com/Mango-kid/py-korad). The py-korad README confirms that the Tenma 72-132xx
is the same instrument.

**Models.** KEL102: 120 V / 30 A / 150 W. KEL103: 120 V / 30 A / 300 W. Tenma 72-13200 is
the 150 W unit and 72-13210 the 300 W unit (Farnell datasheet). CR maximum is reported as
`7500OHM` (CC p.2, `:RES:UPP?` example). Read the per-unit limits at connect time with
`:VOLT:UPP?`, `:CURR:UPP?`, `:POW:UPP?`, `:RES:UPP?` (and `…:LOW?`).

**Interfaces.**
- USB: CDC-ACM (appears as `/dev/ttyACM*` per KP). VID:PID is **unknown**. Korad PSUs use
  0416:5011, which is a plausible guess but must be verified.
- RS232 (DB9).
- LAN: **UDP**, device port 18190 by default. The local port must equal the device port.
  Factory IP is 192.168.1.198 (LAN p.2-5).
- Baud: 9600/19200/38400/57600/115200, factory **115200** 8N1 (LAN p.2 `DEVINFO`/FACTRESET;
  KP default).

**Protocol.** ASCII SCPI-like. TX terminator `\n` (KP, PK). Replies end in `\n`.
**Every value carries a unit suffix**, both sent and received: `:CURR 2A` and the reply
`10.000A`; `20W`; `10OHM`; `5V` (KP note at the end; CC p.2-3).

**Identify.** `*IDN?` → `RND 320-KEL103 V2.60 SN:01234567` (KP). Korad-branded units
presumably return `KORAD KEL103 V…`. Model regex: `KEL10[23]` anywhere in the reply
(the Tenma IDN form is unknown).

| Function | Command | Reply / params | Source |
|---|---|---|---|
| Identify | `*IDN?` | see above | CC p.1, KP |
| Input | `:INP ON\|OFF` (`1/0` accepted, PK) / `:INP?` | `ON`/`OFF` | CC p.2, KP |
| Mode select | `:FUNC CC\|CV\|CR\|CW\|SHORt` | **The vendor table says `VOLT\|CURR\|RES\|POW\|SHORT` (CC p.3), but KP and PK use `CC/CV/CR/CW`** (tested). Use CC/CV/CR/CW | KP, PK |
| Mode query | `:FUNC?` | `CC`, `CV`, `CR`, `CW`, `SHORt` (sic), `BATTERY`, `OCP`, `OPP`, `LIST`, `CONTINUOUS CC/CV/CR/CW`, `PULSE`, `TOGGLE` | KP (kelenums.py) |
| CC setpoint | `:CURR <x>A` / `:CURR?` | `10.000A` | CC p.2, KP |
| CV setpoint | `:VOLT <x>V` / `:VOLT?` | `10.000V` | CC p.2, KP |
| CR setpoint | `:RES <x>OHM` / `:RES?` | `10.000OHM` | CC p.2-3, KP |
| CP setpoint | `:POW <x>W` / `:POW?` | `100.00W` | CC p.3, KP |
| Limits | `:VOLT:UPP <v>V` / `:VOLT:UPP?`, `:VOLT:LOW?`; same for CURR/POW/RES | `150V` … | CC p.2-3, KP |
| Readings | `:MEAS:VOLT?`, `:MEAS:CURR?`, `:MEAS:POW?` | `7.4486V`, `3.2415A`, `24.145W` | CC p.3, KP |
| Status | `:STAT?` | `0,4,0,0,0,0` = beep, baud index, lock, ext-trigger, **remote compensation**, ? | CC p.1-2, KP |
| Remote sense | `:SYST:COMP ON\|OFF` / `?` (undocumented by Korad) | `ON`/`OFF` | KP, element14 thread |
| Panel lock | `:SYST:LOCK ON\|OFF` / `?` | `ON`/`OFF` | KP |
| Battery | `:BATT g,<range>A,<I>A,<Vcut>V,<Ah>AH,<t>M`; `:RCL:BATT g`; `:BATT:TIM?` (`0.9547M`), `:BATT:CAP?` (`0.1093AH`) | | CC p.6-7, KP |
| Dynamic | `:DYN 1..6,…` / `:DYN?` (must recall first) | | CC p.7, KP |
| LIST/OCP/OPP tests | `:LIST`, `:OCP`, `:OPP`, `:RCL:*` | | CC p.4-6 |
| Store/recall | `*SAV 1..100`, `*RCL 1..100` | | CC p.1 |

**Quirks.**
- **Setting a setpoint switches the mode** to the matching one (KP: "will also switch to CV
  mode when setting"). A definition must not expose that as a silent side effect. Gate each
  setter with `requires: func_q == "<mode>"` (as the OEL definition does), or document it as
  intentional.
- Values above the user limit are clamped to the limit. **The CR and CP limits are mixed up
  in firmware V2.60** (KP): a CR value above the limit is set to the *power* limit expressed
  in Ω. Do not trust POW/RES limits for validation; use fixed per-model ranges.
- kelctl waits 100 ms after every send (`send_sleep_time=0.1`). Use
  `timing.gap_after_write_ms: 100` until measured.
- There is **no remote/local command and no protection-trip status**. `:STAT?` field 3 is
  the panel lock. Bind `remote` to `:SYST:LOCK` (closest equivalent) or to a client-side
  state; see §4.
- Recalling an empty LIST/OCP slot "will get error on device potentially causing issues with
  serial connection" (KP).

**DSL fit.** Works with today's DSL: serial, `tx "\n"`, `rx` terminator `\n`. Parse with
`[trim, {regex: {pattern: "^([-+0-9.Ee]+)", group: 1}}, number]` to strip the unit. The
`FUNC?` map: `CC→CURR, CV→VOLT, CR→RES, CW→POW, SHORt→SHORT`, plus `_` passthrough for the
test modes. Short circuit is a *mode* (`:FUNC SHORt`), not a separate input switch. `short`
set ON maps to `:FUNC SHORt`; OFF has no inverse (restore the previous mode). It is better
not to bind `short` at all. LAN needs a **`udp` link**, which does not exist yet.
**Confidence: medium-high.** The command set is verified by two independent open-source
projects. Framing and timing are unmeasured.

### 2.2 Rigol DL3000 (DL3021, DL3021A, DL3031, DL3031A)

**Docs.** `rigol_dl3000/doc/DL3000_ProgrammingGuide_EN_2019.pdf` (PG, Apr 2019, SW 00.01.04).

**Models (datasheet).**

| Model | Rating | CC ranges | Other |
|---|---|---|---|
| DL3021/DL3021A | 150 V / 40 A / 200 W | 4 A / 40 A | |
| DL3031/DL3031A | 150 V / 60 A / 350 W | 6 A / 60 A | |
| all | | | CV ranges 15 V / 150 V (`:VOLT:RANG 150`, PG p.47); CR ranges 15 Ω / 15 kΩ (`:RES:RANG 15000`, PG p.49) |

The "A" models include the high-slew-rate, high-frequency, high-readback-resolution, LAN and
digital-I/O options (PG p.11, p.23 `*OPT?`). On non-A models LAN is a paid option.

**Interfaces.**
- USBTMC **1AB1:0E11** (PG p.76, p.80 `USB0::0x1AB1::0x0E11::DL3000A000001::INSTR`).
- RS232: 9600 8N1, no flow control (factory default, PG p.95). "**The RS232 protocol command
  ends with `\r\n`**" (PG p.10).
- LAN: VXI-11/LXI. Rigol's raw socket port is usually 5555 (not stated in PG; verify).
- GPIB via a USB-GPIB module.

**Identify.** `*IDN?` → `RIGOL TECHNOLOGIES,DL3031A,LS000001,00.01.00.04.05` (PG p.80,
p.22). Regex: `^RIGOL TECHNOLOGIES,DL30[23]1A?,`.

| Function | Command | Reply | PG page |
|---|---|---|---|
| Input | `:SOUR:INP:STAT ON\|OFF` / `:SOUR:INP:STAT?` | `0`/`1` | 38 |
| Mode | `:SOUR:FUNC CURR\|RES\|VOLT\|POW` / `:SOUR:FUNC?` | **`CC`, `CV`, `CR`, `CP`** | 38 |
| What drives regulation | `:SOUR:FUNC:MODE FIX\|LIST\|WAV\|BATT\|OCP\|OPP` / `?` | `FIX`, `LIST`, `WAV`, `BATT`, `OCP`, `OPP` | 38-39 |
| CC setpoint | `:SOUR:CURR:LEV:IMM <A>` / `?` (accepts `MIN\|MAX\|DEF`) | real | 39-40 |
| CC range | `:SOUR:CURR:RANG <A>` / `?` (low range is the default) | real | 40 |
| CC slew | `:SOUR:CURR:SLEW[:BOTH\|:POS\|:NEG] <A/µs>` | real | 40-41 |
| Von / V limit / I limit | `:SOUR:CURR:VON`, `:SOUR:CURR:VLIM`, `:SOUR:CURR:ILIM` (also per mode: `VOLT:VLIM/ILIM`, `RES:…`, `POW:…`) | real | 42, 48, 49-51 |
| CV setpoint | `:SOUR:VOLT:LEV:IMM <V>` / `?` | real | 47 |
| CR setpoint | `:SOUR:RES:LEV:IMM <Ω>` / `?` | real | 48-49 |
| CP setpoint | `:SOUR:POW:LEV:IMM <W>` / `?` | real | 50 |
| Readings | `:MEAS:VOLT?`, `:MEAS:CURR?`, `:MEAS:POW?`, `:MEAS:RES?` (`FETC:` variants also exist) | real | 31-33 |
| Battery results | `:MEAS:CAP?` (mAh), `:MEAS:WATT?`, `:MEAS:DISCT?`, `:MEAS:TIME?` | real | 33-34 |
| Battery setup | `:SOUR:BATT:RANG/LEV/VST/CST/TIM/VON/VEN/CEN/TEN` | | 54-58 |
| OCP/OPP *tests* | `:SOUR:OCP:*`, `:SOUR:OPP:*` (test routines, not protection limits) | | 58-66 |
| Remote sense | `:SOUR:SENS 0\|1` / `?` | `0`/`1` | 67 |
| Transient | `:SOUR:TRAN 0\|1`, `:SOUR:CURR:TRAN:MODE CONT\|PULS\|TOGG`, `ALEV/BLEV/AWID/BWID/FREQ/PER/ADUT` | | 39, 43-47 |
| Protection status | `:STAT:QUES:COND?` → integer bits: 0 VF, 1 OC, 2 RS, 3 OP, 7 RUN, 9 RRV, 10 UNR, 11 LRV, **12 OV**, **13 PS** (protection shutdown: OC/OP/OT), 14 VON | | 15, 27 |
| Error queue | `:SYST:ERR?` → `-113,"Undefined header…"` | | 69 |
| Key press | `:SYST:KEY <n>` (8 = Local, 32 = ON/OFF, **33 = SHORT**) | | 68 |

**Quirks.**
- There is no `SYST:REM`/`SYST:LOC` command and no SCPI short-circuit command (only
  `:SYST:KEY 33`). There are no OVP/OCP/OPP *level* commands: the "OCP/OPP" subsystems are
  test sequences. Protection trips are visible only through `STAT:QUES:COND?` (and the input
  dropping to OFF).
- The setpoint can exceed the active range. Select `:SOUR:CURR:RANG` first (for example
  60 A for currents above 6 A on a DL3031). It is unclear whether the range auto-switches;
  test this.
- Over RS232 use the `\r\n` terminator (PG p.10).
- `:SOUR:FUNC?` returns the short form `CC/CV/CR/CP`, not the set keyword.

**DSL fit.** Works today over **USBTMC** (`rx.mode: message`) or serial. The OV/OC/OP flags
come from `STAT:QUES:COND?` with `{bits: {ov: "12", oc: "1", op: "3", ps: "13"}}`. The
missing `remote` operation is a class-contract issue (§4). LAN needs the TCP link to be
implemented. **Confidence: high** (complete official guide; the reply format is documented).

### 2.3 Siglent SDL1000X / SDL1000X-E (SDL1020X, SDL1030X, SDL1020X-E, SDL1030X-E)

**Docs.** `siglent_sdl1000x/doc/SDL1000X_programming_guide_E02B.pdf` (PG). Also the sigrok
driver `siglent-sdl10x0` (SR).

**Models.** SDL1020X(-E): 150 V / 30 A / 200 W. SDL1030X(-E): 150 V / 30 A / 300 W. The
6th character of the model name gives the wattage (SR api.c). Ranges: current **5 A / 30 A**
(values above 5 select 30 A); voltage **36 V / 150 V** (values above 36 select 150 V)
(PG p.14). CR ranges: `LOW|MIDDLE|HIGH|UPPER` (PG p.31). The -E models have lower
resolution and the same command set.

**Interfaces.**
- USBTMC, vendor **0xF4EC** (the Siglent USB VID; the SDL product ID is **unknown**, verify).
- LAN: VXI-11, raw socket, telnet (PG p.7). Siglent's usual ports are 5025 (socket) and
  5024 (telnet); verify.
- The programming guide documents no RS232.

**Identify.** `*IDN?` → `Siglent Technologies,SDL1020X,0123456789,1.01.01.15` (PG p.15;
`\s` in the PDF stands for a space). Regex: `^Siglent Technologies,SDL10[23]0X(-E)?,`.

| Function | Command | Reply | PG page |
|---|---|---|---|
| Input | `:SOUR:INP:STAT ON\|OFF` / `?` (SR uses `:INPUT?`) | `0`/`1` | 18 |
| Short | `:SOUR:SHOR:STAT ON\|OFF` / `?` | `0`/`1` | 18 |
| Mode | `:SOUR:FUNC CURR\|VOLT\|POW\|RES\|LED` / `:SOUR:FUNC?` | **`CURRENT`, `VOLTAGE`, `POWER`, `RESISTANCE`** (uppercase long form; `LED`) | 19, SR |
| Transient mode | `:SOUR:FUNC:TRAN CURR\|…` / `?` | | 18-19 |
| CC setpoint | `:SOUR:CURR:LEV:IMM <A>` / `?` | `2.674` | 19-20 |
| CC ranges | `:SOUR:CURR:IRANG <A>`, `:SOUR:CURR:VRANG <V>` / `?` | `30`, `36` | 20 |
| CV setpoint | `:SOUR:VOLT:LEV:IMM <V>` / `?` (+ `IRANG/VRANG`) | `4.000` | 24-25 |
| CP setpoint | `:SOUR:POW:LEV:IMM <W>` / `?` | | 27-28 |
| CR setpoint | `:SOUR:RES:LEV:IMM <Ω>` / `?`, `:SOUR:RES:RRANG LOW\|MIDDLE\|HIGH\|UPPER` | | 30-31 |
| Readings | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?`, `MEAS:RES?` | `7.924678` | 17 |
| Waveform | `MEAS:WAVE? CURR\|VOLT\|POW\|RES` → 200 floats (CSV) | | 18 |
| Von / latch | `:SOUR:VOLT:LEV:ON <V>`, `:SOUR:VOLT:LATC ON\|OFF` | | 57 |
| OCP / OPP limits | `:SOUR:CURR:PROT:STAT/LEV/DEL`, `:SOUR:POW:PROT:STAT/LEV/DEL` | | 58-59 |
| Remote sense | `SYST:SENS ON\|OFF` / `?` | `0`/`1` | 59-60 |
| Battery | `:SOUR:BATT:FUNC`, `:SOUR:BATT:MODE CURR\|POW\|RES`, `:LEV`, `:VOLT`, `:CAP`, `:TIM`, `:…:STAT`, `:SOUR:BATT:DISCHA:CAP?` (mAh), `:DISCHA:TIM?` (s) | | 35-39 |
| Averaging | `SENS:AVER:COUN 6..14` | | 61 |
| LAN config | `LAN:IPAD`, `LAN:SMAS`, `LAN:GAT`, `DHCP`, `LAN:MAC?` | | 63-65 |

**Quirks.**
- No remote/local command and **no protection-trip query** is documented (no
  `STAT:QUES`). Only `*STB?`/`*ESR?` are available (PG p.15-16). OVP/OCP/OPP status keys
  stay unbound.
- PG p.58: the query row for `CURR:PROT:STAT?` is a copy-paste error (it shows
  `EXT:INPUT?`). Assume `:SOUR:CURR:PROT:STAT?` and verify.
- The mode-set keyword (`CURR`) differs from the reply (`CURRENT`): map
  `CURRENT→CURR, VOLTAGE→VOLT, POWER→POW, RESISTANCE→RES, LED→LED`.
- Setpoints are per mode. Whether setting another mode's level switches the mode is
  undocumented.

**DSL fit.** Works today over USBTMC (`message`). A `features.modes.LED` entry is optional.
**Confidence: high.**

### 2.4 Atorch DL24 / DL24P / DL24M (and Atorch UD18/AT24 meters)

**Sources.** sigrok `atorch` (SR) and the NiceLabs/syssi protocol design (AP:
https://github.com/syssi/esphome-atorch-dl24/blob/main/docs/protocol-design.md). Atorch
publishes no manual. Local note: `atorch_dl24/doc/SOURCES.md`.

**Interfaces.** BLE (service FFE0, characteristic FFE1; name `*-BLE`), Bluetooth SPP
(`*-SPP`), and on some versions a USB/TTL serial port at 9600 8N1 (SR).

**Protocol** (binary, AP):
- Every packet is `FF 55 | type | payload | checksum`.
- Type `01` is a **report**: 32-byte payload, **sent unsolicited about once per second**.
  Type `02` is a reply (4 bytes; `02 01` = OK, `02 03` = unsupported). Type `11` is a
  command (6 bytes).
- DC-meter report (device type `02`, used by the DL24): V = u24be/10, I = u24be/1000,
  P = u24be/10, Wh = u32be/100, temperature u16be at 0x18, h/m/s at 0x1A-0x1D. The sigrok
  scaling differs slightly: V u24be ×0.1, I u24be ×0.001, Ah u24be ×0.01 at 0x0A.
- Commands: `FF 55 11 <devtype> <cmd> <u32be value> <cks>`. cmd `01/02/03/05` resets Wh/Ah/
  time/all; `11/12` = **[+]/[−] key**; `31` = setup; `32` = enter; `21` = backlight; `22` =
  price.
- Checksum = (sum of bytes from type to the last payload byte) & 0xFF, **XOR 0x44**.

**Fit.** **Not an ELL in the contract sense.** The current setpoint can only be changed by
simulated key presses, there is no input on/off command on most firmware, and there is no
mode or remote. A read-only integration (VIN/IIN/PIN, energy, temperature) needs a **push-
stream framing mode** (subscribe to unsolicited frames), u24be field extraction, the XOR-0x44
checksum, and BLE transport. **Recommend: defer, or treat as a read-only "meter" class later.
Confidence: medium.**

### 2.5 OWON OEL15xx/30xx (+ OEL85xx)

Already defined and bench-tested (`devices/owon_oel.yaml`,
`ai_context/migration/device-notes/owon_oel.md`). Notes for this report:
- Its SCPI is close to the ITECH/BK 8600 tree (`INP`, `FUNC`, `CURR/VOLT/RES/POW`,
  `MEAS:VOLT?`). `FUNC?` replies `current/voltage/...`.
- The panel menu also offers **Modbus RTU**, which would need the Modbus feature (§5).
- A good template for the `scpi_load` base.

### 2.6 ITECH IT8500 / IT8500+ binary frame - B&K Precision 8500 series

**Docs.**
- `bk_8500/doc/85xx_manual.pdf` (BK; remote chapter p.46-69).
- `itech_it8500/doc/ITECH_IT8500plus_Programming-Guide_Frame-Format.pdf` (IF; Ver1.1 2008).
- sigrok `itech-it8500` (SR).
- Python: https://github.com/cdwilson/BK-8500-Electronic-Load ,
  https://github.com/justengel/pybk8500.

**Same protocol (clones).** ITECH IT8511/IT8512/IT8513/IT8514/IT8516 (non-plus) and
IT8511+…IT8516C+ in frame mode, and **B&K 8500/8502/8510/8512/8514/8518/8520/8522/8524/8526**
(ITECH OEM; the adapter is ITECH's IT-E132B). Array 3710A reportedly uses a similar frame
(see 2.10).

**Models.**
- BK (p.15-16): 8500 120 V/30 A/300 W · 8502 500 V/15 A/300 W · 8510 120 V/120 A/600 W ·
  8512 500 V/30 A/600 W · 8514 120 V/240 A/1200 W · 8518 60 V/240 A/1200 W ·
  8520 120 V/240 A/2400 W · 8522 500 V/120 A/2400 W · 8524 60 V/240 A/5000 W ·
  8526 500 V/120 A/5000 W.
- ITECH IT8511+ ≈120 V/30 A/150 W, IT8512+ ≈120 V/30 A/300 W, and so on. Read the real limits
  at runtime with ITECH cmd `01H` (rated I, V max/min, P, R max/min; IF p.20) or the BK
  "max" queries `0x23/0x25/0x27`.

**Interfaces.**
- DB9 at **TTL levels**. Do not connect it to RS-232 (BK p.46, IF p.5). PC adapters:
  IT-E121 (RS232), IT-E122 / IT-E132B (USB, **Prolific PL2303**, vid:pid usually
  067B:2303), IT-E123 (RS485). IT8513B+/14B+/14C+/16C+ have RS232 + USB built in (IF p.5,
  p.9).
- Serial: 4800/9600/19200/38400, 8N1, default 9600 (IF p.7). The BK manual shows 38400 in
  its example (BK p.48). **"DTR and RTS signal lines must be enabled/asserted"** (BK p.46).
- Address 0-31 (ITECH) / 0x00-0xFE (BK). 0xFF is broadcast (IF p.9).

**Frame** (BK p.47, IF p.9-12, SR protocol.h). All packets are **26 bytes** in both
directions:

```
byte 0     0xAA                 preamble
byte 1     address              (0 default)
byte 2     command
byte 3..24 data (22 bytes)      unused = 0x00; integers little-endian
byte 25    checksum = (sum of bytes 0..24) mod 256
```

Every command gets a 26-byte reply. A command without data returns a **status packet**:
`AA addr 12 <status> …`, where status `0x80` = OK, `0x90` = bad checksum, `0xA0` = bad
parameter, `0xB0` = unknown command, `0xC0` = invalid command (BK p.47). Example (BK p.49):
remote ON = `AA 00 20 01 00…00 CB`, reply `AA 00 12 80 00…00 3C`.

| Function | Cmd | Data (offset from 0) | Scale | Source |
|---|---|---|---|---|
| Remote / local | `0x20` | [3] 1 = remote, 0 = panel | - | BK p.54, IF p.12 |
| Input on/off | `0x21` | [3] 1/0 | - | BK p.54 |
| Max V / I / P set·get | `0x22/23`, `0x24/25`, `0x26/27` | [3..6] u32le | 1 mV / 0.1 mA / 1 mW | BK p.54-55 |
| Mode set / get | `0x28` / `0x29` | [3] 0 = CC, 1 = CV, 2 = CW, 3 = CR | - | BK p.56 |
| CC set / get | `0x2A` / `0x2B` | [3..6] u32le | **0.1 mA** | BK p.56-57 |
| CV set / get | `0x2C` / `0x2D` | u32le | 1 mV | BK p.57 |
| CW set / get | `0x2E` / `0x2F` | u32le | 1 mW | BK p.57 |
| CR set / get | `0x30` / `0x31` | u32le | 1 mΩ | BK p.58 |
| Function set / get | `0x5D` / `0x5E` | [3] 0 = FIXED, **1 = SHORT**, 2 = TRANSIENT, 3 = LIST, 4 = BATTERY | - | BK p.67-68 |
| **Read state** | `0x5F` | V u32le [3..6] (1 mV), I u32le [7..10] (0.1 mA), P u32le [11..14] (1 mW), operation reg u8 [15], demand reg u16le [16..17] | | BK p.68, IF p.20 |
| Remote sense set / get | `0x56` / `0x57` | [3] 1/0 | - | BK p.66 |
| Local key enable | `0x55` | [3] 1/0 | - | BK p.66 |
| Product info | `0x6A` (BK) | model ASCII [3..7], FW [8] lo / [9] hi, serial ASCII [10..19] | - | BK p.69 |
| Load limits (ITECH +) | `0x01` | I max, V max, V min, P max, R max u32le; R min u16le | | IF p.20 |
| Soft OCP / OPP (ITECH +) | `0x80-0x89` | level / delay / enable | | IF p.11 |
| Clear protection (ITECH +) | `0x90` | - | | IF p.11 |

Register bits:
- Operation register (`0x5F` [15]): 0 CAL, 1 WTG, **2 REM**, **3 OUT (input on)**, 4 LOCAL
  key, 5 SENSE, 6 load-on timer (BK p.68).
- Demand register (u16): 0 RV, **1 OV, 2 OC, 3 OP, 4 OT**, 5 sense disconnected, **6 CC,
  7 CV, 8 CW, 9 CR** (the active regulation mode), 10-12 autotest (IF p.20-21).

**Binding sketch.** One `0x5F` poll gives VIN/IIN/PIN, INPUT (op bit 3), REMOTE (op bit 2),
OVP/OCP/OPP (demand bits 1/2/3) and MODE (`0x29` or demand bits 6-9). Setters `0x20`,
`0x21`, `0x28`, `0x2A`/`2C`/`2E`/`30`, `0x5D` (short = function 1, back to 0 = FIXED),
`0x56`. Identify: `0x6A` (BK) or `0x01` + model from `6A` where present. sigrok reads the
model from the `6A` reply data.

**DSL fit: ❌.** It needs binary framing; §5 gives the exact feature list.
**Confidence: high** (three consistent sources plus a working sigrok driver).

### 2.7 ITECH IT8500+ in SCPI mode

**Docs.** `itech_it8500/doc/ITECH_IT8500plus_Programming-Guide_SCPI.pdf` (IS, Rev V1.3).
Models IT8511+, IT8511A+, IT8512+, IT8512A+, IT8512B+, IT8512C+, IT8512H+, IT8513A+,
IT8513B+, IT8513C+, IT8514C+, IT8514B+, IT8516C+ (IS p.1). The same physical link applies
(TTL DB9 + IT-E12x, or built-in RS232/USB-PL2303 on B+/C+ models; IS p.20-21).
Retailers say "many older units can be upgraded to SCPI". Whether frame or SCPI is active
is probably a menu setting; this is unverified.

- Terminator `\n` (IS p.13 "Message Terminator": newline 0x0A).
- `*IDN?` → `ITECH Ltd, IT85XX, XXXXXXXXXXXXXXXXXX, 1.21-1.28` (IS p.71). Note the spaces
  after the commas.

| Function | Command | Reply | IS page |
|---|---|---|---|
| Remote / local | `SYST:REM` / `SYST:LOC` / `SYST:RWL` (lock incl. LOCAL key). In remote mode every key except LOCAL and Shift is locked | - | 25-26 |
| Input | `INP 0\|1\|OFF\|ON` / `INP?` | `0`/`1` | 31 |
| Short | `INP:SHOR 0\|1` / `INP:SHOR?` | `0`/`1` | 31-32 |
| Mode | `FUNC` (= `MODE`) `CURR\|VOLT\|POW\|RES\|DYN\|LED\|IMP` / `FUNC?` | `<CRD>` (expected `CURR`/`VOLT`/`POW`/`RES`/`DYN`/…; exact form unverified) | 41 |
| Setpoints | `CURR <A>`, `VOLT <V>`, `POW <W>`, `RES <Ω>` and `?` (long form `[SOUR:]CURR[:LEV][:IMM][:AMPL]`) | NR2 | 42-45 |
| Ranges | `CURR:RANG`, `VOLT:RANG`, `VOLT:RANG:AUTO` | | 32-34 |
| Protection limits | `CURR:PROT[:LEV]`, `POW:PROT[:LEV]` (`…:DEL`, `CURR:PROT:STAT`) | | 37-39 |
| Von / Voff | `VOLT:ON`, `VOLT:OFF` | | 39-40 |
| Readings | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?`, `MEAS:RES?` (+ `MAX/MIN/PTP`) | NR2 | 53-56 |
| Remote sense | `SYST:SENS 0\|1` / `?` | `0`/`1` | 24-25 |
| Protection status | `STAT:QUES:COND?` bits: 0 VF, 1 OC, 3 OP, **4 OT**, 8 SV, 11 UNR, **13 OV** (this layout **differs** from Rigol/BK 8600) | | 22, 27 |
| Error | `SYST:ERR?` | | 24 |

**DSL fit: ✅** (serial, `\n`; `dtr/rts: high` when using the PL2303 cable).
**Confidence: medium.** The guide is good, but how SCPI mode is selected on TTL models and
the exact `FUNC?` reply are unverified.

### 2.8 B&K Precision 8600 series (ITECH IT8800 OEM)

**Docs.** `bk_8600/doc/8600_Series_Programming_Manual.pdf` (BP).

**Models.** 8600, 8601, 8602, 8610, 8612, 8614, 8616 (BP title). Ratings from the datasheet
(medium confidence): 8600 120 V/30 A/150 W · 8601 120 V/60 A/250 W · 8602 500 V/15 A/200 W ·
8610 120 V/120 A/750 W · 8612 500 V/30 A/750 W · 8614 120 V/240 A/1500 W ·
8616 500 V/60 A/1200 W.

**Interfaces.**
- USBTMC / USB488 (BP p.8). VID 0x2EC7 (B&K/ITECH). PID **unknown**; 0x8800 is the
  commonly reported value, so verify.
- RS232: 4800-115200, 8N1, null-modem cable, RTS hold-off / XON-XOFF flow control (BP
  p.6-7).
- GPIB.

**Protocol.** SCPI. Commands end in NL; responses end in LF (BP p.21, p.24).
**Identify.** `*IDN?` → `B&K PRECISION, 8600, 600150010677510002, 1.32-1.37` (BP p.29).
Regex: `^B&K PRECISION, ?86\d\d,`.

| Function | Command | Reply | BP page |
|---|---|---|---|
| Remote / local | `SYST:REM`, `SYST:LOC`, `SYST:RWL` (RS-232 operation) | - | 37 |
| Input / short | `INP[:STAT] 0\|1` / `INP?`; `INP:SHOR[:STAT] 0\|1` / `INP:SHOR:STAT?` | `0`/`1` | 44 |
| Remote sense | `REM:SENS[:STAT] 0\|1` / `?` | `<CRD>` | 44 |
| Mode | `FUNC CURR\|RES\|VOLT\|POW` / `FUNC?` | `<CRD>` | 44-45 |
| Fixed / list | `FUNC:MODE FIX\|LIST` | | 45 |
| Clear protection | `PROT:CLE` (restores the input to its state before the fault) | - | 45-46 |
| OCP / OPP soft limits | `CURR:PROT:STAT/LEV/DEL`, `POW:PROT:LEV/DEL` | | 48-49, 61 |
| Setpoints | `CURR`, `VOLT`, `RES`, `POW` (+ transient A/B levels and widths) | NR2 | 47-61 |
| Readings | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` (`FETC:` = no new acquisition) | NR2 | 66 |
| Status | `STAT:QUES:COND?` bits: 0 VF, 1 OC, 2 RS, 3 OP, 4 OT, 7 RUN, 9 RRV, 10 UNR, 11 LRV, **12 OV**, **13 PS**, 14 VON (same layout as Rigol) | | 9-10, 39 |

**DSL fit: ✅** over USBTMC or serial. **Confidence: high** for commands, low for the PID.

### 2.9 Maynuo M97xx (M9711, M9712, M9712B, M9712C, M9710 …)

**Sources.** sigrok `maynuo-m97` (SR; local note `maynuo_m97/doc/SOURCES.md`).
**Protocol.** **Modbus RTU** over RS232/RS485/USB-serial. The serial settings and slave
address are configurable; 9600 8N1 and address 1 are typical, but verify.

- Coils (FC 01 read / 05 write): PC1 `0x0500` (**remote control**), TRIG `0x0502`, REMOTE
  `0x0503` (sense), ISTATE `0x0510` (input state), and protection flags IOVER `0x0520`,
  UOVER `0x0521`, POVER `0x0522`, HEAT `0x0523`, REVERSE `0x0524`.
- Registers (FC 03 read / 16 write), **float32 big-endian over 2 registers**: IFIX `0x0A01`
  (CC A), UFIX `0x0A03` (CV V), PFIX `0x0A05` (CP W), RFIX `0x0A07` (CR Ω), IMAX/UMAX/PMAX
  `0x0A34/36/38`, measured **U `0x0B00`, I `0x0B02`**.
- CMD `0x0A00` (u16) selects the mode or action: 1 CC, 2 CV, 3 CW, 4 CR, 26 short,
  38 battery, **42 input ON, 43 input OFF**.
- SETMODE `0x0B04`; MODEL `0x0B06` (numeric id, for example 28 = M9710, 53 = M9711,
  54 = M9712); EDITION `0x0B07`.

The sigrok driver enables only the M9710 and lists the rest commented out (SR api.c).
**DSL fit: ❌** (Modbus RTU with CRC16; float32 BE decode). **Confidence: medium.**

### 2.10 Families without a retrieved manual (low confidence; next step = fetch the manual)

| Family | What is known | Next step |
|---|---|---|
| **East Tester ET5410/ET5411/ET5420(A+)** | Cheap 150 V/40 A/400 W class (ET5420 has 2 channels). USB-CDC. A SCPI-like ASCII command set with a channel prefix (`CH1:…`) is reported; the identify string is unknown | Get the "ET54 series programming manual" from the vendor (et-instrument / Hangzhou Zhongchuang); likely ✅ DSL |
| **UNI-T UTL8211+/UTL8212+** | 150 V/30 A/150-300 W, USB (CDC) + RS232, SCPI per datasheet | Programming manual on uni-trend.com; likely ✅ |
| **Array 3710A/3711A** | Reported ITECH-like **26-byte 0xAA frame with sum checksum**, but with a different, compact command set (one "set all" / "read all" pair). Sold also as Tekpower/Velleman rebrands. **3720A/3721A**: SCPI over USB/RS232 per retailer pages | Array 3710A manual (protocol appendix); same binary DSL feature as §2.6 |
| **GW Instek PEL-3000(E)/PEL-2000A** | SCPI over USB-CDC (VID 0x2184), RS232, GPIB, LAN. PEL-2000 is a mainframe with channels | GW programming manual; ✅ expected (multi-channel for PEL-2000) |
| **Keysight EL30000 (EL33133A, EL34143A, EL34243A)** | SCPI over USBTMC (VID 0x2A8D) / LAN. N3300-style tree (`FUNC`, `INP`, `MEAS:VOLT?`); EL34243A has 2 channels with `(@1)` channel lists | Keysight programming guide; ✅ (channel list via `{ch}`) |
| **ZKETECH EBD-USB / EBD-A20H** (sigrok `zketech-ebd-usb`) | Binary frames `FA <cmd> … <xor> F8`, values encoded as `hi*240+lo`, **9600 8E1**, 10 ms between bytes. Commands: `01` start (current, cut-off V), `02` toggle, `05` connect (starts streaming), `06` stop, `07` set config | ❌ binary; niche |
| **Arachnid Labs Re:load Pro** (sigrok) | ASCII at 115200 8N1: `version`, `set <mA>` / `set`, `on`/`off`, `read` → `read <mA> <mV>`, `uvlo <mV>`, `monitor <ms>` (push), errors `err …`, events `overtemp`/`undervolt`. CC only | ✅ DSL (CC-only ELL) |
| West Mountain Radio CBA | USB HID with a closed protocol | out of scope |

---

## 3. Shared SCPI baseline (`devices/bases/scpi_load.yaml`) and clone map

**Clone and OEM map.**
- ITECH IT8500 (frame) = **B&K 8500** (same 26-byte frame, ITECH adapters).
- ITECH IT8800 = **B&K 8600** (SCPI).
- Korad KEL10x = **Tenma 72-13200/13210** = **RND 320-KEL102/103**.
- Rigol DL3000 and B&K 8600 use the **same QUES bit layout** (Keysight N3300 heritage).
  ITECH IT8500+ SCPI uses a different one (OT bit 4, OV bit 13, no PS bit).
- OWON OEL follows the ITECH SCPI tree closely.

**Common subset** (Rigol DL3000, Siglent SDL1000X, ITECH IT8500+ SCPI, B&K 8600, OWON OEL;
Keysight/GW expected):

| Op | Wire (base) | Per-family override |
|---|---|---|
| identify | `*IDN?` | - |
| input set/get | `INP {on}` (`ON\|OFF`) / `INP?` → `0\|1` | OEL replies `ON/OFF` |
| mode set | `FUNC {mode}` with `CURR\|VOLT\|RES\|POW` | Korad: `CC\|CV\|CR\|CW` (not SCPI; does not extend the base) |
| mode get | `FUNC?` → parse `[trim, lower, {regex: "^(cc\|cv\|cr\|cp\|cw\|curr\|volt\|res\|pow)"}, {map: {cc: CURR, curr: CURR, cv: VOLT, volt: VOLT, cr: RES, res: RES, cp: POW, cw: POW, pow: POW}}]` | One map covers Rigol `CC`, Siglent `CURRENT`, OEL `current`, ITECH `CURR` |
| setpoints | `CURR {a}`, `VOLT {v}`, `RES {r}`, `POW {p}` (+ `?`, NR2/NR3 → `number`) | Rigol and Siglent accept the short forms (`[:SOUR]` and `[:LEV][:IMM]` are optional per SCPI) |
| readings | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` | OEL `MEAS:ALL:INFO?` |
| short | `INP:SHOR {on}` / `INP:SHOR?` | Siglent `SOUR:SHOR`; Rigol none |
| remote sense | `SYST:SENS {on}` | Rigol `SOUR:SENS`, BK 8600 `REM:SENS` |
| remote/local | `SYST:REM` / `SYST:LOC` (enum-fragment command like OEL) | Rigol, Siglent: none (§4) |
| protection status | `STAT:QUES:COND?` → `bits` | Bit numbers differ per family (see above); Siglent and Korad have none |
| protection clear | `PROT:CLE` | ITECH, BK 8600 only |

The base should hold `identify`, `framing` (`tx "\n"`, `rx` terminator `\n`; definitions
using USBTMC override `rx.mode: message`), the commands above, and the ELL class binding.
Each definition then adds `link`, `models`, ranges and overrides.

---

## 4. Class-contract findings (ELL.yaml)

1. **`remote` is `required: [get, set]`, but Rigol DL3000, Siglent SDL1000X and Korad KEL10x
   have no remote/local command.** (Rigol only simulates the Local key, `SYST:KEY 8`; Korad
   only has a panel lock.) Proposal: make `remote` optional and make `REMOTE` status
   optional, or allow a definition to bind it to a client-side `state` constant
   (`ON` while connected over USBTMC). Today the definition cannot be valid without
   inventing a command.
2. **`short`**: Korad and BK frame mode treat short as a *function/mode* (`:FUNC SHORt`,
   `0x5D 1`), not an independent switch. Its OFF has no inverse command. Keep `short`
   optional and bind it only where it is a real toggle (Siglent, ITECH SCPI, BK 8600, OEL).
3. **Mode gating**: on Korad, setting a setpoint *switches* the mode; on OEL it is silently
   ignored in another mode. Keep the `requires: func_q == "<MODE>"` pattern for every
   family until it is verified.
4. Useful optional status keys seen across families: `OTP` (BK, ITECH, Maynuo), `PS`
   (protection shutdown, Rigol and BK 8600), battery `CAP` (Ah) and `TIME` (Rigol, Siglent,
   Korad).

---

## 5. DSL features that blocked families

| Feature | Needed by | Exact requirement |
|---|---|---|
| **Binary fixed-frame send templates** | ITECH IT8500/BK 8500, Array 3710A, ZKETECH, Atorch | `send` as bytes: constants (`0xAA`), `{config.address}`, an integer-encoded param `{a: {encode: u32le, scale: 10000}}` (A → 0.1 mA), zero padding to a fixed length (`pad_to: 25`), then a checksum byte. Encoders needed: `u8`, `u16le`, `u32le`, `u24be`, `u32be`, and a `base240` pair (ZKETECH) |
| **Checksum generation and verification** | as above, plus Modbus | `sum8` (ITECH/BK/Array: sum of bytes 0..24 mod 256), `sum8_xor:0x44` over the bytes after the header (Atorch), `xor8` over bytes 1..7 (ZKETECH), `crc16_modbus` (Maynuo, OWON Modbus mode). On RX: drop frames that fail the check (with `drain_idle_ms`) |
| **Binary field parse at an offset** | same | Parse step such as `{bin: {v: {at: 3, type: u32le, scale: 0.001}, op: {at: 15, type: u8}, ds: {at: 16, type: u16le}}}` → map. Today only `byte` (first byte) and `samples` (whole payload as a list) exist. Then `bits` on `op`/`ds` |
| **Status-reply setters** | ITECH/BK frame | Setters *do* reply (26-byte status packet `AA addr 12 80 …`). This needs `reply: {bytes: 26}` plus an `expect` (byte 2 = `0x12`, byte 3 = `0x80`, otherwise an error with a mapped message: 0x90 checksum, 0xA0 parameter, 0xB0 unknown, 0xC0 invalid) |
| **Reply header match** | ITECH/BK frame | The reply's byte 2 must echo the command (or be `0x12`). That rejects a late frame from a previous command (`rx.mode: fixed` + verification) |
| **Modbus RTU framing** | Maynuo M97xx, OWON OEL Modbus mode | A link/framing kind `modbus_rtu: {address}` with operations `read_coils(addr)`, `write_coil(addr, bool)`, `read_holding(addr, n)`, `write_multiple(addr, regs)`, and parse types `f32be` over 2 registers and `u16`. CRC16 is handled by the framing |
| **Push / unsolicited stream** | Atorch DL24, Re:load Pro `monitor`, ZKETECH | A `subscribe` poll kind: the device sends reports by itself; the latest decoded frame feeds `status` instead of request/response |
| **UDP link** | Korad KEL10x LAN | `udp: {port: 18190}`, where the local port equals the device port (LAN p.5) |
| **TCP link implementation** | Rigol, Siglent, BK 8600, Keysight, GW (LAN) | Declared but not implemented |
| BLE transport | Atorch | GATT FFE0/FFE1, out of scope for now |

Everything else (all SCPI families, Korad ASCII, Re:load Pro) fits today's DSL. Korad needs
only a `regex` unit-strip parse, and Rigol, Siglent and BK 8600 work over USBTMC `message`
framing.

---

## 6. Recommended first definitions (all `support: awaiting`)

1. `korad_kel10x.yaml`: serial 115200, `\n`, unit-suffixed setters, `requires` mode gating,
   regex unit strip, `remote` → `:SYST:LOCK` (or per §4).
2. `rigol_dl3000.yaml`: USBTMC 1AB1:0E11, extends `scpi_load`, `FUNC?` short-form map,
   `STAT:QUES:COND?` bits, current-range note.
3. `siglent_sdl1000x.yaml`: USBTMC 0xF4EC:?, extends `scpi_load`, `SOUR:SHOR`, long-form
   `FUNC?` map, no protection status.
4. `bk_8600.yaml` and `itech_it8500plus_scpi.yaml`: extend `scpi_load`, with `SYST:REM/LOC`
   on connect/disconnect and `PROT:CLE`.
5. `reload_pro.yaml` (CC-only, ASCII), if the class contract allows CC-only loads.

The binary families (ITECH frame / BK 8500, Array 3710A), Maynuo (Modbus) and Atorch wait for
the §5 features.
