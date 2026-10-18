# PSU coverage: programmable DC bench power supplies

Researched 2026-10-18. Format: `README.md` in this folder. Class contract: `devices/classes/PSU.yaml`.
Defined today: `tenma_72` (Korad protocol, bench-tested on 72-2540) and `owon_spm` (PSU half, bench-tested on SPM3103).
Every new definition starts as **Awaiting confirmation**.

`DOC` = `/home/marek/project/BenchMeshDOC`. Each `DOC/<vendor>_<family>/doc/` has a `SOURCES.md`.
Page citations use the page numbering stated per family: "p." is the printed page and "PDF p." is the
page of the PDF file. Rigol uses chapter-page (`p.2-34`).

**Research limits.** The web-search budget ran out partway through. **No manual was downloaded** for
Keysight (E36300, E36100, E3631A), R&S (HMC804x, NGE100) or ITECH IT6700, because their download
links load through JavaScript. Those sections rest on open-source drivers (sigrok `scpi-pps`,
python-ivi) and vendor conventions, and their confidence is lowered to match. The Rigol DP900
programming guide was not read either.

**Verified against the code (2026-10-18).** These facts about today's DSL and interpreter matter for several families:
- The `bits` parse step needs an unsigned integer or raw bytes. `number` yields an f64, so **`[number, bits]` fails at runtime** (`bm-driver/src/parse.rs`, `Bits` arm). Decimal status registers must be decoded in CEL instead, e.g. `int(isum) % 2 == 1`.
- The `cel` crate is built with `default-features = false`, so the regex feature is off and **CEL `matches()` is unavailable**. `startsWith`, `size` and `int` are available.
- `rx.terminator` takes any non-empty string, and the framer matches a byte sequence, so `"OK\r"` is a valid terminator.
- `serial.flow` is `none | rts_cts | xon_xoff` only. There is no `dsr_dtr`.

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Price band | Protocol | Doc quality | Confidence | DSL fit |
|---|---|---|---|---|---|---|
| 1 | **Korad KA/KD** + rebrands (Tenma 72-xxxx, RND 320, Velleman PS3005D, Stamos) | €80–250 | Korad ASCII, no terminators | medium | high | **works** (`tenma_72` exists; generalise) |
| 2 | **Riden RD60xx** (RD6006/6006P/6012/6012P/6018/6024/6030) + RuiDeng DPS | $60–180 | Modbus RTU | community only | high (map) | **blocked**: Modbus |
| 3 | **OWON SPE/SP/SPS** + Kiprim DC310S/DC605S | €90–300 | OWON SCPI (= `owon_spm` PSU half) | short, replies wrong | high | **works** (extend SPM) |
| 4 | **Siglent SPD3303X/X-E/C** (RS PRO RSPD3303X) | $350–450 | Siglent legacy SCPI | medium | medium | **gap**: hex status |
| 5 | **Rigol DP800** (DP832/A, DP831/A, DP821/A, DP811/A) | $450–900 | SCPI | excellent | high | **works** (USBTMC) |
| 6 | **Manson HCS-3xxx** + B&K 1685B/1687B/1688B, Voltcraft PPS/DPPS, Peaktech 15xx | €120–350 | Manson ASCII, `OK\r` replies | good (B&K) | high | **gap**: implied-decimal ints |
| 7 | **Uni-T UDP3305S/-E/-U** | $330–450 | SCPI | good | high | **works** (USBTMC/RS232) |
| 8 | **GW Instek GPD-2303S/3303S/4303S** (+3303D) | €350–700 | Korad-like ASCII + line terminators | good | high | **works** (status via CEL arithmetic) |
| 9 | **Rigol DP700** (DP711/DP712) | $300–400 | SCPI (DP800 subset), RS232 only | good | high | **works** |
| 10 | **Keysight/Agilent/HP E3631A** family (used) | €150–400 used | SCPI over RS-232/GPIB | not downloaded | medium | mostly; `dsr_dtr` flow missing |
| 11 | **Juntek DPM86xx** (Joy-IT JT-DPM86xx) | $40–150 | ASCII `:01r30=0,` or Modbus | fair | medium | **gap**: implied-decimal ints |
| 12 | **Aim-TTi CPX400DP/CPX200DP, MX100TP/MX180TP** | £500–1500 | TTi ASCII | excellent | high | **works** (serial/USB-CDC) |
| 13 | **Siglent SPD1000X** (SPD1168X/SPD1305X) | $300–400 | Siglent legacy SCPI | medium | medium | **gap**: hex status |
| 14 | **Siglent SPD4000X** (4121X/4306X/4323X) | $700–1100 | modern SCPI | good | medium-high | **works** (USBTMC) |
| 15 | **OWON ODP** (3032/3033/3063/6033/3122/6062) | €200–450 | OWON SCPI + `INST:NSEL` | good | medium | works (per-channel output awkward) |
| 16 | **Uni-T UDP6720/6721/6730/6731** | $120–250 | SCPI, RS232 | medium | medium-high | **works** |
| 17 | **Keysight E36300** (E36311A/12A/13A) | €1100–1600 | SCPI | not downloaded | medium | **works** (USBTMC) |
| 18 | **Hantek PPS2116A/PPS2320A** | $150–250 | lowercase ASCII | poor | medium | **gap**: implied-decimal ints |
| 19 | GW Instek GPP-1326/2323/3323/4323 | €650–1100 | SCPI + GPD legacy | good, no reply examples | medium | works (serial) |
| 20 | OWON P4000 (P4305/P4603) | €150–250 | OWON SCPI subset | good | medium | works |
| 21 | R&S HMC8041/42/43, NGE100/NGE102B/103B | €900–2500 | SCPI, stateful channel select | not downloaded | low | works (`before:` steps) |
| 22 | B&K 9103/9104 | $500–1200 | Manson-like ASCII dialect | good | medium | gap: implied-decimal ints |
| 23 | Keysight E36100B, ITECH IT6700/IT6720, B&K 9200 | €600–1500 | SCPI | not downloaded | low | works (`scpi_psu` base) |
| 24 | Velleman LABPS3005DN ("QJE3005P"), Korad KWR102/103 | €100–300 | Korad-like, newline-terminated | poor / none | low | probably works, capture first |

Dropped or out of scope:
- **OWON SPE500** speaks Modbus only (`files.owon.com.cn/software/Application/SPE500_Modbus.pdf`).
- **Atten PPS3000** (the protocol some Velleman PS3005D units use) was not researched.
- Panel-only units such as the Tenma 72-10480 have no remote interface.

---

## 2. Families

### 2.1 Korad KA/KD series and rebrands (Korad protocol)

Local: `DOC/korad_ka/doc/KA3300_Series_User_Manual_V3.12.pdf` (KA33, contains "KA3000 Series Remote
Control Syntax V2.0"), `DOC/korad_ka/doc/KA3000-6000_Series_User_Manual_V3.41.pdf` (KA30), and
`DOC/tenma_72/doc/Series+Protocol+V2.0+of+Remote+Control.pdf` (TENMA). **The Korad PDFs contain vector
outlines only, so pdftotext returns nothing; render the pages with `pdftoppm` to read them.** Page numbers are the printed booklet pages.

**Models.** Limits are from sigrok `korad-kaxxxxp/api.c` and the KA33 spec table on p.12.

| Model | V / A / W | Ch | Rebrands |
|---|---|---|---|
| KA3003P | 30/3/90 | 1 | Tenma 72-2535, 72-2705 |
| KA3005P | 30/5/150 | 1 | Tenma 72-2540/2710, RND 320-KA3005P, Velleman PS3005D/LABPS3005D, RS Pro RS-3005P (unverified) |
| KD3005P | 30/5/150 | 1 | RND 320-KD3005P |
| KA3010P | 30/10/300 | 1 | Tenma 72-2930 (probable) |
| KA6002P / KA6003P | 60/2/120, 60/3/180 | 1 | Tenma 72-2545 / 72-2550 |
| KA6005P / KD6005P | 60/5/300 | 1 | Tenma 72-2940 (probable) |
| KA3303P / KA3305P | 2× 30 V/3 or 5 A + fixed 5 V/3 A | 2 programmable | Tenma 72-13320/13330 (likely) |
| Stamos S-LS-31 | 30/5 | 1 | IDN has no vendor |

Settable: V 0–31 V (or 0–61 V) in 0.01 V steps; I 0–3.1 A (or 0–5.1 A) in 0.001 A steps.

**Interfaces.**
- USB CDC "Nuvoton USB Virtual COM", **0416:5011**. Rear RS232 also exists; only one of the two is active.
- **9600 8N1**, no flow control (KA30 p.14, KA33 p.9). One V5.8 unit was reported at 19200.

**Identify.** `*IDN?`. Variable length, no terminator. Examples:
- `KORAD KA3005P V5.8 SN:YYYYYYYY`
- `KORADKA3005PV2.0`
- `KORAD KA3305P V2.0` (KA33 p.11 #9)
- `VELLEMANPS3005DV1.3`
- `RND 320-KA3005P V…`
- `TENMA 72-2540 V2.1` (our bench)

Some firmware answers only the first `*IDN?` per power cycle. sigrok tolerates one trailing garbage byte (`\x01`, `\xbc`).

**Commands** (KA33 pp.10–11 = TENMA pp.1–2; `X` = channel 1|2)

| Operation | Send | Reply | Notes |
|---|---|---|---|
| Set I / get I | `ISET<X>:%05.3f` / `ISET<X>?` | none / 5 B `N.NNN` **+ optional 6th byte** | The 6th byte varies by firmware (Tenma V2.1 sends a space) |
| Set V / get V | `VSET<X>:%05.2f` / `VSET<X>?` | none / 5 B `NN.NN` | Tenma silently ignores 3 decimals and exponent forms |
| Measured I / V | `IOUT<X>?` / `VOUT<X>?` | 5 B | VOUT reads 0 while the output is off |
| Output | `OUT1` / `OUT0` | none | **Global** (both channels on the KA3305P, KA33 p.11 #7) |
| Status | `STATUS?` | 1 binary byte | see below |
| OCP / OVP enable | `OCP1/0`, `OVP1/0` | none | The OCP level is ISET; the OVP level is set on the panel only |
| Tracking (KA33xx) | `TRACK0/1/2` | none | indep / series / parallel (KA33 p.11 #13) |
| Memory / beep | `RCL1..5`, `SAV1..5`, `BEEP0/1` | none | |
| Remote / local | (no command) | | Opening USB locks the panel; closing releases it (KA30 p.15) |

**STATUS? bits.** KA33 p.11 documents bit0 = CH1 CV(1)/CC(0), bit1 = CH2, and bit6 = output. Community drivers disagree on bits 2–5 and 7 (tracking, beep, OCP/OVP/lock). **Only bits 0, 1 and 6 are safe to map.**

**Timing.**
- Bench (Tenma): queries need no gap; setters need at least 50 ms.
- sigrok waits 80 ms per request and adds 450 ms on Velleman PS3005D V1.3.

**Changes needed vs `devices/tenma_72.yaml`:**
1. Create a `korad_ka` base and make Tenma extend it. Model regexes must accept every vendor's string, e.g. `^(KORAD ?)?K[AD]3005P`, `^RND 320-KA3005P`, `^VELLEMAN ?PS3005D`.
2. KA3305P: `channels: 2`. OUT, OVP and OCP stay global. Add `ch2_cv` from bit1. Add TRACK as a `configure` command.
3. `ISET?` is 5 or 6 bytes depending on firmware. Use `reply: line` under `rx: idle`, not `bytes: 6`.

**Doc quality** medium. **Confidence** high for KA/KD 3005P and Tenma; medium for KA3305P channel addressing (documented, untested); low for the RS Pro and Stamos IDN strings.
**DSL fit: works today.** Same framing as `tenma_72`.

**Korad-like variants (capture first):**
- **Velleman LABPS3005DN ("QJE3005P" firmware)**, `DOC/velleman_labps/doc/labps3005dn_communication_protocol.pdf` pp.1–2.
  - Commands are `VSET1:`, `ISET1:`, `VOUT1?`, `IOUT1?`, `OUTPUT1`/`OUTPUT0` (not `OUT`) and `STATUS?`, each terminated by `\n`.
  - Forum reports: IDN `QJE3005PV1.0`; some units need the literal two characters `\n` as the terminator; the unit sometimes stops replying.
  - Confidence low.
- **Korad KWR102/KWR103** (30 V/30 A and 60 V/15 A, 300 W). From the `kwr103` crate:
  - Link: 0416:5011 at **115200**, with `\n` terminators.
  - Commands: `VSET:%.3f`, `VSET?`, `ISET:`, `ISET?`, `OUT:1`/`OUT:0`, `OUT?`, `VOUT?`, `IOUT?`.
  - LAN is UDP on port 18190, which the DSL does not support. The user manual (`DOC/korad_ka/doc/KWR_Series_User_Manual_V3.12--.pdf`) has no command list.
  - Confidence low to medium. Works over USB today.

### 2.2 Riden RD60xx (Modbus RTU) and RuiDeng DPS/DPH

No official document; no PDF. Register map from three independent implementations that agree:
<https://github.com/Baldanos/rd6006/blob/master/registers.md>, <https://github.com/ShayBox/Riden/blob/master/riden/register.py>,
sigrok `src/hardware/rdtech-dps/{api,protocol}.c`.

**Models** (register 0 = model ID; the scale is the value of one register count):

| Model | ID | V/I/P max | V scale | I scale | P scale |
|---|---|---|---|---|---|
| RD6006 | 60060–60064 | 60/6/360 | 0.01 | 0.001 | 0.01 |
| RD6006P | 60065 | 60/6/360 | 0.001 | 0.0001 | 0.001 |
| RK6006 | 60066 | 60/6/360 | 0.01 | 0.001 | 0.01 |
| RD6012 | 60120–60124 | 60/12/720 | 0.01 | 0.01 | 0.01 |
| RD6012P | 60125–60129 | 60/6 or 12 | 0.001 | **0.0001 if reg 20 = 0 (6 A range), 0.001 if reg 20 = 1** | 0.001 |
| RD6018 | 60180–60189 | 60/18/1080 | 0.01 | 0.01 | 0.01 |
| RD6024 | ≥ 60241 | 60/24/1440 | 0.01 | 0.01 | 0.01 |
| RD6030 | 60301 | 60/30/~1800 | unverified | unverified | |

**Link.**
- On-board CH340 **1a86:7523**. This is a generic ID, so identify by register 0 and the serial in registers 1–2.
- **115200 8N1**, slave address **1** (both changeable in the menu).
- The WiFi board makes the PSU a Modbus-TCP *client* to the vendor app, so it is unusable as a server.

**Registers** (holding registers; fc 03 read, fc 06 write single, fc 16 write multiple; CRC16/Modbus, poly 0xA001, init 0xFFFF, low byte first):

| Reg | Meaning | Class mapping |
|---|---|---|
| 0 | Model ID | identify |
| 1–2 | Serial (u32, high word first) | identity |
| 3 | Firmware (128 = v1.28) | |
| 8 / 9 | V / I setpoint (R/W) | `voltage`, `current` |
| 10 / 11 | V / I measured | VOUT, IOUT |
| 12–13 | Power (13 alone suffices up to 655 W) | POUT |
| 14 | Input voltage ×0.01 | |
| 15 | Keypad lock | REMOTE |
| 16 | Protection: 0 none, 1 OVP, 2 OCP | OVP / OCP / FAIL |
| 17 | 0 CV, 1 CC | CV / CC |
| 18 | Output enable (R/W) | OUTPUT, `output` |
| 20 | RD6012P current range | scaling |
| 80–83 | M0 active V, I, **OVP, OCP** levels | `voltage_limit` (82), `current_limit` (83) |
| 55–62, 256 | calibration / bootloader | **never write** |

Example frames:
- Read registers 8–18: `01 03 00 08 00 0B <crc>`.
- Output on: `01 06 00 12 00 01 <crc>`. The reply echoes the request (8 bytes).

**RuiDeng DPS3005/5005/5015/5020/8005 and DPH5005** (legacy):
- Needs the USB/BT comms board: CH340, **9600 8N1**, address 1.
- Registers: 0x00 U-SET, 0x01 I-SET, 0x02 UOUT, 0x03 IOUT, 0x04 POWER, 0x06 LOCK, 0x07 PROTECT (1 OVP, 2 OCP, 3 OPP), 0x08 CV/CC, 0x09 ONOFF, 0x0B MODEL (e.g. 5005), 0x50–0x53 M0 U/I/OVP/OCP.
- Scale: V 0.01. I 0.001 on 3005/5005/8005, 0.01 on 5015/5020.

**Quirks.**
- Recent stock firmware reportedly breaks Modbus; the community recommends the UniSoft firmware.
- Libraries use reply timeouts of 0.25–0.5 s with retries.
- OVP/OCP are always-on levels, not switches.

**Confidence** high (map, RD6006/6012/6018/6024 scaling), low (RD6030).
**DSL fit: blocked.** It needs:
1. Modbus RTU framing with CRC16 appended and checked, exception replies (fc | 0x80) and the slave address.
2. Register read (fc 03) and write (fc 06/16) commands.
3. u16 and u32 decoding.
4. Integer↔SI scaling per model *and* per register value (RD6012P), on both the set side and the get side.
5. A numeric identify (match the decimal model ID with a regex such as `^6018\d$`).

This is the largest single gain for hobby coverage.

### 2.3 OWON SP / SPE / SPS (+ Kiprim DC310S/DC605S): extends `owon_spm`

Local: `DOC/owon_sp_spe_sps/doc/SP_and_SPE_SPS_programming_manual.pdf` (PDF pages), plus the SPE and SP user manuals in the same folder.
The programming manual is **word for word the PSU subset of the SPM manual** that `owon_spm.yaml` implements.

**Models** (retailer data and sigrok; medium confidence): SPE3051 30/5/150 · SPE3102 30/10/200 ·
SPE3103 30/10/300 · SPE6102 60/10/200 · SPE6103 60/10/300 · SPE6053 60/5/300 · SP3103 / SP6103 /
SP80xx · SPS (switching). Kiprim DC310S (30 V/10 A) and DC605S (60 V/5 A) are SPE rebrands.

**Link.**
- SPE rear USB is a CH340 **1a86:7523**, **115200 8N1**. Verified on SPE6103 (github kakuteki/owon-spe6103-mcp) and SPE3102 (crate `owon-spe`).
- SP has an RS-232 COM port. `-L` variants have LAN.
- TX `\n`, `\r\n` and `\r` all work; RX ends in `\r\n`.

**Identify** (p.6). `*IDN?` → `OWON,SPE6103,24371986,FV:V5.2.0` (observed). Kiprim: `KIPRIM,DC310S,22371243,FV:V3.7.0` (InstrumentKit PR #471).

| Op | Command | Reply (observed) | Page |
|---|---|---|---|
| Meas V / I / P | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` | `0.000` | p.7–8 |
| Meas all | `MEAS:ALL?` | `0.000,0.000` (V,I; the manual wrongly shows spaces and P) | p.8 |
| Status | `MEAS:ALL:INFO?` | `5.000,0.000,0.000,OFF,OFF,OFF,1`: V, I, P, OVP, OCP, OTP, mode (0 SBY, 1 CV, 2 CC, 3 fail) | p.9 |
| Output | `OUTP ON\|OFF` / `OUTP?` | `ON`/`OFF` (the manual says 1/0) | p.9–10 |
| V / I setpoint | `VOLT <v>` / `VOLT?`, `CURR <i>` / `CURR?` | `16.000` | p.10–11 |
| OVP / OCP level | `VOLT:LIM <v>` / `?`, `CURR:LIM <i>` / `?` | `40.000` | p.10–12 |
| Remote / local | `SYST:REM` / `SYST:LOC` | — | p.12 |

**Notes.**
- There are no OVP/OCP enable commands: protection is always armed at the `*:LIM` level. There is no remote-state query.
- `SYST:STAT?` → `ERR`.
- The output ramps (about 1.5 s to 16 V), so readings right after a set are unsettled.
- The Kiprim "no stable mode query" note means `MEAS:ALL:INFO?` support must be checked per unit.

**Confidence** high. **DSL fit: works today.** Move the PSU half of `owon_spm.yaml` into a base (`owon_psu`). `owon_spe` then extends it with baud 115200, models `^OWON,SP[ES]?\d{4},` and `^KIPRIM,DC(310|605)S,`, and per-model `absolute_limits`.

### 2.4 Siglent SPD3303X / X-E / C (RS PRO RSPD3303X)

Local: `DOC/siglent_spd3303/doc/SPD3303X_QuickStart.pdf` (PDF pages).

**Models.**
- SPD3303X (1 mV/1 mA) and X-E (10 mV/10 mA): CH1/CH2 0–32 V / 0–3.2 A. CH3 is 2.5/3.3/5 V set by DIP switch, on/off only.
- Series up to 64 V, parallel up to 6.4 A.
- SPD3303C: USB only, 30 V/3 A.

**Link.**
- USBTMC, likely **0483:7540** (community memory, *verify*).
- LAN VXI-11 / raw socket 5025. No RS232.

**Identify.** `*IDN?` → `Siglent Technologies,SPD3303X-E,SPD3XHBC3R0662,1.01.01.02.05,V3.0` (EEVblog). SPD3303C: `Siglent, SPD3303C, SPD00001130025, 1.01.01.02` (spaces after the commas).

| Op | Command | Reply | PDF p. |
|---|---|---|---|
| Meas V / I / P | `MEAS:VOLT? CH{ch}`, `MEAS:CURR? CH{ch}`, `MEAS:POWE? CH{ch}` | `30.000` | 36 |
| V / I setpoint | `CH{ch}:VOLT {v}` / `CH{ch}:VOLT?`, `CH{ch}:CURR {i}` / `?` | `25.000` | 36–37 |
| Output | `OUTP CH{ch},ON\|OFF` (CH1–CH3) | — | 37 |
| Status | `SYST:STAT?` → **hex** `0x0224` | see bits below | 38 |
| Tracking | `OUTP:TRACK 0\|1\|2` | — | 37 |
| Select / errors | `INST CH1`, `SYST:ERR?`, `SYST:VERS?` | | 35, 38 |
| Lock | `*LOCK` / `*UNLOCK` | | C guide p.29 |
| OVP / OCP | none | | |

`SYST:STAT?` bits:

| Bit(s) | Meaning |
|---|---|
| 0 | CH1 CC(1)/CV(0) |
| 1 | CH2 CC(1)/CV(0) |
| 2–3 | tracking mode |
| 4 | CH1 on |
| 5 | CH2 on |
| 6–7 | timers |

CH3 state is not in the register.

**Quirks** (EEVblog "SPD3303X-E SCPI Interface issues", Kurokesu `siglent_psu_api`):
- Terminator `\n` only; CR breaks commands.
- Needs **≥100 ms between commands** (300 ms is safe). Two fast `OUTP` commands lost the second one.
- Beeps on every command.
- Not IEEE 488.2 compliant (no `*OPC?`).

**Confidence** medium. **DSL fit:** USBTMC, `min_gap_ms: 150`, `gap_after_write_ms: 300`. **Blocked for OUTPUT/CV/CC** (a required status) by the **hex parse gap**: `number` rejects `0x…`, and `bits` needs an integer.

### 2.5 Rigol DP800 (DP811/A, DP821/A, DP831/A, DP832/A)

Local: `DOC/rigol_dp800/doc/DP800_ProgrammingGuide_EN_2015.pdf` (PG2015, chapter-page numbers), plus a 2013 edition in the same folder.

| Model | Ch | Ratings | Settable |
|---|---|---|---|
| DP832/A | 3 | 30V/3A, 30V/3A, 5V/3A | 0–32 V / 0–3.2 A (CH3 5.3 V) |
| DP831/A | 3 | 8V/5A, 30V/2A, **−30V/2A** | CH3 is negative |
| DP821/A | 2 | 60V/1A, 8V/10A | 63 V/1.05 A; 8.4 V/10.5 A |
| DP811/A | 1 | ranges 20V/10A or 40V/5A (`:OUTP:RANG P20V\|P40V`, p.2-62) | |

Non-A models have lower resolution unless licensed (p.2-8). Setpoint replies: A models `00.000`/`3.000`, non-A `00.00`.

**Link.**
- USBTMC, VID 0x1AB1, PID likely **0x0E11** (*verify*).
- LAN (VXI-11 / raw socket).
- RS232 **9600 8N1**; **TX `\r\n` over RS232** (p.1-2); replies end in `\n`.

**Identify** (p.2-24). `*IDN?` → `RIGOL TECHNOLOGIES,DP831A,DP8A000001,00.01.01`.

| Op | Command | Reply | Page |
|---|---|---|---|
| Meas V,I,P | `:MEAS:ALL? CH{ch}` | `2.0000,0.0500,0.100` | 2-34 |
| Meas V / I / P | `:MEAS? CH{ch}`, `:MEAS:CURR? CH{ch}`, `:MEAS:POWE? CH{ch}` | `5.0000` | 2-35/36 |
| V setpoint | `:SOUR{ch}:VOLT {v}` / `:SOUR{ch}:VOLT?` | `7.500` | 2-90 |
| I setpoint | `:SOUR{ch}:CURR {i}` / `?` | `1.5000` | 2-83 |
| Both | `:APPL CH{ch},{v},{i}` / `:APPL? CH{ch}` | `CH1,5.000,1.0000` | 2-8 |
| Output | `:OUTP CH{ch},ON\|OFF` / `:OUTP? CH{ch}` | `ON`/`OFF` | 2-63 |
| CV/CC | `:OUTP:CVCC? CH{ch}` | `CV`/`CC`/`UR` | 2-53 |
| OVP level / enable / tripped / clear | `:OUTP:OVP:VAL CH{ch},{v}` (`?`), `:OUTP:OVP CH{ch},ON\|OFF` (`?`), `:OUTP:OVP:QUES? CH{ch}` → `YES`/`NO`, `:OUTP:OVP:CLEAR CH{ch}` | | 2-58…2-61 |
| OCP (same pattern) | `:OUTP:OCP:VAL`, `:OUTP:OCP`, `:OUTP:OCP:QUES?`, `:OUTP:OCP:CLEAR` | | 2-54…2-57 |
| OTP | `:SYST:OTP ON\|OFF` / `?` | | 2-122 |
| Remote / local | `:SYST:REM`, `:SYST:LOC`, `:SYST:RWL ON` | | 2-121…2-124 |
| Errors | `:SYST:ERR?` | `-113,"Undefined header…"` | 2-118 |
| Tracking | `:OUTP:TRAC CH{ch},ON\|OFF` | | 2-66 |

Without a `CH{n}` argument or a `SOUR{n}` suffix, commands act on the `:INST:NSEL` selection. Always use the explicit forms. Setters return nothing.

**Quirks.**
- Early firmware failed over USBTMC on Linux/macOS; 00.01.16 works (EEVblog).
- The inter-command delay is undocumented. Start at 50 ms, or use `*OPC?`.

**Confidence** high (also sigrok `rigol_dp800_cmd`). **DSL fit: works today** over USBTMC (`rx: message`) or RS232.
- Status: `MEAS:ALL?` → `fields [v,i,p]`; `OUTP?` → OUTPUT; `CVCC?` → CV/CC; `OVP:QUES?`/`OCP:QUES?` → `== "YES"`.
- REMOTE is not queryable, so track it in `state`.
- DP811 ranges have no class operation (an optional `range` extension would be needed).

### 2.6 Manson HCS-3xxx + B&K 1685B/1687B/1688B (+ Voltcraft, Peaktech)

Local: `DOC/manson_hcs/doc/conrad_000513911DS01_hcs_usb_protocol.pdf` (MANSON),
`DOC/bk_1680/doc/1685B_Series_programming_manual.pdf` (BK PM, 15 pp.),
`DOC/bk_1680/doc/Remote_communication_1696_1698.pdf`. sigrok `manson-hcs-3xxx`.

**Models** (sigrok api.c). The V step is 0.1 V on every model, with a minimum of 1.0 V.

| Model | V | I | I step |
|---|---|---|---|
| HCS-3100 | 18 V | 10 A | 0.1 A |
| HCS-3102 | 36 V | 5 A | **0.01 A** |
| HCS-3104 | 60 V | 2.5 A | **0.01 A** |
| HCS-3150 | 18 V | 15 A | 0.1 A |
| HCS-3200 | 18 V | 20 A | 0.1 A |
| HCS-3202 | 36 V | 10 A | 0.1 A |
| HCS-3204 | 60 V | 5 A | **0.01 A** |
| HCS-3300/3302/3304 | 16/32/60 V | 30/15/8 A | 0.1 A |
| HCS-3400/3402/3404 | 16/32/60 V | 40/20/10 A | 0.1 A |
| HCS-3600/3602/3604 | 16/32/60 V | 60/30/15 A | 0.1 A |

Clones (the B&K match is inferred from identical ratings and command set):
- B&K 1688B = HCS-3200, 1687B = HCS-3202, 1685B = HCS-3204.
- Voltcraft PPS-11810/11360/11603/13610/16005/11815 and DPPS-32-15.
- Peaktech 1565/1575/1580.
- B&K 1696–1698 use the same set with a 2-digit RS-485 address (`GETD00`).

**Link.** CP2102 **10c4:ea60** (generic ID), **9600 8N1**, no flow (BK PM p.2, p.11). The mode switch must be in "Normal".

**Protocol.**
- Commands end with `\r`.
- **Every command is answered**: queries return `<data>\rOK\r`, setters return `OK\r` (MANSON p.1; BK PM p.11–13).
- There is no `*IDN?`. Identify with `GMOD` → `3202` or `HCS-3302` (sigrok), or with `GMAX` as a fingerprint (B&K: `180200` = 18.0 V/20.0 A).

| Op | Command | Reply | Format | Page |
|---|---|---|---|---|
| Measured V/I/mode | `GETD` | `030201450` | V×0.01 (4 digits), I×0.01 (4 digits), M 0 = CV, 1 = CC | BK PM p.12 |
| Setpoints | `GETS` | `025051` | V×0.1 (3 digits), I×0.1 or ×0.01 per model (3 digits) | BK PM p.11 |
| Set V / I | `VOLT<vvv>` / `CURR<iii>` | `OK` | `VOLT127` = 12.7 V, `%03d` | BK PM p.11 |
| Output | `SOUT0` = **ON**, `SOUT1` = OFF (inverted) | `OK` | | BK PM p.12 |
| Output state | none | | sigrok infers on from GETD V ≠ 0 | |
| OVP / OCP level | `SOVP<vvv>` / `GOVP`, `SOCP<iii>` / `GOCP` | | | BK PM p.12–13 |
| Max | `GMAX` | `VVVIII` | | BK PM p.13 |
| Lock | `SESS` / `ENDS` | `OK` | reportedly times out after about 5 s | 1696 doc p.2 |

**Confidence** high for the commands, medium for per-model current decimals.
**DSL fit: framing works today** (`tx "\r"`, `rx: {mode: terminator, terminator: "OK\r"}`). Strip the `\r` between data and OK with `trim`; setters become `reply: line` expecting an empty line.
**Blocked for setpoint and measurement operations** by implied-decimal integers:
- Setters need value × 10 formatted `%03d`.
- `voltage` gets (`as: si`) need a parse-side scale.
- The current scale is per model.

Status can be computed today with CEL arithmetic on `number` of the GETD reply: `int(x/100000)/100.0` for V and `int(x) % 10` for mode. OUTPUT has no query.

**B&K 9103/9104** (`DOC/bk_9100/doc/9103_9104_programming_manual.pdf`, 9 pp.) is a dialect of the same command family:
- Link: 9600 8N1 over a USB VCP, `\r` terminator, `OK\r` replies.
- **`SOUT1` = ON** (the reverse of the 1685B). `GOUT` → 0|1.
- `VOLT<p><vvvv>` and `CURR<p><iiii>` take a preset digit plus 4 digits at 0.01 resolution.
- `GETD` → `050001000`. `SOVP`/`GOVP` and `SOCP` are ×0.01. `SESS`/`ENDS` lock and unlock; `GALL` dumps all state.
- Model ratings are unstated. Confidence medium. Same scale gap as above.

### 2.7 Uni-T UDP3305S / -E / -U (UDP3000S)

Local: `DOC/unit_udp/doc/UDP3000S_Series_Programming_Manual_V1.1.pdf` (U3S, "n / 49" footer pages), plus the user manual in the same folder.

**Models.**
- CH1/CH2 0–30 V / 0–5 A (settable 33 V / 5.2 A).
- CH3 0–6 V / 3 A, programmable.
- CH4 fixed 5 V USB.
- Series 60 V/5 A, parallel 30 V/10 A.
- The -E model reads back with one fewer voltage decimal.

**Link** (U3S p.5).
- USBTMC **0483:5740**.
- LAN VXI-11 / socket 5025.
- RS232 (crossed cable; 9600 seen in the field).
- Terminator **`\n`** both ways.

**Identify.** `Uni-Trend,UDP3305S,<SN>,<fw>` (philpagel TestController; also `-E`).

| Op | Command | Reply | Page |
|---|---|---|---|
| Meas V,I,P | `:MEAS:ALL? CH{ch}` | `05.10,0.089,00.45` | 18–19 |
| Meas V / I / P | `:MEAS:VOLT? CH{ch}`, `:MEAS:CURR? CH{ch}`, `:MEAS:POWE? CH{ch}` | | 19 |
| V / I setpoint | `:SOUR{ch}:VOLT <v>` / `?`, `:SOUR{ch}:CURR <i>` / `?` | `25.00`, `5.000` | 13–15 |
| Both | `:APPL CH1,15.00V,2.000A`; `:APPL? CH1,VOLT` → `CH1,15.00` | | 12 |
| Output | `:OUTP:STAT CH{ch},ON\|OFF` (also `ALL`) / `:OUTP:STAT? CH{ch}` | `ON`/`OFF` | 16–17 |
| CV/CC | `:OUTP:CVCC? CH{ch}` | `CV`/`CC` | 17 |
| OVP level / enable | `:SOUR{ch}:VOLT:PROT <v>` / `?`; `:SOUR{ch}:VOLT:PROT:STAT ON\|OFF` / `?` | | 14–15 |
| OCP level / enable | `:SOUR{ch}:CURR:PROT <i>` / `?`; `…:CURR:PROT:STAT` | | 15–16 |
| Tripped | `:STAT:QUES:INST:ISUM{ch}?` (latched event): bit0 CC, bit1 CV, **bit2 OVP, bit3 OCP** | integer | 9, 42–43 |
| OTP | `:STAT:QUES?` bit4 | | 6 |
| Mode | `:SOUR:MODE NORM\|SER\|PARA`, then wait ≥500 ms | | 13 |
| Lock | `:SYST:LOCK ON\|OFF` / `?`; `:SYST:RWL ON` | | 48–49 |

**Quirks** (pnuding/udp3000s-control).
- Only the latched ISUM event register shows OVP/OCP trips; the CONDition register clears within about 150 ms.
- Numbers come back as fixed decimals, despite p.4.
- philpagel uses a 0.2 s gap.

**Confidence** high. **DSL fit: works today** over USBTMC or RS232. Decode ISUM with CEL arithmetic (`[number, bits]` fails, see the verified notes at the top). Leave SER/PARA out at first.

### 2.8 GW Instek GPD-2303S / 3303S / 4303S (+ GPD-3303D)

Local: `DOC/gwinstek_gpd/doc/GPD-X303S_User_Manual.pdf` (PDF pages) and `GPD-3303D_User_Manual.pdf`.

**Models** (p.13).
- GPD-2303S: 2× 30 V/3 A.
- GPD-3303S: 2× 30 V/3 A + CH3 2.5/3.3/5 V, not programmable.
- GPD-4303S: 2× 30 V/3 A + CH3 5 V/1 A + CH4 5 V/3 A.
- Settable range: 0–32.000 V, 0–3.200 A (p.48). Remote channels 1–2 (1–4 on the 4303S).
- GPD-3303D: 10 mV/10 mA resolution.

These are not Korad clones, but they use the same command style.

**Link.**
- FTDI virtual COM (vid:pid undocumented; 0403:6001 is the FTDI default).
- **9600** (default), 57600 or 115200, 8N1, no flow (p.43). Change with `BAUD0/1/2` = 115200/57600/9600 (p.51). The 3303D runs at 9600 only.

**Remote.** Entered automatically on connect. Exit with `LOCAL`; re-enter with `REMOTE` (p.44, p.48).

**Identify** (p.50). `*IDN?` → `GW INSTEK,GPD-2303S,SN:ER915277,V2.10` (sigrok).

| Op | Command | Reply | Page |
|---|---|---|---|
| I setpoint | `ISET<X>:<NR2>` / `ISET<X>?` | `2.234A` | 48 |
| V setpoint | `VSET<X>:<NR2>` / `VSET<X>?` | `20.345V` | 48–49 |
| Meas I / V | `IOUT<X>?` / `VOUT<X>?` | `0.000A` / `12.000V` | 49 |
| Output | `OUT1` / `OUT0` (global) | none | 50 |
| Status | `STATUS?` | 8 ASCII `0/1` characters (see below) | 50 |
| Tracking | `TRACK0/1/2` | none | 49 |
| Remote / local | `REMOTE` / `LOCAL` | none | 48, 51 |
| Errors | `ERR?` | `No Error.` | 52 |

`STATUS?` characters, first to last:

| Position | Meaning |
|---|---|
| 1 | CH1 CV(1)/CC(0) |
| 2 | CH2 CV(1)/CC(0) |
| 3–4 | tracking (01 indep, 11 series, 10 parallel) |
| 5 | beep |
| 6 | output |
| 7–8 | baud |

Firmware older than 2.0 answers 3 lines in a different format.

**Framing and quirks.**
- TX: `\n` or `\r\n`, at most 15 characters per command (p.46).
- RX: **`\r` on old firmware, `\r\n` on new** (PyGPD3303S). Use terminator `\r` plus `skip_leading: "\n"`.
- Setters do not reply.
- Response time is at least 10 ms; 70 ms on the 3303D (3303D PDF p.20).
- The first `*IDN?` may return `Invalid Character.` (sigrok). Drain on connect and retry identify.
- No OVP/OCP.

**Confidence** high (manual plus sigrok plus PyGPD3303S); medium for the 3303D.
**DSL fit: works today.** Parse readings with `[trim, {replace: {from: "V", to: ""}}, number]`.
- `STATUS?`: CEL `matches()` is unavailable, so use `number` on the 8-digit string and decimal arithmetic. Example: CH1 CV = `int(st / 10000000.0) % 10 == 1`; output = `int(st / 100.0) % 10 == 1`.
- A parse step that splits a string into characters would be cleaner.

### 2.9 Rigol DP700 (DP711, DP712)

Local: `DOC/rigol_dp700/doc/` (DP700 user guide, 105 pp.; chapter-page numbers).

**Models.**
- DP711: 30 V/5 A, settable 32 V/5.3 A.
- DP712: 50 V/3 A, settable 53 V/3.2 A.
- Both 150 W, 1 channel.

**Link.**
- **RS232 only** (straight cable). No USB, no LAN (p.1-2).
- **9600 8N1** by default; 7200–115200; no flow (p.2-31, p.5-1).
- TX **`\n`** (p.1-2).

**Identify.** `RIGOL TECHNOLOGIES,DP711,<sn>,<fw>` (p.2-4).

The commands are the DP800 set minus OTP/LAN:

| Op | Command | Page |
|---|---|---|
| Measure V,I,P | `:MEAS:ALL?` → `v,i,p` | 2-9 |
| Measure V / I / P | `:MEAS?`, `:MEAS:CURR?`, `:MEAS:POWE?` | 2-9 |
| Setpoints | `:APPL CH1,v,i`; `:SOUR:VOLT`, `:SOUR:CURR` (+ `?`) | 2-2, 2-22, 2-25 |
| Output | `:OUTP CH1,ON\|OFF`; `:OUTP? CH1` | 2-21 |
| CV/CC | `:OUTP:CVCC?` | 2-14 |
| OVP / OCP | `:OUTP:OVP:VAL`, `:OUTP:OVP ON\|OFF`, `:OUTP:OVP:QUES?`, `:OUTP:OVP:CLEAR` (OCP likewise) | 2-15…2-20 |
| Remote / local | `:SYST:REM`, `:SYST:LOC`, `:SYST:RWL` | 2-35, 2-37 |

Setpoint replies have 2 decimals (`5.00,1.00`). The USB-RS232 adapter's vid:pid identifies the cable, not the instrument, so `match` relies on the IDN serial.

**Confidence** high. **DSL fit: works today**, as an extension of a shared `rigol_dp` base.

### 2.10 Keysight/Agilent/HP E3631A (+ E3632A/33A/34A, E3640A–49A)

**No manual downloaded.** Fetch the "E3631A User's Guide" by hand into `DOC/keysight_e3631a/doc/`.
Commands are from python-ivi (`agilentE3600A.py`, `scpi/dcpwr.py`).

**Models.**
- E3631A: P6V 6 V/5 A, P25V 25 V/1 A, N25V 0 to −25 V/1 A (settable 6.18/5.15, 25.75/1.03). No OVP/OCP. ±25 V tracking.
- E3632A–34A and E3640A–49A: single output with 2 ranges (`VOLT:RANG P15V|P30V`), plus OVP/OCP.

**Link.**
- RS-232 (DTE, null-modem cable) + GPIB.
- From memory, *verify*: 9600 by default, 8N1 or 7E1/7O1, **2 stop bits**, **DTR/DSR handshake**.
- **`SYST:REM` is required before any other RS-232 traffic**; end with `SYST:LOC`.

**Identify.** `HEWLETT-PACKARD,E3631A,0,<fw>` or `Agilent Technologies,E3631A,0,2.1-5.0-1.0`. The serial field is 0.

| Op | Command |
|---|---|
| Select output | `INST:NSEL 1\|2\|3` or `INST:SEL P6V\|P25V\|N25V` |
| Both setpoints | `APPL P6V,<v>,<i>` / `APPL? P6V` → `"+5.000000E+00,+1.000000E+00"` (quoted) |
| V / I setpoint | `VOLT <v>` / `VOLT?`, `CURR <i>` / `CURR?` (selected output) |
| Measure | `MEAS:VOLT? P6V`, `MEAS:CURR? P25V` → NR3 |
| Output | `OUTP ON\|OFF` / `OUTP?` → `0`/`1` (**global**) |
| CV/CC | `STAT:QUES:INST:ISUM<n>:COND?`: bit0 CC, bit1 CV |
| E3632A protection | `VOLT:PROT <v>`, `VOLT:PROT:STAT ON`, `CURR:PROT …`, `VOLT:PROT:CLE` |

Replies end in `\r\n` (*verify*). Measurements are slow, so allow `timeout_ms` of at least 1000.

**Confidence** medium for the commands, low-medium for the RS-232 details.
**DSL fit: mostly works.**
- Use `session.on_connect: [SYST:REM]` and `on_disconnect: [SYST:LOC]`.
- Per channel, use the `P6V|P25V|N25V` enum in the send template, or `before: INST:NSEL {ch}`.
- **Gap:** `flow: dsr_dtr` is missing. The `dtr: high` workaround ignores the instrument's busy signal.
- Check that `8N2` passes the validator.
- GPIB-only units are out of scope.

### 2.11 Juntek / JUNCE DPM86xx (Joy-IT JT-DPM86xx)

Local: `DOC/juntek_dpm86xx/doc/JT-DPM86XX_Communication-protocol_2025-07-22.pdf` (PDF pages).

**Models.** DPM8605 (60 V/5 A), 8608, 8616, 8624, 8650 (60 V/50 A).
- `-485` variants come with a USB-RS485 dongle (typically CH340).
- Baud 2400–115200, default 9600. Address 01–99.
- Protocol selected by menu or fn 15: 0 = simple ASCII, 1 = Modbus.
- DPH8909/8920 buck-boost: not in this document; reportedly the same protocol (unverified).

**Simple ASCII protocol.** Frame `:<aa><r|w><fn>=<operand>,\r\n`. Values are **implied-decimal integers** (V×100, I×1000).

| Op | Send | Reply | PDF p. |
|---|---|---|---|
| Max V (identify) | `:01r00=0,` | `:01r00=6000` (60.00 V) | 3 |
| Max I | `:01r01=0,` | `:01r01=24000` → DPM8624 | 3 |
| V / I setpoint | `:01r10=0,` / `:01r11=0,` | `:01r10=1234` (12.34 V) | 3 |
| Output | `:01r12=0,` | `=0` / `=1` | 3 |
| Measured V / I | `:01r30=0,` / `:01r31=0,` | `:01r30=2345` | 3 |
| CV/CC | `:01r32=0,` | 0 CV / 1 CC | 3 |
| Set V / I / output | `:01w10=1234,`, `:01w11=12345,`, `:01w12=1,` | `:01ok` (community) | 4 |
| Set V+I | `:01w20=1234,12345,` | | 5 |

There is no OVP/OCP, lock or power command. The manual's examples contain typos.

**Modbus mode** (PDF p.5–10):

| Register | Meaning |
|---|---|
| 0x0000 | V setpoint, ×0.01 |
| 0x0001 | I setpoint, ×0.001 |
| 0x0002 | output switch |
| 0x1000 | state (0 off, 1 CV, 2 CC) |
| 0x1001 / 0x1002 | measured U / I |

**Confidence** medium. **DSL fit:** the ASCII framing works today (`\r\n` both ways; `regex` parse of `=(\d+)`).
- Setters are blocked by the implied-decimal gap.
- Readings scale in CEL for status, but not for `as: si` operations.
- Modbus mode is blocked, as for Riden.

### 2.12 Aim-TTi CPX400DP / CPX200DP, MX100TP / MX180TP (+ QPX, PL-P probable)

Local: `DOC/tti_cpx/doc/CPX400D+DP_Instruction_Manual_EN_48511-1480_14.pdf` (CPX, PDF pages),
`DOC/tti_mx/doc/MX100T_MX100TP_Instruction_Manual_EN_48511-1610_9.pdf` (MX).

**Models.**
- CPX400DP: 2× 60 V/20 A, 420 W each (PowerFlex).
- CPX200DP: 2× 60 V/10 A, 180 W.
- MX100TP: 3 outputs with ranges (`VRANGE<N>`).
- MX180TP: 3 outputs up to 120 V/20 A (unverified).
- Non-P models have no remote interface.

**Link** (CPX PDF p.17–20).
- RS-232 **9600 8N1, XON/XOFF**.
- USB CDC (baud ignored; vid:pid unknown, probably 103e:xxxx).
- LAN raw socket **TCP 9221**. GPIB optional.

**Protocol.**
- TTi ASCII. Commands end with **LF**; replies end with **CR LF** (PDF p.27).
- Setters do not reply.
- Typical processing time is under 25 ms (PDF p.3).

**Identify.** `*IDN?` → `THURLBY THANDAR,CPX400DP,279730,1.00 – 1.00` (PDF p.20). Newer units may report `TTi` or `Aim-TTi`.

| Op | Command | Reply | CPX PDF p. |
|---|---|---|---|
| Set V / I | `V{ch} <nrf>`, `I{ch} <nrf>` | none | 28 |
| Get setpoint | `V{ch}?` / `I{ch}?` | `V1 12.50` → `split " " [1]` | 29 |
| Measured | `V{ch}O?` / `I{ch}O?` | `12.003V` / `0.512A` → strip the unit | 29 |
| Output | `OP{ch} 1\|0`, `OPALL 1\|0`; `OP{ch}?` | `1`/`0` | 29 |
| OVP / OCP level | `OVP{ch} <nrf>` / `?` → `VP1 66.00`; `OCP{ch} <nrf>` / `?` → `CP1 22.00` | | 28–29 |
| OVP / OCP enable | MX only: `OVP{ch} ON\|OFF` (MX p.35–36); on CPX always armed | | |
| Trip reset | `TRIPRST` | | 29 |
| Status | `LSR{ch}?` (**read clears it**): bit0 CV, bit1 CC, bit2 OVP, bit3 OCP, bit4 power limit, bit6 hard trip | NR1 | 23 |
| Remote / local | Any command enters remote; `LOCAL` returns to local | | 17, 30 |
| Tracking / range | `CONFIG 0\|2` (CPX), `VRANGE{ch} <n>` (MX) | | 29–30 |

**Quirks.**
- Entering remote resets both CPX ranges to 60V/20A (PDF p.17).
- LSR bits are latched events, not a steady CV/CC state. Treat CC as "entered since the last poll".

**Confidence** high (CPX400DP, MX100TP), medium for the others.
**DSL fit: works today**: `serial {baud: 9600, flow: xon_xoff}`, `tx "\n"`, `rx terminator "\n"` with `strip_trailing "\r"`. LAN needs TCP.

### 2.13 Siglent SPD1000X (SPD1168X 16 V/8 A, SPD1305X 30 V/5 A)

Local: `DOC/siglent_spd1000/doc/SPD1000X_UserManual_ProgrammingGuide.pdf` (PDF pages).

**Link.**
- USBTMC. The manual's example uses `0xF4EC:0x1300`; community reports mention 0483:7540. *Verify.*
- LAN socket 5025 (p.53).
- `\n` required (p.77).

**Identify** (p.56). `Siglent, SPD1168X, SPD1XDAD1R0001, 2.01.01.06, V1.0`. The prefix may be `Siglent Technologies`; *verify*.

| Op | Command | PDF p. |
|---|---|---|
| Measure | `MEAS:VOLT? CH1`, `MEAS:CURR? CH1`, `MEAS:POWE? CH1` → `16.000` | 57 |
| Setpoints | `CH1:VOLT {v}` / `?`, `CH1:CURR {i}` / `?` | 58 |
| OVP / OCP level | `OVP {v}` / `OVP?`, `OCP {i}` / `OCP?` | 59 |
| Sense | `MODE:SET 2W\|4W` | 59 |
| Output | `OUTP CH1,ON\|OFF`; `OUTP:RESE:PROT` clears the trip pop-up | 60 |
| Status | `SYST:STAT?` hex: bit0 CV/CC, bit4 output, bit5 4W | 61–62 |
| Lock | `*LOCK` / `*UNLOCK` | 65 |

There is no OVP/OCP enable and no trip query. Pace like the SPD3303X; a user waited 2 s after `OUTP ON`.

**Confidence** medium. **DSL fit:** blocked by the **hex parse gap**, as for the SPD3303X.

### 2.14 Siglent SPD4000X (SPD4121X, SPD4306X, SPD4323X; 4 channels)

Local: `DOC/siglent_spd4000/doc/SPD4000X_UserManual_ProgrammingGuide.pdf` (PDF pages).

**Models** (p.23).

| Model | CH1 | CH2 / CH3 | CH4 | Total |
|---|---|---|---|---|
| SPD4323X | 6 V/3.2 A | 32 V/3.2 A | 6 V/3.2 A | 240 W |
| SPD4121X | 15 V/1.5 A | 12 V/10 A | 15 V/1.5 A | 285 W |
| SPD4306X | 15 V/1.5 A | 30 V/6 A | 15 V/1 A | 400 W |

**Link.**
- USBTMC (vid:pid unknown, probably 0xF4EC).
- LAN socket 5025 (p.57).
- A proper IEEE 488.2 SCPI dialect. Replies end in `\n`; numbers have 6 decimals.

**Identify** (p.59). `Siglent Technologies,SPD4306X,0123456789,4.1.2.4`.

| Op | Command | Reply | PDF p. |
|---|---|---|---|
| Measure | `MEAS:VOLT? CH{ch}`, `MEAS:CURR? CH{ch}`, `MEAS:POWER? CH{ch}` | `2.991442` | 78 |
| CV/CC | `MEAS:RUN:MODE? CH{ch}` | `CV`/`CC` | 78 |
| Setpoints | `VOLT CH{ch},{v}` / `VOLT? CH{ch}`; `CURR CH{ch},{i}` / `CURR? CH{ch}` | `3.000000` | 61–62 |
| OVP | `OVP CH{ch},{v}` / `?`; `OVP:PROT:STAT? CH{ch}` → `0`/`1` | | 62 |
| OCP | `OCP CH{ch},{i}`; `OCP:STAT CH{ch},ON\|OFF` (`?`); `OCP:PROT:STAT? CH{ch}` | | 63–64 |
| Output | `OUTP CH{ch},ON\|OFF` / `OUTP? CH{ch}` → `0`/`1`; `OUTP:ALL` | | 64 |
| Lock / clear | `LOCK ON\|OFF`; `RESET:PROT CH{ch}` | | 70 |

The channel is the first argument (`VOLT CH1,3`), unlike the SPD3303X. OVP is always armed.
**Confidence** medium-high. **DSL fit: works today** (USBTMC; start with a 50 ms gap). Do not extend the legacy Siglent base.

### 2.15 OWON ODP (ODP3032/3052, 3033/3063/6033, 3122/6062)

Local: `DOC/owon_odp/doc/ODP_Series_Power_Supply_Programming_Manual.pdf` (PDF pages) and `ODP3032_PowerSupply_USER_MANUAL.pdf`.

**Models.**
- ODP3032: 2× 30 V/3 A + fixed 5 V/3 A. ODP3052 is the 5 A variant.
- ODP6033: 60 V/3 A ×2 + 6 V/3 A.

**Link.**
- USB (needs OWON's driver, so vendor serial or USBTMC) and RS-232. Baud and vid:pid are undocumented; assume 115200.
- **Local mode rejects everything except common and SYST commands. Send `SYST:REM` first** (p.5, p.7).

**Identify.** `OWON,ODP6033,1715040,FV:V1.0.2` (p.6).

| Op | Command | PDF p. |
|---|---|---|
| Channel select | `INST:NSEL 1\|2\|3` / `?` | 11–12 |
| Measure (selected) | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` | 9–10 |
| Measure all channels | `MEAS:VOLT:ALL?`, `MEAS:CURR:ALL?` → `1.000, 2.000, 3.000` | 10–11 |
| Setpoints | `VOLT`/`CURR` (selected), `APP:VOLT v1,v2,v3` / `APP:VOLT?` | 8–9, 16, 18 |
| OVP / OCP level | `VOLT:LIM`, `CURR:LIM` (selected), `…:LIM:ALL` | 16–19 |
| Output | `OUTP 0\|1` (all); `CHAN:OUTP:ALL 1,0,1` / `?` (per channel, list only) | 13, 15–16 |
| Tracking / series / parallel | `OUTP:TRAC`, `OUTP:SER`, `OUTP:PAR` | 13–15 |

There is no CV/CC or trip query.
**Confidence** medium. **DSL fit: works today**, with `before: INST:NSEL {ch}` steps and the `*:ALL?` list queries split per channel.
- Per-channel output set needs a computed `from` param that rebuilds the list. Otherwise expose output as global.

### 2.16 Uni-T UDP6720 / 6721 / 6730 / 6731

Local: `DOC/unit_udp/doc/UDP6720_Series_Programming_Manual_V1.2.pdf` (U67) and `UDP6700_Series_Data_Sheet_V1.0.pdf`.

**Models.** UDP6720 60 V/5 A/100 W · 6721 60 V/8 A/180 W · 6730 40 V/30 A/360 W · 6731 80 V/15 A/360 W. 1 channel.

**Link.**
- RS232 only. Baud is set in the menu.
- Input ends at `\n`; replies end with CR LF (p.2).
- RS485 adds an `ADDR <n>::` prefix. A Modbus mode also exists (p.21 ff.).

**Identify** (p.20). `UNIT,UDP6722,UNLICENSED,REV1.21`.

| Op | Command | Page |
|---|---|---|
| Measure | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?`, `MEAS:ALL?` → `v,i,p` | 11–12 |
| Setpoints | `VOLT <v>` / `?`, `CURR <i>` / `?`, `APPL v,i`, `APPL:ALL v,i,ovp,ocp` | 8–11 |
| OVP | `VOLT:PROT <v>`, `VOLT:PROT:STAT ON\|OFF`, `VOLT:PROT:TRIP?` → 0/1, `VOLT:PROT:CLE` | 9–10 |
| OCP | `CURR:PROT …` (same pattern) | 8–9 |
| Output | `OUTP ON\|OFF` / `OUTP?` → `ON`/`OFF` | 7 |
| CV/CC | `OUTP:CVCC?` → `CV`/`CC` | 7 |
| Remote | `SYST:REM` | V1.0 manual p.3 |

**Confidence** medium-high. **DSL fit: works today** (confirm the baud).

### 2.17 Keysight E36300 (E36311A/12A/13A) and E36100B

**No manuals downloaded.** Fetch the E36300 User's Guide and the E36100B Programming Guide by hand.

**Models.**
- E36312A: 6 V/5 A + 2× 25 V/1 A.
- E36313A: 6 V/10 A + 2× 25 V/2 A.
- E36311A: 6 V/5 A, +25 V/1 A, −25 V/1 A.
- E36102B–06B, single output: 6 V/5 A, 20 V/2 A, 35 V/1 A, 60 V/0.6 A, 100 V/0.4 A (from memory).

**Link.**
- USBTMC, VID **0x2A8D** (E36312A PID 0x1102, *unverified*).
- LAN 5025 / VXI-11.

**Identify.** `Keysight Technologies,E36312A,MY5xxxxxxx,2.1.7-1.0.4-1.16`.

Commands (sigrok `keysight_e36300a_cmd`, tested on an E36312A):

| Op | Command |
|---|---|
| Measure | `MEAS:VOLT? (@{ch})`, `MEAS:CURR? (@{ch})` → NR3 |
| Setpoints | `VOLT <v>,(@{ch})` / `VOLT? (@{ch})`, `CURR …` (or `INST:NSEL {ch}` first) |
| Output | `OUTP ON\|OFF,(@{ch})` / `OUTP? (@{ch})` → 0/1 |
| OVP | `VOLT:PROT`, `VOLT:PROT:TRIP?`, `VOLT:PROT:CLE` |
| OCP | `CURR:PROT:STAT 0\|1`, `CURR:PROT:TRIP?` |
| Status | `STAT:QUES:INST:ISUM<n>:COND?`: bit0 CC, bit1 CV, bit4 OT, bit9 OV, bit10 OC |
| Remote | `SYST:REM` / `SYST:LOC` |

E36100B uses the same tree without channels.
**Confidence** medium (E36300), low (E36100B). **DSL fit: works today** over USBTMC; decode status in CEL.

### 2.18 Hantek PPS2116A / PPS2320A (Circuit Specialists PPS2320A)

Local: `DOC/hantek_pps/doc/hantek_power_supply_communication_protocol.pdf` (2 pp.).

**Models.**
- PPS2116A: 32 V/5 A.
- PPS2320A: 2× 32 V/3 A + CH3 2.5/3.3/5 V; series, parallel and tracking.

**Link.** USB virtual serial at **9600** (chip unknown). Commands end with `\n`; replies probably end with CR LF.

**Protocol.** Lowercase ASCII, implied-decimal integers.
- Identify: `a` → `3203`.
- Setters reply `OK`, or `N` on failure. Always read the reply.

| Op | CH1 | CH2 | Format |
|---|---|---|---|
| Set V | `suXXXX` | `saXXXX` | V×100, `%04d` |
| Set I | `siXXXX` | `sdXXXX` | I×1000 |
| Measured V / I | `rv` / `ra` | `rh` / `rj` | same scales |
| Setpoints | `ru` / `ri` | `rk` / `rq` | |
| State | `rs` | `rp` (CH3: `rb`) | `00` off, `01` CV, `10` CC |

Other commands:
- `o1` / `o0` switch the output.
- `o2`–`o5` select independent, parallel, series or tracking mode.
- `rm` reads the mode; `rl` reads the lock.
- There are no OVP/OCP or remote commands.

**Confidence** medium (three scripts agree). **DSL fit:** blocked by the implied-decimal gap. Framing is fine.

### 2.19 GW Instek GPP-1326 / 2323 / 3323 / 4323

Local: `DOC/gwinstek_gpp/doc/GPP_Series_User_Manual.pdf` (PDF pages).

**Models** (p.199).
- GPP-1326: 32 V/6 A.
- GPP-2323: 2× 32 V/3 A.
- GPP-3323: 2× 32 V/3 A + CH3 1.8–5 V/5 A (fixed steps).
- GPP-4323: 2× 32 V/3 A + 5 V/1 A + 15 V/1 A.
- All models have a load mode on CH1/CH2.

**Link.**
- RS-232 at 115200 8N1 by default (p.100), or USB CDC (p.102).
- LAN raw socket **1026** (p.108). Only one interface is active at a time.
- Commands end with `\n` or `\r\n`; replies end with `\n` (p.114).

**Identify** (p.188). `GW INSTEK,GPP-3323,XXXXXXXXX,V1.00`.

| Op | Command | Page |
|---|---|---|
| Measure | `:MEAS{ch}:VOLT?`, `:MEAS{ch}:CURR?`, `:MEAS{ch}:POWE?`, `:MEAS{ch}:ALL?` | 124–125 |
| Setpoints | `:SOUR{ch}:VOLT` / `?`, `:SOUR{ch}:CURR` / `?` | 132–134 |
| CC state | `:SOUR{ch}:CURR:STAT?` → 1 when the current limit is reached | 133 |
| Output | `:OUTP{ch} ON\|OFF` / `?`; `:ALLOUTON` / `:ALLOUTOFF` | 128–129 |
| OVP | `:OUTP{ch}:OVP <v>`, `:OUTP{ch}:OVP:STAT` | 129–130 |
| OCP | `:OUTP{ch}:OCP <a>`, `:OUTP{ch}:OCP:STAT` | 130–131 |
| Mode | `:MODE{ch}?` | 135 |
| Remote / local | `:SYST:REM` / `:SYST:LOC` | 183–184 |

The GPD legacy commands are also accepted.

**Confidence** medium (no reply examples). **DSL fit: works today** over serial.

### 2.20 OWON P4000 (P4305 30 V/5 A, P4603 60 V/3 A)

Local: `DOC/owon_p4000/doc/P4000_Series_DC_Power_Supply_Programming_Manual.pdf` (11 pp.) and `P4000_Single_Output_Power_Supply_USER_MANUAL.pdf`.

**Link.**
- RS-232 at **115200** by default (user manual p.16–17).
- Optional USBTMC (vid:pid unknown).

**Commands.** The OWON base **minus** `MEAS:ALL?`, `MEAS:ALL:INFO?` and `SYST:REM`/`LOC`. IDN `OWON,P4305,1715040,FV:V1.0.2` (p.6). Same as sigrok's `owon_p4000_cmd`.

There is no CV/CC query, so poll the individual `MEAS:*?` queries and `OUTP?`.
**Confidence** medium. **DSL fit: works today** (extends `owon_psu`).

### 2.21 Low-confidence SCPI families (no manual downloaded)

These come from general knowledge only. Fetch their manuals before writing definitions.

**R&S HMC8041/42/43 and NGE100/NGE102B/103B.**
- Models: HMC8041 1× 32 V/10 A; HMC8042 2× 32 V/5 A; HMC8043 3× 32 V/3 A; NGE102B/103B 2 or 3× 32 V/3 A.
- Link: USB VCP or USBTMC (VID 0aad); LAN 5025.
- Identify: `Rohde&Schwarz,HMC8043,<sn>,<fw>`.
- Channel selection is **stateful**: `INST:NSEL {ch}` (or `INST OUT{ch}`), then plain `VOLT`, `CURR`, `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?`, `VOLT:PROT`, `VOLT:PROT:TRIP?`, `FUSE:STAT` (OCP).
- Output is two-level: `OUTP:CHAN ON` + `OUTP:MAST ON` (HMC), or `OUTP:SEL` + `OUTP:GEN` (NGE).
- DSL fit: works with `before:` steps and a two-step output setter.

**ITECH IT6700/IT6720.**
- Link: USBTMC (VID 2ec7?) or RS-232 at 9600.
- Identify: `ITECH Ltd., IT6720, …`.
- **`SYST:REM` is required** before RS-232 setters.
- Commands: standard `VOLT`, `CURR`, `OUTP`, `MEAS:*?`, `VOLT:PROT`, `STAT:QUES:COND?`.

**B&K 9200 (9201/9202/9205/9206).** SCPI over USBTMC, RS-232 or GPIB.

All three are natural users of the `scpi_psu` base.

---

## 3. Shared baselines and clones

### Clone / OEM map (one definition, many names)

| Protocol owner | Same protocol |
|---|---|
| Korad KA/KD | Tenma 72-2535/2540/2545/2550/2705/2710/2930/2940/133xx, RND 320-KA/KD, Velleman PS3005D/LABPS3005D, Stamos S-LS-31, RS Pro RS-3005P (probable) |
| Manson HCS-3xxx | B&K 1685B/1687B/1688B (+ 1696–1698 with address), Voltcraft PPS-xxxxx/DPPS, Peaktech 1565/1575/1580. B&K 9103/9104 is a dialect |
| OWON SPM/SPE/SP | Kiprim DC310S/DC605S. P4000 = subset. ODP = superset with channels |
| Siglent SPD3303X | RS PRO RSPD3303X/-E/C; Uni-T UDP3303C/3305C use the same syntax (hex `SYST:STAT?`) |
| Rigol DP800 | DP700 (subset); DP900 expected to be the same (unverified) |
| Hantek PPS2320A | Circuit Specialists PPS2320A |
| Riden RD60xx | RuiDeng DPS/DPH (different register map, same Modbus mechanics) |
| GW Instek GPD | GPP accepts the GPD commands as a legacy set |

### Proposed bases (`extends`)

1. **`korad_ka`**: `*IDN?`, `VSET{ch}:%05.2f`, `ISET{ch}:%05.3f`, `VSET{ch}?`, `ISET{ch}?`, `VOUT{ch}?`, `IOUT{ch}?`, `OUT0/1`, `STATUS?` (bits 0/1 CV, bit 6 output), `OCP0/1`, `OVP0/1`.
   - Framing: `tx ""`, `rx idle 15 ms`, `gap_after_write_ms: 50`.
   - Tenma becomes model entries (or `extends`). KA33xx adds channel 2 and TRACK.
2. **`owon_psu`**: the PSU half of `owon_spm.yaml`. `*IDN?`, `MEAS:VOLT?/CURR?/POW?`, `MEAS:ALL:INFO?`, `VOLT`/`VOLT?`, `CURR`/`CURR?`, `VOLT:LIM`/`CURR:LIM` (+`?`), `OUTP ON|OFF`/`OUTP?`, `SYST:REM`/`SYST:LOC`.
   - Users: SPM, SPE, SP, SPS, Kiprim. P4000 drops INFO/REM. ODP adds `INST:NSEL`.
3. **`rigol_dp`**: explicit-channel forms (`:MEAS:ALL? CH{ch}`, `:SOUR{ch}:VOLT`, `:SOUR{ch}:CURR`, `:OUTP CH{ch},ON|OFF`, `:OUTP? CH{ch}`, `:OUTP:CVCC? CH{ch}`, `:OUTP:OVP|OCP[:VAL] CH{ch},…`, `:OUTP:OVP|OCP:QUES? CH{ch}`, `:SYST:REM/LOC`). DP700 and DP800 differ only in link and OTP.
4. **`siglent_spd_legacy`**: SPD3303X/X-E/C and SPD1000X (+ Uni-T UDP3303C). `CH{ch}:VOLT`, `CH{ch}:CURR`, `MEAS:VOLT|CURR|POWE? CH{ch}`, `OUTP CH{ch},ON|OFF`, `SYST:STAT?` (hex), `*LOCK/*UNLOCK`, `\n`, 100–300 ms pacing. Blocked by the hex parse gap.
5. **`scpi_psu`** (common SCPI core: Keysight, Uni-T, Rigol, SPD4000X, GPP, ITECH, R&S, B&K 9200, OWON):

   | Purpose | Command |
   |---|---|
   | Identify | `*IDN?` → `<vendor>,<model>,<serial>,<fw>`. The model regex lives in the derived definition |
   | Measure | `MEAS:VOLT?`, `MEAS:CURR?`, optionally `MEAS:POW?` (spelled `POW`, `POWE` or `POWER` by vendor) |
   | Setpoints | `VOLT <v>` / `VOLT?`, `CURR <i>` / `CURR?` (`[SOUR:]` optional) |
   | Output | `OUTP ON\|OFF` / `OUTP?`. The reply is `ON`/`OFF` or `1`/`0`; parse with `{map: {"1": "ON", "0": "OFF", "ON": "ON", "OFF": "OFF"}}` |
   | Protection levels | `VOLT:PROT <v>` / `?`, `CURR:PROT <i>` / `?` where present. OWON uses `VOLT:LIM`/`CURR:LIM` instead |
   | Remote / local | `SYST:REM` / `SYST:LOC` (session `on_connect` / `on_disconnect` where required) |
   | Errors | `SYST:ERR?` |

   - Framing: `tx "\n"`, `rx message` (USBTMC) or `rx terminator "\n"` with `strip_trailing "\r"` (serial).
   - The base only fits **single-channel** devices. Channel syntax diverges per vendor:

     | Vendor | Channel syntax |
     |---|---|
     | Rigol | `CH{n}` argument or `SOUR{n}` |
     | Uni-T UDP3305S | `SOUR{n}` / `CH{n}` argument |
     | Siglent legacy | `CH{n}:` prefix |
     | SPD4000X | `CH{n},` as the first argument |
     | Keysight | `(@n)` or `INST:NSEL` |
     | R&S, OWON ODP | stateful `INST:NSEL` |
     | GPP | `MEAS{n}` / `SOUR{n}` / `OUTP{n}` suffix |

     Multi-channel definitions override every channelled command.
   - CV/CC and trip state are vendor-specific: `OUTP:CVCC?` (Rigol, Uni-T), `MEAS:RUN:MODE?` (SPD4000X), `STAT:QUES:INST:ISUM<n>:COND?` (Keysight, Uni-T latched), `MEAS:ALL:INFO?` (OWON).
6. **`gwinstek_gpd`**: Korad-like command names, but with line terminators, `V`/`A` unit suffixes and an ASCII `STATUS?`. It is not a `korad_ka` extension, because the framing differs.

---

## 4. DSL features that block or hamper families

| # | Feature | Unblocks | Size |
|---|---|---|---|
| 1 | **Modbus RTU** framing (address, CRC16, fc 03/06/16, exception replies), register decode (u16/u32), numeric identify | Riden RD60xx (#2 overall), RuiDeng DPS/DPH, Juntek Modbus mode, Uni-T UDP6700 Modbus, OWON SPE500 | large |
| 2 | **Implied-decimal integers**: `params.<p>.scale` (wire = round(v × scale), `%03d`), a `{scale: k}` parse step, and per-model param overrides | Manson HCS / B&K 168xB / Voltcraft / Peaktech, B&K 9103/9104, Juntek ASCII, Hantek PPS, and the Riden scaling | small |
| 3 | **Integer parse** steps: `hex` / `{int: {base: 16}}`, and `bits` accepting an integral `number` (today `[number, bits]` fails) | Siglent SPD3303X/X-E/C, SPD1000X, Uni-T UDP3303C (required OUTPUT status); cleaner status for Keysight, Uni-T and TTi decimal registers | small |
| 4 | String-to-characters parse (or enable the CEL `regex` feature for `matches()`) | GW Instek GPD `STATUS?` (CEL arithmetic works meanwhile) | small |
| 5 | `serial.flow: dsr_dtr`, plus confirming `8N2` | Keysight/HP E3631A family over RS-232 | small |
| 6 | **TCP** link (declared, not implemented); UDP for Korad KWR | LAN-only use of Rigol, Siglent, Uni-T, Keysight, GPP (1026), TTi (9221), R&S | medium. No family is blocked: all except Korad KWR LAN have USB or serial |
| 7 | Optional class extensions: `range` (Rigol DP811, TTi MX, E3632A, RD6012P) and `tracking` (KA33xx, SPD3303X, DP832, GPD, Hantek) | richer panels | contract change |

Per-channel output on the OWON ODP (list-only `CHAN:OUTP:ALL a,b,c`) needs a computed `from` param. That is possible today but awkward.

## 5. Next steps

1. Write the definitions that work today, in this order:
   - `korad_ka` (with Tenma extending it)
   - `owon_psu` / `owon_spe`
   - `rigol_dp`
   - `unit_udp3000s`
   - `gwinstek_gpd`
   - `tti_cpx`
   - `siglent_spd4000x`
   - `unit_udp6700`
   - `owon_odp`
   - `owon_p4000`
   - `keysight_e36300`
2. Add DSL gaps #2 and #3 (small). They unblock Manson/B&K, Juntek, Hantek and the Siglent SPD3303X/SPD1000X.
3. Modbus (#1) is the big step for Riden.
4. Fetch the missing manuals (Keysight, R&S, ITECH, B&K 9200, Rigol DP900) before writing their definitions.
