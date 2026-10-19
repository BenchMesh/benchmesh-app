# AWG coverage: function / arbitrary waveform generators

Research 2026-10-18. Class contract: `devices/classes/AWG.yaml`. Reference definition: `devices/owon_dge.yaml`
(Bench-tested, DGE2070). Everything else in this report is **Awaiting confirmation**.

**Conventions**
- Doc root `D = /home/marek/project/BenchMeshDOC`. Each `D/<folder>/doc/` has a `SOURCES.md` (URL, date, title).
- `pN` = **PDF page** (the page number in the file) unless marked "printed". Offsets are given where known.
- **Confidence** = how sure we are that a definition written from this report will work on first contact.
- **DSL fit**:
  - ✅ = works with today's DSL.
  - ⚠️ = works with a workaround, or with one operation left unbound.
  - ⛔ = needs a DSL feature (listed in §5).

**API values the class binding should use** (bench UI `kit/WavePreview.tsx` draws `SIN SQU RAMP PUL PRN`):
- `SHAPE` / `shape`: `SIN SQU RAMP PUL PRN` plus `DC` and `ARB` where the instrument has them. The wire spelling is mapped in the enum params and in the query `map`, as `owon_dge.yaml` does (`PULS → PUL`).
- `PHASE` in **degrees**. Instruments that answer in radians (Tek-AFG dialect) are converted in CEL.
- `LOAD`: `"50"` … `"10000"` (ohms as text) or `"HIGHZ"`. `9.9E+37`, `INF`, `INFINITY`, `Infinity` and `HZ` all mean `HIGHZ`.
- `OUTPUT`: `ON`/`OFF`. Output setters are `safety: energize` (two presses in the UI).
- Not in the contract yet but common to every SCPI family: square **duty**, **load** as a setter, and amplitude **unit**. See the §4 proposal.

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Price band | Protocol / link | Doc quality | Confidence | DSL fit |
|---|---|---|---|---|---|---|
| 1 | **Rigol DG1000Z** (DG1022Z/1032Z/1062Z), **DG800/DG900**, DG2000, DG4000 | €250-1500 | SCPI `:SOURn:` + `APPLy?`, USBTMC (+LAN) | excellent | high (DG4000 medium) | ✅ |
| 2 | **Siglent SDG1000X / SDG2000X** (+ X Plus, SDG6000X), older SDG800/SDG1000 | €300-3000 | Siglent `C1:BSWV` key,value, USBTMC (+LAN) | good | high (X) / medium (old) | ✅ (6 exchanges/poll), ✅✅ with `pairs` |
| 3 | **FeelTech/FeelElec FY6900 / FY6800 / FY6600 / FY2300** | €60-150 | proprietary ASCII `WMF…`/`RMF`, CH340 serial 115200 | poor (vendor PDF wrong) + good OSS | medium | FY6900 ⚠️, FY6600/6800 ⛔ (frequency in µHz) |
| 4 | **JDS6600** (Juntek / Koolertron / Joy-IT), JDS2915 | €70-120 | proprietary ASCII `:w21=…`, CH340 serial 115200 | medium + sigrok | high (JDS2915 low) | ⛔ (value scaling, shared output register) |
| 5 | **UNI-T UTG900E / UTG1000X** (+ UTG2000B, UTG4000A) | €150-1500 | UNI-T SCPI `:CHANn:BASE:*`, USBTMC | medium | medium | ✅ |
| 6 | **OWON XDG / DGE** + **Tektronix AFG1000** (OWON OEM) | €200-700 | Tek-AFG SCPI `SOURn:FUNC:SHAP`, USBTMC | good | high | ✅ (DGE is Bench-tested) |
| 7 | **Keysight EDU33210, 33500B/33600A, 33210A/33220A** (used market) | €600-5000 (33220A used €300) | Keysight SCPI + `APPLy?`, USBTMC/LAN/GPIB | excellent (33500B guide needs a login) | high (33500B medium) | ✅ |
| 8 | **GW Instek AFG-2000/2100, AFG-2225**, AFG-3000 | €250-1000 | Keysight-33120-style SCPI, **USB CDC serial** (AFG-3000: RS-232) | good | high | ✅ |
| 9 | **Rigol DG1022 / DG1022A** (old, non-Z; very common used) | used €100-200 | old Rigol SCPI with `:CH2` suffix, USBTMC 0400:09C4 | medium | medium | ✅ via computed channel suffix |
| 10 | **BK Precision 4050B / 4060B** (Siglent dialect) | €500-1500 | Siglent `C1:BSWV`, USBTMC | good | high | ✅ (shares the Siglent base) |
| 11 | **Hantek HDG2000B** | €250-400 | Keysight-33500-like SCPI, USBTMC 049F:505B | medium (typos) | medium | ✅ (no load control over SCPI) |
| 12 | **FeelTech FY3200S / FY3224S** (old) | €40 | lowercase ASCII `bf…`, 9600 | none official | low | ⛔ (no output command, no CH2 read-back) |
| - | OWON AG1022 / AG-F / AG-S | €150 | OWON-AG SCPI, stateful `:CHAN CH1`, replies to every command; USB uses OWON's own driver | medium | low (link) | ⚠️ RS-232 only |
| - | Tektronix AFG31000 | €3000+ | Tek AFG3000 command set (not verified) | **not obtained** | - | - |
| - | Fnirsi / handheld scope+AWG combos, JDS2800, JDS8000 | - | no documented remote protocol found (search budget ran out) | - | - | - |

---

## 2. Families in depth

### 2.1 Rigol DG800 / DG900 / DG1000Z / DG2000 (one dialect), DG4000 (near-identical)

**Docs**
| Path | Doc |
|---|---|
| `D/rigol_dg1000z/doc/DG1000Z_ProgrammingGuide_EN.pdf` | PGB09106-1110, 264 pp; printed 2-n = PDF − 12 |
| `D/rigol_dg800/doc/DG800_ProgrammingGuide_EN.pdf` | PGB11100, printed = PDF − 14 |
| `D/rigol_dg800/doc/DG900_ProgrammingGuide_EN.pdf` | PGB10100, printed = PDF − 12 |
| `D/rigol_dg2000/doc/DG2000_ProgrammingGuide_EN.pdf` | PGB12100 |
| `D/rigol_dg4000/doc/DG4000_ProgrammingGuide_EN.pdf` | PGB04108, mirror copy, 646 small pages |

Open source: QCoDeS `Rigol_DG1062.py` / `Rigol_DG4000.py`, sigrok `rigol-dg`.

**Models / features** (Table 2-1: DG800 p64, DG900 p59, DG2000 p60, DG1000Z p79). Frequency resolution is 1 µHz.

| Model | Ch | Sine | Square | Pulse | Ramp |
|---|---|---|---|---|---|
| DG811 / DG812 | 1 / 2 | 10 MHz | 5 MHz | 5 MHz | 200 kHz |
| DG821 / DG822 | 1 / 2 | 25 MHz | 10 MHz | 10 MHz | 500 kHz |
| DG831 / DG832 | 1 / 2 | 35 MHz | 10 MHz | 10 MHz | 1 MHz |
| DG952 / DG2052 | 2 | 50 MHz | 15 MHz | 15 MHz | 1.5 MHz |
| DG972 / DG2072 | 2 | 70 MHz | 20 MHz | 20 MHz | 1.5 MHz |
| DG992 / DG2102 | 2 | 100 MHz | 25 MHz | 25 MHz | 2 MHz |
| DG1022Z | 2 | 25 MHz | 25 MHz | 15 MHz | 500 kHz |
| DG1032Z | 2 | 30 MHz | 25 MHz | 15 MHz | 500 kHz |
| DG1062Z | 2 | 60 MHz | 25 MHz | 25 MHz | 1 MHz |
| DG4062 / 4102 / 4162 / 4202 | 2 | 60 / 100 / 160 / 200 MHz | 25 / 40 / 50 / 60 MHz | - | - |

- **Amplitude into 50 Ω** (High-Z is ×2): 1 mVpp-10 Vpp up to 10 MHz, 5 Vpp up to 30 MHz, 2.5 Vpp up to 60 MHz, DG900 1 Vpp above 60 MHz.
  - DG4000: 10 Vpp ≤20 MHz, 5 Vpp ≤70 MHz.
  - The programming guides give a 2 mVpp minimum (DG1000Z p194).
- **Offset:** ±5 V (50 Ω) / ±10 V (HiZ), bounded by \|offs\| + Vpp/2.
- **Out-of-range values are silently clamped** (DG800 p84, p152), so always read back.
- Recommended `features`: `{max_freq_hz, channels}` per model. Setter `max` = the sine limit at HiZ (20 Vpp, ±10 V). The instrument clamps the rest.

**Interfaces**
| Family | USBTMC | LAN |
|---|---|---|
| DG1000Z | **1AB1:0642** (VISA string DG1000Z p209, p228); high confidence | built in, raw SCPI port **5555** (`:SYST:COMM:LAN:CONT?` → 5555, DG1000Z p203) and VXI-11. **TCP is not implemented in BenchMesh** |
| DG800 | **1AB1:0643** (real DG811, pyvisa issue #521; the guide's 0642 is copy-paste); high confidence | USB-LAN adapter only |
| DG900 | probably 0643 (verify) | USB-LAN adapter only |
| DG2000 | unknown (verify) | built in |
| DG4000 | 0640 or 0641 (sources conflict; verify) | built in |

No RS-232 for control. The DB9 "RS232" on DG800/900/2000 is a waveform output.

**Identify:** `*IDN?` → `Rigol Technologies,DG1062Z,DG1ZA000000001,00.01.03` (DG1000Z p42).
- Other examples: `Rigol Technologies,DG832,DG80000000001,00.01.05.00.03` (DG800 p37), real DG811 `Rigol Technologies,DG811,DG8A220800316,00.02.04.00.00`.
- Match `^Rigol Technologies,DG(8[123][12]|9[579]2|20[579]2|2102|10[236]2Z|4(06|10|16|20)2),`.

**Command table** (`<n>` = 1 | 2. Pages DG1000Z / DG800. All setters silent. Queries return `%E` with 7 significant digits.)

| Operation | Set | Query → reply | Pages |
|---|---|---|---|
| Output | `:OUTP<n> ON\|OFF` (also `:OUTP<n>:STAT`) | `:OUTP<n>?` → `ON` / `OFF` (**text**) | p70 / p58-59 |
| Load | `:OUTP<n>:LOAD <1..10000>\|INF\|MIN\|MAX` (alias `:IMP`); default 50 Ω | `5.000000E+01`; High-Z → `9.900000E+37` | p68 / p57-58 |
| Polarity | `:OUTP<n>:POL NORM\|INV` | DG1000Z `NORMAL`/`INVERTED`, DG800 `NORM`/`INV` | p69 / p58 |
| Shape | `:SOUR<n>:FUNC SIN\|SQU\|RAMP\|PULS\|NOIS\|DC\|USER\|HARM` (+ ~150 built-in arb names) | `SIN`, `SQU`, … (an arb returns its own name) | p121 / p101 |
| Frequency | `:SOUR<n>:FREQ <Hz>` | `1.000000E+03` | p109 / p84 |
| Amplitude | `:SOUR<n>:VOLT <v>` (current unit, default Vpp) | `5.000000E+00` | p194 / p152 |
| Amplitude unit | `:SOUR<n>:VOLT:UNIT VPP\|VRMS\|DBM` (no dBm at HiZ) | `VPP` | p199 / p155 |
| Offset | `:SOUR<n>:VOLT:OFFS <V>` | `1.000000E+00` | p195 / p154 |
| High / low level | `:SOUR<n>:VOLT:HIGH` / `:LOW` | `%E` | - / p153 |
| Phase | `:SOUR<n>:PHAS <0..360>` (degrees) | `1.000000E+01` | p167 / p137 |
| Square duty | `:SOUR<n>:FUNC:SQU:DCYC <pct>` (limit depends on frequency) | `%E` | p122 / p102 |
| Ramp symmetry | `:SOUR<n>:FUNC:RAMP:SYMM <0..100>` | `%E` | p120 / p95 |
| Pulse duty | `:SOUR<n>:FUNC:PULS:DCYC` | `%E` | - / p91 |
| Apply (one shot) | `:SOUR<n>:APPL:SIN f,amp,offs,phase` (SQU, RAMP, PULS, USER, HARM have 4 args; `NOIS amp,offs`) | - | p85-93 / p66-72 |
| **Complete state** | - | `:SOUR<n>:APPL?` → `"SQU,1.000000E+03,2.000000E+00,3.000000E+00,4.000000E+00"` | p84 / p65-66 |
| Errors | - | `:SYST:ERR?` → `-113,"Undefined header; …"` | p211 / p164 |

**`APPL?` parsing**
- Fields: shape, Hz, amplitude (current unit), offset V, phase °. Quotes are part of the reply. A field that does not apply is the literal `DEF`: `"NOISE,DEF,5.000000E+00,0.000000E+00,DEF"` (DG4000 p281).
- **The shape names differ from the `FUNC` keywords:** `SIN SQU RAMP PULSE NOISE DC USER` (DG4000 also has `HARMONIC`, `CUSTOM`).
- DSL: `parse: [trim, unquote, {fields: {sep: ",", names: [shape, freq, ampl, offs, phase]}}]`, then status CEL:
  - `num(appl_q.freq)` (null for `DEF`)
  - `{'SIN':'SIN','SQU':'SQU','RAMP':'RAMP','PULSE':'PUL','NOISE':'PRN','DC':'DC','USER':'ARB'}[appl_q.shape]`
- One exchange per channel gives SHAPE, FREQ, AMPL, OFFS and PHASE. SYMM needs a separate `FUNC:RAMP:SYMM?` and LOAD a separate `OUTP:LOAD?`.
- **Amplitude caveat:** `APPL?` reports amplitude in the current `VOLT:UNIT`. Either set `VOLT:UNIT VPP` in `session.on_connect` (a configure step, acceptable) or query `VOLT:UNIT?` and refuse otherwise.

**DG4000 differences** (DG4000 guide)
- `[:SOURce<n>]` is optional (p199).
- `:OUTP<n>:IMP?` answers the **text `INFINITY`** for HiZ (p199-200). The documented default is High-Z.
- Square duty is limited to 20-80 % ≤10 MHz, 40-60 % ≤40 MHz, fixed 50 % above (p344).
- IDN `Rigol Technologies,DG4162,DG41620000,00.01.02` (p147).

**Quirks**
- Terminator `\n` on both tx and rx (the guide never states it; QCoDeS and pyvisa use `\n`).
- Vendor examples sleep 200 ms before reading `*IDN?` (DG1000Z p248). sigrok syncs with `*OPC?`.
- DG800/900: after `COUN:STAT ON` the instrument is deaf for about 1 s, and on 2-channel models CH2 conflicts with the counter (sigrok). Never touch the counter.
- DG811 firmware 1.08 had USB read failures on Linux; 2.06 fixed them (pyvisa #521).
- The first command enters remote; **Help** returns to local (DG1000Z p38). No `on_connect` is needed.

**Framing:** `link.usbtmc {vid: 1AB1, pid: 0642|0643}`, `tx "\n"`, `rx message` with `strip_trailing "\n"`, `timeout_ms 1000`, `gap_after_write_ms 20`. Status poll: `outp_q` every 1 s; `appl_q`, `symm_q` and `load_q` every 5 s.

**Confidence / fit:** **high**; ✅. The only unknowns are the DG900/DG2000/DG4000 PIDs. Because `link.usbtmc` needs a vid:pid, one definition per PID is needed (or a `pid` list, see §5).

### 2.2 Siglent SDG (SDG1000X, SDG1000X Plus, SDG2000X, SDG6000X; old SDG800/SDG1000) + BK Precision 4050B/4060B

**Docs**
| Path | Doc |
|---|---|
| `D/siglent_sdg/doc/SDG_Programming_Guide_PG02-E05C.pdf` | Generic guide for all series, with a per-series availability table under each command; printed = PDF − 12 |
| `D/siglent_sdg/doc/SDG1000X_Plus_ProgrammingGuide_EN01B.pdf`, `SDG6000X_ProgrammingGuide_EN06A.pdf` | Series guides |
| `D/siglent_sdg/doc/ds_SDG*.pdf` | Datasheets |
| `D/bk_4050/doc/4050B_Series_programming_manual.pdf`, `4060B_Series_programming_manual.pdf` | BK Precision |

Open source: QCoDeS contrib `sdg_channel.py` / `_sdg_response_fields.py`.

**Models / features** (datasheets; HiZ values, halve for 50 Ω)
| Model | Ch | Sine | Square | Amplitude (HiZ) | Offset |
|---|---|---|---|---|---|
| SDG805 / 810 / 830 | 1 | 5 / 10 / 30 MHz | - | 4 mVpp-20 Vpp (≤10 MHz), 10 Vpp above | ±10 V |
| SDG1010 / 1025 / 1050 | 2 | 10 / 25 / 50 MHz | - | CH1 as SDG800; **CH2 ≤6 Vpp HiZ** | CH2 ±3 V |
| SDG1032X / 1062X | 2 | 30 / 60 MHz | 30 / 60 MHz | 2 mVpp-20 Vpp (≤10 MHz), 10 Vpp above | DC ±10 V |
| SDG1022X / 1032X / 1062X Plus | 2 | 25 / 30 / 60 MHz | 20 MHz | 2 mVpp-20 Vpp (≤20 MHz), 10 Vpp above | DC ±10 V |
| SDG2042X / 2082X / 2122X | 2 | 40 / 80 / 120 MHz | 25 MHz | 2 mVpp-20 Vpp (≤20 MHz), 10 Vpp above | DC ±10 V |
| SDG6022X / 6032X / 6052X | 2 | 200 / 350 / 500 MHz | - | 20 Vpp ≤40 MHz | DC ±10 V |
| BK 4053B / 4054B / 4055B | 2 | 10 / 30 / 60 MHz | - | - | ±5 V @50 Ω |
| BK 4062B / 4063B / 4064B | 2 | 40 / 80 / 120 MHz | 25 MHz | - | - |

On all X models, \|offs\| + Vpp/2 ≤ 10 V at HiZ.

**Interfaces**
- USBTMC, vendor **F4EC**. The guide's VISA example is `USB0::0xF4EC::0x1101::…` (p175 printed / PDF 187).
- Per-model PIDs are **not documented**: 0x1101 is printed, 0x1102 is reported for some X models. Verify with `cargo run -p bm-transport --example ports`.
- The old SDG1000 enumerates as **F4ED:EE3A** in TMC mode (F4ED:EE37 is the non-TMC mode, `usb.ids`).
- BK 4050B/4060B: USBTMC, VID not documented (probably F4EC; verify).
- LAN: raw socket **5025**, telnet 5024, VXI-11 (PDF 19). Not usable today (no TCP).

**Identify** (PDF 24-25)
- X series: `Siglent Technologies,SDG1062X Plus,SDG1XPLUS00041,V1P.1.1.1.40`. The model field **can contain a space**, and one example has a space after a comma (`, SDG6XBAX1R0034`), so trim.
- Old SDG800/1000/5000: `*IDN SDG,SDG1025,<serial>,<fw>,<hw>` (header prefix, manufacturer `SDG`).
- BK: `*IDN BK Precision,4050B,…` (4050B p8) or `BK,4063B,…,2.01.01.35R3B2` (4060B p7).
- Match patterns:
  - `^Siglent Technologies,SDG(1032X|1062X|2042X|2082X|2122X|60[235]2X)`, plus a separate ` Plus` model entry
  - `^(\*IDN )?SDG,SDG(80[5-9]|81[0-9]|830|10(10|25|50))`
  - `^(\*IDN )?(BK Precision|BK),40(5[345]|6[234])B,`

**Command headers** (PDF 26)
- X series and BK 4060B **always** prefix replies with a header (`C1:BSWV …`).
- `CHDR SHORT|LONG|OFF` exists only on SDG800/SDG1000/SDG5000 (and the BK 4050B, p12). There `CHDR OFF` removes the header **and** the unit suffixes.

**Command table** (generic guide, printed pages; PDF = +12)

| Operation | Set | Query → reply | Pages |
|---|---|---|---|
| Output | `C<n>:OUTP ON\|OFF` | `C<n>:OUTP?` → `C1:OUTP ON,LOAD,HZ,PLRT,NOR` | p15-16 |
| Load | `C<n>:OUTP LOAD,50\|HZ\|<ohms>` (SDG800: 50 or HZ; SDG1000: 50-10000; X: 50-100000) | same reply, `LOAD,<v>` pair | p15-16 |
| Polarity | `C<n>:OUTP PLRT,NOR\|INVT` | `PLRT,<v>` pair | p15-16 |
| Shape | `C<n>:BSWV WVTP,SINE\|SQUARE\|RAMP\|PULSE\|NOISE\|ARB\|DC` (+PRBS, IQ on 6000X) | `WVTP,<v>` in BSWV? | p17-20 |
| Frequency | `C<n>:BSWV FRQ,<Hz>` (plain number, `100E6` allowed) | `FRQ,100HZ` | p17-20 |
| Amplitude | `C<n>:BSWV AMP,<Vpp>` (also AMPVRMS, AMPDBM) | `AMP,2V` | p17-20 |
| Offset | `C<n>:BSWV OFST,<V>` | `OFST,0V` | p17-20 |
| Phase | `C<n>:BSWV PHSE,<0..360>` (not NOISE, PULSE or DC) | `PHSE,0` (no unit) | p17-20 |
| Duty | `C<n>:BSWV DUTY,<pct>` (SQUARE / PULSE) | `DUTY,50` (only when relevant) | p17-20 |
| Symmetry | `C<n>:BSWV SYM,<pct>` (RAMP) | `SYM,50` (only when relevant) | p17-20 |
| High / low level | `C<n>:BSWV HLEV,<V>` / `LLEV,<V>` | `HLEV,1V`, `LLEV,-1V` | p17-20 |
| **Complete state** | - | `C<n>:BSWV?` → `C1:BSWV WVTP,SINE,FRQ,100HZ,PERI,0.01S,AMP,2V,OFST,0V,HLEV,1V,LLEV,-1V,PHSE,0` (+ `AMPVRMS,…`, `MAX_OUTPUT_AMP,…`; SQUARE adds DUTY, RAMP adds SYM, PULSE adds WIDTH/RISE/FALL/DLY) | p17-20 |
| Both outputs | `OUT_BOTHCH ON\|OFF` (not on SDG800) | - | - |
| Sync | `*OPC?` → `1` (X) / `*OPC 1` (old) | - | p13-14 |

- SDG800 is single-channel and takes **no `C1:` prefix** (availability table). That needs a separate definition.
- BK: `C1:OUTP …` and `C1:BSWV …` are identical (4050B p13-15, 4060B p9-11).

**Parsing (today's DSL)**
- `OUTP?` and `BSWV?` replies are `key,value` pairs whose **order and set vary** by series and waveform. The 1000X Plus and 6000X insert `POWERON_STATE,OFF` into `OUTP?`. **Parse by key, never by position.**
- Each status key gets its own command re-sending `C{ch}:BSWV?` with `parse: [{regex: {pattern: "[ ,]FRQ,([-0-9.eE+]+)", group: 1}}, number]`, and likewise for `AMP`, `OFST`, `PHSE`, `SYM`, `DUTY`.
- `WVTP` uses regex `WVTP,([A-Z]+)`, then a map `SINE→SIN, SQUARE→SQU, RAMP→RAMP, PULSE→PUL, NOISE→PRN, DC→DC, ARB→ARB`.
- `OUTP?` → regex `OUTP (ON|OFF)`; `LOAD` → regex `LOAD,([0-9]+|HZ)`, then map `HZ → HIGHZ`.
- The regex anchors on `[ ,]` so `FRQ` does not match inside other keys. **Units are glued** (`100HZ`, `0.01S`, `2V`); the numeric capture group stops at the unit.
- Values are plain decimals with optional exponent.
- This costs about 6 exchanges per channel every 5 s, which is acceptable. A `pairs` parse step (§5) would cut it to one.
- `SYM` / `DUTY` are absent for other waveforms. Use `on_empty`-style null handling: needs "regex no-match → null" (§5), or bind them through `requires: shape == …`.

**Quirks**
- Setters are silent. Terminator `\n` both ways ("SCPI command strings must be terminated with "\n"", p182 printed).
- The instrument parses a command only after the previous one has executed (p13). Use `gap_after_write_ms` ≈ 50 and confirm by read-back.
- All commands work in local and remote (p8). No remote/local handling is needed.
- On SDG1000X, `WVTP SQUARE` is refused while Wave Combine is on (PDF 32).
- Changing LOAD rescales the displayed amplitude. Re-poll after a load set: a set already makes all groups due.

**Confidence / fit:** X series **high** (syntax, IDN, OUTP and BSWV are well documented; exact field sets are medium). Old SDG800/1000 medium. BK high. ✅ today.

### 2.3 FeelTech / FeelElec FY6900, FY6800, FY6600, FY2300 (proprietary ASCII)

**Docs**
| Path | Doc |
|---|---|
| `D/feeltech_fy6900/doc/FY6900_Serial_communication_protocol.pdf` | Rev 1.8; reported incomplete and partly wrong |
| `D/feeltech_fy6900/doc/FY6900_Users_Manual.pdf` | User manual |
| `D/feeltech_fy6600/doc/FY6600_Serial_Communication_Protocol_V1.5.pdf` | Protocol V1.5 |
| `D/feeltech_fy6800/doc/FY6800_Manual.pdf` | No protocol section |
| `D/feeltech_fy2300/doc/FY2300_Serial_communication_protocol.pdf` | Protocol Rev 1.2 |

**The tested open-source drivers are the authority wherever they disagree with the PDFs:**
- `github.com/mattwach/fygen` (fygen.py:276-365, 557-595, 1217-1336; wavedef.py:148-194)
- `gitlab.com/adeuring/fy6x00` (fy6x00.py:50-186, 1013-1060, README "firmware bugs")
- `github.com/katzenjens/fy6900commander` protocol.md
- EEVblog FY6900 "Gotchas" post msg5542089

**Models:** FY6900-20…100M, FY6800-…60M, FY6600-15…60M, FY2300 (all 2 channels).
- Amplitude ≤20 Vpp, 1 mV resolution (≤5 Vpp above 20 MHz).
- Offset: FY6900 ±12 V; FY6800 ±10 V below 20 MHz, ±2.5 V above.
- Duty 0.01-99.99 %, phase 0.01° steps (FY6900 manual p5).
- The `UMO` reply suffix gives the frequency limit: `^FY\d{4}-(\d+)M`.
- FY8300 (3 channels) has a different command set and is out of scope.

**Interface**
- CH340 USB-serial (1a86:7523). One report shows a CP2102 (10c4:ea60).
- **115200 8N2**: fygen says 2 stop bits are "important for FY6900", and 8N2 is harmless on 8N1 receivers.
- The FY2300 PDF says 9600 but a real FY2300 V2.3 ran at 115200, so make the baud configurable.
- No flow control and no DTR/RTS handshake.
- Commands end with **LF only**; CR confuses the parser.

**Identify:** `UMO` → `FY6900-60M`; `UVE` → `V2.3.2`; `UID` → id number (format not documented).
- Use `identify: umo`, models `^FY6900-`, `^FY6800-`, `^FY6600-`, `^FY2300`.

**Command table** (CH1 = `WM*` / `RM*`, CH2 = `WF*` / `RF*`)

| Operation | Set | Query → reply → value |
|---|---|---|
| Output | `WMN1` / `WMN0` | `RMN` → `0` / `255` (map; fygen treats any nonzero as on). `WMN1` also stops modulation/sweep |
| Shape | `WMW<idx>` (2 digits) | `RMW` → integer, maybe zero-padded |
| Frequency, **FY6900 / FY6300** | `WMF<Hz with 6 decimals>`, e.g. `WMF1000.000000` (fygen pads `%015.6f`; FW 1.8 accepts unpadded) | `RMF` → `59000000.000000` (Hz) |
| Frequency, **FY6600 / FY6800 / FY2300** | `WMF<µHz, 14-digit integer>`, e.g. `WMF00001000000000` = 1 kHz | `RMF` → `00010000.000000` (Hz) |
| Amplitude | `WMA<V %.4f>` | `RMA` → integer ÷ 10000 = V |
| Offset | `WMO<V %.3f>` (negative allowed) | `RMO` → integer mV; **negatives come back as 32-bit unsigned** (`4294963480` = −3.816 V) |
| Duty | `WMD<% %.3f>` | `RMD` → ÷ 1000 = % |
| Phase | `WMP<° %.3f>` | `RMP` → ÷ 1000 = ° |
| CH2→CH1 tracking | `USA0..4` / `USD0..4` | `RSA0..4` |

**Waveform index tables**
- Wire values; API mapping in brackets. They **differ per model, per channel and per firmware**.
- FY6900 CH1: 0 sine [SIN], 1 square [SQU], 2 rectangle, 3 trapezoid, 4 CMOS, 5 adj-pulse [PUL], 6 DC [DC], 7 triangle, 8 ramp [RAMP], 9 neg-ramp, …, 27 random, 32 AWGN [PRN], 35 chirp, 36+ arb (firmware-dependent; FW 1.8 shifts arb by +1/+2).
- FY6900 CH2: identical minus adj-pulse, so everything ≥5 is −1 (5 DC, 6 tri, 7 ramp, 31 AWGN). **CH2 has no adjustable pulse.**
- FY6600 / FY6800 / FY2300 (measured; the vendor PDFs are wrong): CH1 0 sine, 1 square, 2 CMOS, 3 adj-pulse, 4 DC, 5 triangle, 6 ramp, …, 30 AWGN, 34+ arb. CH2 drops adj-pulse, so 3 DC, 4 tri, 5 ramp, 29 AWGN.
- Only the built-in waves (CH1 0-35 / CH2 0-34 on FY6900) are stable across firmware.

**Framing behaviour**
- **Every setter answers a lone `\n`.** Queries sometimes return 1-2 empty lines before the data line.
- Today's DSL handles both with `rx: {mode: terminator, terminator: "\n", skip_leading: "\r\n", drain_idle_ms: 50}` and setters `reply: none` + `gap_after_write_ms: 100`. The stray `\n` ack is then consumed as leading noise of the next reply: `Framer` skips leading chars per exchange, and the engine never flushes input before a write.
- Settings take 0.3-1 s to apply (waveform change). Confirm by read-back with a 6 s window, which the UI already does.
- fygen sends `\n\n\n` at open to resync. Do not auto-retry setters.

**Channel-dependent wire text works today with a computed param:**
`send: "W{p}F{hz}"`, `params: {p: {type: string, from: "ch == 1 ? 'M' : 'F'"}, hz: {type: number, format: "%.6f"}}`.
`from` CEL sees `ch` (validator `query_vars`).

**Quirks**
- Frequency read-back is wrong in the last ~5 digits (wraps modulo 65536 µHz).
- Offset and phase can be ±1 count off.
- Duty only acts on rectangle / CMOS.
- Setting a non-modulation parameter leaves modulation mode.
- Arb upload (`DDS_WAVE` + binary) is out of scope.

**Status CEL**
- `OFFS: "rmo > 2147483648.0 ? (rmo - 4294967296.0) / 1000.0 : rmo / 1000.0"`
- `AMPL: "rma / 10000.0"`

**DSL fit**
- **FY6900 ⚠️**: frequency (`%.6f` Hz), amplitude (`%.4f` V), offset (`%.3f` V), duty and phase all work today, because their values are plain V / % / °.
  - **Waveform select is the problem.** The same API value has a different wire index on CH1 and CH2 (DC 6/5, RAMP 8/7, AWGN 32/31). A `from` expression could pick the index per channel, but it cannot see the caller's value, so this needs §5 F1.
  - Until then, bind `shape` with only `SIN` (0) and `SQU` (1), which are identical on both channels.
- **FY6600 / FY6800 / FY2300 ⛔**: frequency must be sent in µHz (Hz × 10⁶, integer). Needs §5 F1 (caller value in `from` CEL) or a param `scale`.

**Confidence:** command set, terminators and acks **high** (2 PDFs + 3 implementations). Scaling medium. FY6900 arb indices low.

### 2.4 JDS6600 (Juntek / Koolertron / Joy-IT), JDS2915

**Docs**
| Path | Doc |
|---|---|
| `D/juntek_jds6600/doc/JT-JDS6600-Communication-protocol_2024-04-23.pdf` | Joy-IT protocol PDF (no reply formats) |
| `D/juntek_jds6600/doc/JT-JDS6600_Manual-EN_2025-01-16.pdf` | Manual |
| `D/juntek_jds6600/doc/JT-JDS6600_Datasheet-EN_2025-09-22.pdf` | Datasheet |
| `D/joyit_jds2900/doc/JT-JDS29XX_Manual-EN_2025-01-16.pdf`, `JT-JDS2915_Datasheet_2023-09-13.pdf` | JDS29xx |

Open source: sigrok `src/hardware/juntek-jds6600/protocol.c` (20-130, 588-800, 1504-1545), `github.com/on1arf/jds6600_python`.

**Models:** JDS6600-15/20/30/40/50/60 (`:r00=` → max MHz), 2 channels.
- Amplitude 2 mVpp-20 Vpp below 11 MHz, ≤5 Vpp above 31 MHz.
- Offset ±9.99 V. Phase 0-359.9°.
- JDS2915: 15 MHz. Joy-IT points to the same protocol PDF; not verified.

**Interface:** CH340G USB CDC, **115200 8N1**, no handshake.
- Request: `:` + `r|w` + 2-digit register + `=` + comma-separated values + `.` + `\r\n` (LF alone also works).
- Replies end with `\r\n`: reads `:r21=103.`, **writes `:ok`**.
- The firmware answers within ~2 ms. Send each line in a single write: typed-by-hand lines are acked but not executed.

**Identify:** `:r00=0.` → `:r00=60.` (max MHz). `:r01=0.` → `:r01=<serial>.`
- Use `identify: r00` with models `^:r00=(15|20|30|40|50|60)\.`. The vendor name is not in any reply, so the config has to name the driver; Detect cannot tell it from a FY.

**Registers** (CH1 / CH2)
| Reg | Operation | Encoding |
|---|---|---|
| 20 | output | `:w20=<ch1>,<ch2>.`; read `:r20=1,1.`. **Both channels in one register** |
| 21 / 22 | shape | 0 sine, 1 square, 2 pulse, 3 triangle, 4 partial-sine, 5 CMOS, 6 DC, 7 half-wave, 8 full-wave, 9 / 10 ladders, 11 noise, 12 / 13 exp, 14 multitone, 15 sinc, 16 Lorentz, 101-160 arb01-60 (same on both channels) |
| 23 / 24 | frequency | `:w23=<Hz×100>,0.` (unit field Y = 0). Read `X,Y`: f = X/100 × [1, 1, 1, 1e-3, 1e-6][Y] (on1arf; Y = 3/4 are mHz/µHz ranges) |
| 25 / 26 | amplitude | mV |
| 27 / 28 | offset | (X − 1000) / 100 V |
| 29 / 30 | duty | 0.1 % |
| 31 | phase | 0.1°, **global** (CH2 relative to CH1) |
| 32 / 33 | mode | sweep / burst etc. Frequency writes are refused in sweep mode |

**DSL fit ⛔**
- Framing is fine today: `rx terminator "\r\n"`, setters `reply: line` with `parse: [trim, {map: {":ok": "ok"}}]`, which turns a missing or wrong ack into an error.
- Register numbers per channel work via `from: "ch == 1 ? '21' : '22'"`.
- Query parsing works: `{regex: {pattern: "^:r2[12]=(\\d+)\\.", group: 1}}, number`, and CEL `(x - 1000.0) / 100.0`.
- **Blocked:** every setter needs the caller value transformed (Hz×100, V×1000, V×100+1000, %×10, °×10). The per-channel output needs read-modify-write of register 20 (`from` must combine the caller value with `outp_q`). Both are §5 F1.

**Confidence:** JDS6600 **high** (vendor PDF + sigrok + on1arf). JDS29xx low.

### 2.5 UNI-T UTG900E / UTG1000X / UTG2000B / UTG4000A

**Docs:** `D/unit_utg900e/doc/UTG900E_Programming_Manual_V1.0.pdf` (fw V3.06.004; older `ELV_UTG_Series_Programming_Manual.pdf`), `D/unit_utg1000x/doc/Uni-T_UTG1000X_Programming_Manual_0424.pdf`, `D/unit_utg2000b/doc/UTG2000B_Programming_Manual_V1.0.pdf`, `D/unit_utg4000a/doc/UTG4000A_Programming_Manual_V1.0.pdf`.
UTG1000A: no manual; assume UTG1000X.

**Models**
| Model | Ch | Sine | Notes |
|---|---|---|---|
| UTG932E / 962E | 2 | 30 / 60 MHz | 10 Vpp @50 Ω / 20 Vpp HiZ |
| UTG1022X(-PA) / 1042X | 2 | 20 / 40 MHz | -PA has a 4 W amplifier; max Vpp = load·20/(50+load), 1000X p14 |
| UTG2062B / 2082B / 2122B | 2 | 60 / 80 / 120 MHz | - |
| UTG4082A … 4202A | 2 | 80 … 200 MHz | - |

**Interface:** USBTMC (900E p23-27). UTG2000B also has LAN.
- **VID:PID undocumented.** Candidates are 6656 or 5345 (UNI-T scopes); verify on hardware.

**Identify:** `*IDN?` → `UNI-T Technologies, UTG900, 000000001, 00.00.01` (900E p5).
- The model field may be the family (`UTG900`, `UTG1000X`), not the nameplate, and has a space after the commas.
- Match `^UNI-T Technologies, ?UTG(9[0-9]{2}E?|1000X|10[24]2X|2000|20[68]2B|2122B|4[0-9]{3}A?)`, loosely.

**Command table** (`<n>` = 1 | 2. Pages 900E / 1000X / 2000B / 4000A.)

| Operation | Command | Query reply | Pages |
|---|---|---|---|
| Output | `:CHANnel<n>:OUTPut ON\|OFF\|1\|0` | `1` / `0` | p9 / p9 / p9 / p9 |
| Shape | `:CHANnel<n>:BASE:WAVe SINe\|SQUare\|PULSe\|RAMP\|ARB\|NOISe\|DC` (2000B adds HARMonic\|EXP and spells it `BASe`; 4000A adds HARMonic) | `SINe` (mixed case: parse with `upper`, then map `SINE→SIN, SQUARE→SQU, PULSE→PUL, NOISE→PRN`) | p11 / p13 / p16 / p13 |
| Frequency | `:CHANnel<n>:BASE:FREQuency <Hz>` | `2e+3` | p12 / p13 / p16 / p13 |
| Amplitude | `:CHANnel<n>:BASE:AMPLitude <v>` (unit from `:CHANnel<n>:AMPLitude:UNIT VPP\|VRMS\|DBM`) | `2e+0` | p13 / p14 / p17 / p14 |
| Offset | `:CHANnel<n>:BASE:OFFSet <V>` | sci | p13 / p15 / p18 / p14 |
| High / low level | `:CHANnel<n>:BASE:HIGH` / `:LOW` | sci | p13-14 |
| Phase | `:CHANnel<n>:BASE:PHASe <deg>` (−360…360) | `20` | p12 / p14 / p17 / p14 |
| Duty | `:CHANnel<n>:BASE:DUTY <pct>` | `20` | p14 / p16 / p19 / p15 |
| Symmetry | `:CHANnel<n>:RAMP:SYMMetry <pct>` | `20` | p14-15 / p16 / p19 / p16 |
| Load | `:CHANnel<n>:LOAD <Ω>` (1…10000, **10000 = HiZ**; 2000B up to 1 MΩ) | `50e+0` / `5.000000e+01` / `5e+1` | p11 / p12 / p10 / p11 |
| Mode | 900E: `:CHANnel<n>:MODe CONTINUE\|AM\|PM\|FM\|FSK\|Line\|Log`; others: `:CHANnel<n>:MODE CONTinue\|MODulation\|SWEep\|BURSt` | - | p15-16 / p9 / p8 / p9 |
| Key lock | `:SYSTem:LOCK ON\|OFF` | - | p5 |

**Quirks**
- Terminator LF (p3); setters are silent.
- Integer parameters must not carry a decimal point (900E p4). Use `%d` for integer params and `%g` otherwise.
- Real-number reply formats differ between models; `number` handles them all.
- No complete-state query: 7 queries per channel.
- The 900E's combined mode/modulation enum means 900E and 1000X+ need separate `mode` commands (not needed for the class today).
- `LOAD 10000` doubles as HiZ: map query `10000 → HIGHZ`.

**Confidence / fit:** commands **high**, reply formats medium (manuals contradict each other), USB IDs low → overall **medium**. ✅

### 2.6 OWON XDG2000/XDG3000, OWON DGE (defined), Tektronix AFG1000: the Tek-AFG dialect

**Docs:** `D/owon_xdg/doc/XDG_Waveform_Generator_SCPI_Protocol.pdf` (2018, 78 pp), `D/owon_xdg/doc/XDG2000_Series_User_Manual.pdf`, `D/owon_xdg/doc/XDG_Arbitrary_Waveform_Generator_User_Manual.pdf`, `D/tektronix_afg1000/doc/AFG1000-Programmer-Manual-EN-077112902.pdf`, `D/owon_dge/doc/` (see `device-notes/owon_dge.md`).
The XDG programmer manual is a near copy of the Tek AFG1000 one (same layout and commands), and the AFG1000 IDN carries the OWON-style `SCPI:99.0 FV:` field. **AFG1022/1062 is an OWON OEM unit.**

**Models**
| Model | Ch | Max output | Amplitude @50 Ω |
|---|---|---|---|
| XDG2035 … 2100 | 2 | 35 … 100 MHz | 10 Vpp ≤25 MHz, 5 Vpp ≤60 MHz, 2.5 Vpp ≤100 MHz |
| XDG3162 … 3252 | 2 | 160 … 250 MHz | - |
| AFG1022 / AFG1062 | 2 | 25 / 60 MHz | - |

**Interface:** USBTMC.
- XDG: Utility → I/O → USBDEV must be **USBTMC**; the factory default is PC (as on DGE).
- XDG VID:PID is probably 5345:1235 like the DGE (verify). Tek VID 0699, PID undocumented.
- XDG2000 also has LAN and an optional COM.

**Identify:** `OWON,XDG3202,1837001,SCPI:99.0 FV:V1.2.0` (XDG p17), `TEKTRONIX,AFG1062,1331030,SCPI:99.0 FV:2.0.0` (AFG1000 p24).

**Commands** (pages XDG / AFG1000)
| Operation | Command | Reply | Pages |
|---|---|---|---|
| Output | `OUTPut{ch}[:STATe] ON\|OFF` | `1` / `0` | p19 / p28 |
| Load | `OUTPut{ch}:IMPedance <Ω>\|INFinity\|MIN\|MAX` (1 Ω-10 kΩ) | `9.9E+37` = HiZ | p18 / p27 |
| Shape | `SOURce{ch}:FUNCtion[:SHAPe] SINusoid\|SQUare\|PULSe\|RAMP\|PRNoise\|<builtin>\|USER<n>\|EMEMory` (built-in names are case-sensitive, not abbreviable) | `SIN SQU PULS RAMP PRN` | p45-46 / p52-54 |
| Frequency | `SOURce{ch}:FREQuency[:FIXed] <f>` | - | p36 / p45 |
| Amplitude | `SOURce{ch}:VOLTage[:LEVel][:IMMediate][:AMPLitude] <v>` | - | p66 / p70-71 |
| Offset | `SOURce{ch}:VOLTage:OFFSet <v>` | - | p66 / p70 |
| Phase | `SOURce{ch}:PHASe[:ADJust] <x>[RAD\|DEG]` (default **RAD**, query answers RAD) | - | p51 / p54-55 |
| Symmetry | `SOURce{ch}:FUNCtion:RAMP:SYMMetry` | - | p45 / - |
| Pulse duty | `SOURce{ch}:PULSe:DCYCle` (no square-duty command) | - | p58 / p62 |

**Quirks**
- SI suffixes are allowed (`15MHz`), a bare `15M` is rejected (AFG1000 p17). Send plain `%g` numbers.
- Error −201 "Invalid while in local" exists; DGE setters worked without remote. XDG has `SYSTem:KLOCk`.
- On DGE every setter is silent and a read after a setter desyncs the link (bench). Assume the same here.

**Fit:** `owon_dge.yaml` is already this dialect, so make it the `tek_afg` base (§3) and derive XDG and AFG1000 by changing models, link and limits. **High; ✅.**

### 2.7 Keysight EDU33210 / 33500B / 33600A / 33210A / 33220A

**Docs**
| Path | Doc |
|---|---|
| `D/keysight_33220a/doc/33220A_Users_Guide_33220-90002_Ed4.pdf` | Full remote reference; printed = PDF − 2 |
| `D/keysight_33220a/doc/33220A_Quick_Reference_33220-90008.pdf` | Quick reference |
| `D/keysight_edu33210/doc/EDU33210_Users_Guide_EDU33212-90002_Ed4.pdf` | Printed = PDF |
| `D/keysight_33500b/doc/SOURCES.md` | Only the URL: the Trueform guide (asset 9018-03714) and the EDU Programming Guide (9921-01382) **need a Keysight login** |

Open source: pymeasure `agilent33500.py`, `agilent33220A.py`.

**Models**
| Model | Ch | Sine | Amplitude | Offset | Confidence |
|---|---|---|---|---|---|
| 33220A | 1 | 20 MHz | 10 mVpp-10 Vpp @50 Ω | \|offs\| + Vpp/2 ≤ 5 V (50 Ω) / 10 V (HiZ) (p180-183) | high |
| 33210A | 1 | 10 MHz | same commands | - | - |
| EDU33211A / EDU33212A | 1 / 2 | 20 MHz | Vmax ±5 V @50 Ω | - | high |
| 33509B…33522B / 33611A…33622A | 1-2 | 20-120 MHz | 1 mVpp-10 Vpp @50 Ω | - | from memory / pymeasure, medium |

**Interfaces (USBTMC)**
| Model | VID:PID | Source |
|---|---|---|
| 33220A | 0957:0407 | `usb.ids` |
| 33210A | 0957:1507 | `usb.ids` |
| EDU33212A | 2A8D:8D01 | guide PDF 52 |
| EDU33211A | 2A8D:8C01 | guide PDF 52, printed "0x08C01" |
| 33500B | 0957:2C07 | memory, verify |
| 33600A | 0957:5707 | memory, verify |

LAN: socket 5025 / VXI-11 (no TCP in BenchMesh). The 33220A also has GPIB.

**Identify:** `Agilent Technologies,33220A,<serial>,f.ff-b.bb-aa-p` (p135 PDF); `Keysight Technologies,EDU33212A,<serial>,K-01.00.04-…` (PDF 149). The 33500B says Agilent or Keysight depending on firmware era, so match on the model field.

**Commands** (`[SOURce[1|2]:]` optional; single-channel units take no suffix)
| Operation | Command | Reply | Pages |
|---|---|---|---|
| Output | `OUTPut[1\|2] ON\|OFF` | `0` / `1` | 33220A p189 PDF; EDU PDF 93 |
| Load | `OUTPut[1\|2]:LOAD <Ω>\|INFinity\|MIN\|MAX\|DEF` (1 Ω-10 kΩ) | ohms or `9.9E+37` (Trueform `+9.90000000000000E+37`): treat ≥ 9e37 as HIGHZ | 33220A p190 PDF |
| Shape | Trueform/EDU: `FUNCtion SIN\|SQU\|TRI\|RAMP\|PULS\|PRBS\|NOIS\|ARB\|DC`; 33220A: `SINusoid\|SQUare\|RAMP\|PULSe\|NOISe\|DC\|USER` | `SIN SQU RAMP PULS NOIS DC USER` | EDU PDF 83; 33220A p180 PDF |
| Frequency | `FREQuency <Hz>` | `+1.0000000000000E+03` | - |
| Amplitude | `VOLTage <v>` (unit `VOLT:UNIT VPP\|VRMS\|DBM`) | - | 33220A p192 PDF |
| Offset | `VOLTage:OFFSet <V>` | - | - |
| Phase | `PHASe <deg>` | - | - |
| Duty | `FUNCtion:SQUare:DCYCle <pct>` (EDU 0.01-99.99 %; 33220A 20-80 %) | - | - |
| Symmetry | `FUNCtion:RAMP:SYMMetry <pct>` | - | EDU PDF 90-91 |
| **Complete state** | `[SOUR<n>:]APPLy?` → `"SIN +5.0000000000000E+03,+3.0000000000000E+00,-2.5000000000000E+00"` (shape, Hz, amplitude, offset; quoted) | - | 33220A p179 PDF |

- `APPLy?` parse: `[trim, unquote, {replace: {from: " ", to: ","}}, {fields: {sep: ",", names: [shape, freq, ampl, offs]}}]`.

**Quirks**
- Out-of-range values are clipped with a "Settings conflict" error; offset may be reduced to satisfy Vmax. Read back.
- `APPLy:<fn>` turns the output **on** and resets duty/symmetry. Never use it as a setter: use the individual commands (they are configure-safe).
- The output relay glitches for ~1 ms on OUTP / LOAD changes.
- Changing LOAD rescales the displayed amplitude.
- Remote locks the front panel; `SYST:LOC` releases it. Add `session.on_disconnect: [syst_loc]`.

**Confidence / fit:** 33220A and EDU **high**, 33500B/33600A **medium** (guide behind a login, PIDs from memory). ✅

### 2.8 GW Instek AFG-2000/2100, AFG-2225, AFG-3000

**Docs:** `D/gwinstek_afg2000/doc/AFG-2000_User_Manual_farnell_2713159.pdf` (newer EF1; remote chapter p87-138) and `…_2313573.pdf` (older), `D/gwinstek_afg2225/doc/AFG-2225_User_Manual_farnell_2344129.pdf` (remote p180-290), `D/gwinstek_afg3000/doc/AFG-3000_User_Manual_vF.pdf` (remote p182-290).

**Models**
| Model | Ch | Sine | Amplitude | Offset |
|---|---|---|---|---|
| AFG-2005/2105 | 1 | 5 MHz | 1 mVpp-10 Vpp @50 Ω (half at 20-25 MHz) | ±5 Vpk @50 Ω (p142-143) |
| AFG-2012/2112 | 1 | 12 MHz | same | same |
| AFG-2025/2125 | 1 | 25 MHz | same | same |
| AFG-2225 | 2 | 25 MHz | - | ±4.99 V @50 Ω (p202) |
| AFG-3051 / 3081 | 1 | 50 / 80 MHz | - | - |

The 21xx models add AM/FM/FSK/sweep/counter.

**Interface**
- AFG-2000 / AFG-2225: USB **CDC virtual COM** (GW driver .inf; the manual says to read the COM settings from Device Manager and gives no baud: p89, p181). For CDC the baud is ignored, so 115200 8N1 is a safe declaration. VID probably 2184 (GW), PID undocumented.
- AFG-3000: also RS-232 DB9 null-modem, 9600-115200, 8N1 / 7O1 / 7E1 (p183-185), and GPIB.

**Identify:** `GW INSTEK,AFG-2025,SN:XXXXXXXX,Vm.mm` (AFG-2000 p98), `GW INSTEK,AFG-2225,…` (p193). Match `^GW INSTEK, ?AFG-(2[01](05|12|25)|2225|30[58]1),`.

**Commands** (AFG-2000 single channel; AFG-2225 `SOURce1|2`, `OUTPut1|2`)
| Operation | Command | Reply | Pages (2000 / 2225) |
|---|---|---|---|
| Output | `OUTPut ON\|OFF` (2000: **no channel suffix**) | `1` / `0` | p112 / p213 |
| Load | `OUTPut:LOAD DEFault\|INFinity` (only 50 Ω or HiZ) | `DEF` / `INF` → map `DEF → "50"`, `INF → HIGHZ` | p113 / p213-214 |
| Shape | `SOUR1:FUNC SINusoid\|SQUare\|RAMP\|NOISe\|USER` (2225 adds PULSe; 3000 adds TRIAngle\|DC) | `SIN SQU RAMP PULS NOIS USER` | p107 / p207 |
| Frequency | `SOUR1:FREQ <f>` | `+1.0000000000000E+03` | p108 / p208 |
| Amplitude | `SOUR1:AMPL <v>` | `+5.0000E+00` | p109 / p209-210 |
| Offset | `SOUR1:DCOffset <V>` | `+3.0000E+00` | p110 / p210-211 |
| Duty | `SOUR1:SQUare:DCYCle <pct>` (1-99 % <100 kHz … 50 % fixed >10 MHz) | `+5.00E+01` | p110-111 / p211 |
| Symmetry | `SOUR1:RAMP:SYMMetry <pct>` | `+1.0000E+02` | p111-112 / p212 |
| Phase | AFG-2000: **none**; AFG-2225: `SOUR1:PHASe <deg>` (−180…180) | `26` | - / p267 |
| Amplitude unit | `SOUR1:VOLTage:UNIT VPP\|VRMS\|DBM` | `VPP` | p113-114 / p214 |
| **Complete state** | `SOUR1:APPLy?` → `SIN +5.0000000000000E+03,+3.0000E+00,-2.50E+00` (Keysight shape, **without quotes** as printed: accept both) | - | p105 / p205 |

**Quirks**
- Terminator LF or CRLF (p94). Setters are silent.
- `APPLy:<fn>` switches the output **on** and cancels modulation (p100). Don't use it.
- Clamped values raise error −222.
- Load switching halves or doubles the stored amplitude.
- AFG-2225 locks the front panel in remote except F5 (p182). AFG-3000 has `SYST:REM` / `SYST:LOC`.
- The AFG-2000 has no phase, so leave `phase` unbound (it is optional in the contract).

**Fit:** `link.serial` + `rx terminator "\n"` (strip `\r`). Single-channel `OUTPut` without suffix works with a literal command (channels: 1). AFG-2225 is ✅ with `{ch}`. **High; ✅.**

### 2.9 Rigol DG1022 / DG1022A (old, non-Z)

**Docs:** `D/rigol_dg1022/doc/DG1000_ProgrammingGuide_EN.pdf` (PGB06109, 100 pp; printed 2-n = PDF − 12).

**Models:** DG1022 20 MHz / DG1022A 25 MHz sine; square 5 MHz, ramp 150 / 500 kHz.
- CH1 2 mVpp-10 Vpp @50 Ω, ±5 V.
- **CH2 2 mVpp-3 Vpp, ±1.5 V**: per-channel limits are needed (§5 F3).

**Interface:** USBTMC **0400:09C4** (National Semiconductor VID; `usb.ids` "Rigol DG1022"), no LAN.

**IDN:** `RIGOL TECHNOLOGIES,DG1022,DG1D100…,00.02.00.06.00.02.06` (p14, uppercase vendor).

**Channel syntax:** a `:CH2` **suffix**; CH1 has none (`FREQ 1000` / `FREQ:CH2 1000`).
- Works today with `send: "FREQ{sfx} {hz}"` and `sfx: {type: string, from: "ch == 1 ? '' : ':CH2'"}`.

**Commands**
| Operation | Command | Reply | Page |
|---|---|---|---|
| Output | `OUTP{sfx} ON\|OFF` | `ON` / `OFF` | p36-39 |
| Load | `OUTP:LOAD{sfx} <Ω>\|INF` | ohms or `Infinity` | p37-39 |
| Shape | `FUNC{sfx} SIN\|SQU\|RAMP\|PULS\|NOIS\|DC\|USER` | **prefixed** `CH1:SIN` (DC and USER both read back `ARB`) | p22-24 |
| Frequency | `FREQ{sfx}` | `1.000000e-06`; CH2 `CH2:1.000000e-06` | p28 |
| Amplitude | `VOLT{sfx}` | CH2 `CH2: 4.000000e-03` (note the space) | p32-34 |
| Offset | `VOLT:OFFS{sfx}` | - | p33-34 |
| Duty | `FUNC:SQU:DCYC{sfx}` | `50.000000` | p23-25 |
| Symmetry | `FUNC:RAMP:SYMM{sfx}` | - | p23-25 |
| Phase | `PHAS{sfx}` **−180…180** | `90.000` | p75 |
| **Complete state** | `APPL{sfx}?` → `CH1:"SIN,1.000000e+03,5.000000e+00,-1.500000e+00"` (4 fields, no phase) | - | p15-20 |
| Remote/local | `SYST:LOC` / `SYST:REM` | - | p72 |

- Parse: `[trim, {regex: {pattern: '"(.*)"', group: 1}}, {fields: …}]`. For single values, `{regex: {pattern: "([-+0-9.eE]+)\\s*$", group: 1}}` strips the `CHn:` prefix.

**Quirks:** EEVblog reputation for needing long inter-command gaps and flaky USBTMC (not sourced). Start with `min_gap_ms: 50`, `gap_after_write_ms: 100` and verify with a capture.

**Confidence:** medium. ✅ (per-channel amplitude limits ⚠️)

### 2.10 Hantek HDG2000B (HDG2032B/2062B/2082B/2102B)

**Docs:** `D/hantek_hdg2000/doc/HDG2000B_SCPI_protocol_EN.pdf` (51 pp), `HDG2000B_Manual_EN_V1.2.pdf`, `HDG3000B_Programming_Manual.pdf` (sibling family).

**Models:** up to 100 MHz. Amplitude @50 Ω: 10 Vpp ≤20 MHz, 7.5 Vpp ≤60 MHz, 5 Vpp ≤80 MHz, 1 Vpp ≤100 MHz.

**Interface:** USBTMC **049F:505B** (user manual p92).

**Commands**
| Operation | Command | Reply | Page |
|---|---|---|---|
| Output | `OUTPut<n> ON\|OFF` | `ON` / `OFF` | p26 |
| Shape | `SOURce<n>:FUNCtion SINusoid\|SQUare\|RAMP\|PULSe\|NOISe\|DC\|USER\|HARMonic\|…` | **full words** (`SINusoid`): map `SINUSOID→SIN`, `SQUARE→SQU`, `PULSE→PUL`, `NOISE→PRN` after `upper` | p19 |
| Frequency | `SOURce<n>:FREQuency` | `+1.000000000000000E+03` | p17 |
| Amplitude | `SOURce<n>:VOLTage` | - | p40 |
| Offset | `SOURce<n>:VOLTage:OFFSet` | - | p40 |
| Phase | `SOURce<n>:PHASe` (0-360) | - | p26 |
| Duty | `SOURce<n>:FUNCtion:SQUare:DCYCle` | - | p20 |
| Symmetry | `SOURce<n>:FUNCtion:RAMP:SYMMetry` | - | p20 |
| Load | **none over SCPI** (front panel only) | - | - |

**Quirks:** terminator LF (CRLF accepted, p3). No `*IDN?` example: match `^.*HDG20[3-9]2B` loosely and confirm. Doc has many typos; the product is labelled discontinued.

**Confidence:** medium. ✅

### 2.11 FeelTech FY3200S / FY3224S (old)

**Docs:** `D/feeltech_fy3200/doc/FY3200S_Users_Manual_CN_Rev2.1.pdf` (Chinese; specs only, no official protocol). Protocol from python-feeltech (`github.com/atx/python-feeltech`), CPAN `Electronics::SigGen::FY3200`, `github.com/kbralten/fy3200s`.

**Interface:** 9600 8N1, LF, lowercase only, ≤15 characters per line.
- Setters are silent, and invalid commands are silently ignored. Keep ≥50 ms between commands.
- Identify: `a` → `FY3224S`.

**Commands**
- Main channel prefix `b`, secondary `d`.
- `bw<n>` shape, `bf<centi-Hz>` frequency, `ba%.2f` amplitude, `bo%.2f` offset, `bd<0.1 %>` duty, `dp<deg>` phase (CH2 only).
- Queries are **CH1 only** and echo the prefix: `cf` → `cf0000000050` (centi-Hz), `cd` → `cd500`.

**Gaps:** no output on/off command and no CH2 read-back. Two conflicting waveform tables (firmware-dependent).

**Fit ⛔:** the class contract **requires** `output` get+set, so this family cannot be bound unless the contract makes output optional. Frequency in centi-Hz also needs §5 F1. **Low confidence; not recommended.**

### 2.12 Not pursued / blocked

- **OWON AG1022/AG1012/AG-F/AG-S** (`D/owon_ag/doc/AG_Series_Waveform_Generator_SCPI_Protocol.pdf`, 2013)
  - IDN `OWON,AG1022,AG10221331030,V_4.0.1` (p7). Stateful channel select `:CHANnel CH1|CH2` (p21), then `:FUNCtion:<shape>:FREQuency` etc. (p8-10). Output `:CHANnel:CH1 ON|OFF` → `ON` / `OFF`.
  - **Every command is answered**: `->\n` accepted, `=?\n` not understood, `NULL\n` rejected (p4).
  - The USB port needs OWON's proprietary `ultrawave\USBDRV` driver (neither TMC nor CDC). Only RS-232 (baud undocumented) is reachable.
  - Per-channel commands need a `before: [chan_select]` step (that works today, as with the DHO800 `:WAV:SOUR`), and setters `reply: line` with `map: {"->": ok}`.
  - Low priority.
- **Tektronix AFG31000:** programmer manual not obtained (tek.com loads links by JavaScript, guessed URLs 404). Expected AFG3000 command set (close to §2.6). Download it by hand from https://www.tek.com/en/support/products/afg31000. Note in `D/tektronix_afg31000/doc/SOURCES.md`.
- **JDS2800, JDS8000, Fnirsi combo devices:** not researched (search budget exhausted). Fnirsi handhelds have no known remote-control protocol.

---

## 3. Shared baselines (`devices/bases/*.yaml`, used via `extends`)

`extends` is implemented (`bm-devicedef/src/lib.rs`): mappings merge key by key and `null` removes an inherited key. No `devices/bases/` directory exists yet. Proposed bases:

### 3.1 `scpi_awg`: Rigol DG800/900/1000Z/2000/4000, Keysight 33500B/EDU, Hantek HDG2000B (GW and 33220A partly)

| Command | Wire | Notes |
|---|---|---|
| `idn` | `*IDN?` → `[trim]` | |
| `outp_q` | `OUTP{ch}?` → `[trim, upper, {map: {"1": ON, "0": OFF, "ON": ON, "OFF": OFF}}]` | Rigol and Hantek answer text, Keysight 0/1. The map covers both |
| `outp_set` | `OUTP{ch} {on}`, `safety: energize` | |
| `shape_q` | `SOUR{ch}:FUNC?` → `[trim, upper, {map: {SIN: SIN, SQU: SQU, RAMP: RAMP, PULS: PUL, PULSE: PUL, NOIS: PRN, NOISE: PRN, DC: DC, USER: ARB, ARB: ARB, _: ""}}]` | |
| `shape_set` | `SOUR{ch}:FUNC {shape}` with values `{SIN: SIN, SQU: SQU, RAMP: RAMP, PUL: PULS, PRN: NOIS, DC: DC, ARB: USER}` | Keysight Trueform: `ARB: ARB` |
| `freq_q` / `freq_set` | `SOUR{ch}:FREQ?` / `SOUR{ch}:FREQ {hz}` (`%g`) | |
| `ampl_q` / `ampl_set` | `SOUR{ch}:VOLT?` / `SOUR{ch}:VOLT {vpp}` | Assumes `VOLT:UNIT VPP`: `session.on_connect: [{call: unit_set, args: {u: VPP}}]` |
| `offs_q` / `offs_set` | `SOUR{ch}:VOLT:OFFS?` / `SOUR{ch}:VOLT:OFFS {v}` | |
| `phase_q` / `phase_set` | `SOUR{ch}:PHAS?` / `SOUR{ch}:PHAS {deg}` (degrees) | |
| `duty_q` / `duty_set` | `SOUR{ch}:FUNC:SQU:DCYC` | |
| `symm_q` / `symm_set` | `SOUR{ch}:FUNC:RAMP:SYMM` | |
| `load_q` / `load_set` | `OUTP{ch}:LOAD?` → `[trim, upper, {map: {"INFINITY": HIGHZ, "INF": HIGHZ, _: …}}]`, then CEL `num(x) >= 9e37 ? 'HIGHZ' : string(int(num(x)))`; set `OUTP{ch}:LOAD {ohm}` with enum `{HIGHZ: INF, "50": "50"}` | |

Framing: `tx "\n"`, `rx message`, `strip_trailing "\n"`, all setters `reply: none`, `gap_after_write_ms: 20`, `timeout_ms: 1000`. Per-device overrides:

| Family | Overrides |
|---|---|
| Rigol | add `appl_q` (`:SOUR{ch}:APPL?`) |
| Keysight 33220A | no `SOUR{ch}`/`OUTP{ch}` suffix (single channel: `null` the base command and give a literal one) |
| Hantek | `load_*: null` |
| GW AFG | `ampl` → `SOUR1:AMPL`, `offs` → `SOUR1:DCOffset`, `duty` → `SOUR1:SQUare:DCYCle`, serial link |

### 3.2 `tek_afg`: OWON DGE (exists), OWON XDG, Tek AFG1000

The body of today's `owon_dge.yaml`: `OUTPut{ch}:STATe`, `SOURce{ch}:FUNCtion:SHAPe` (reply `SIN SQU PULS RAMP PRN`), `SOURce{ch}:FREQuency`, `SOURce{ch}:VOLTage`, `VOLTage:OFFSet`, `PHASe {deg}DEG` (query in RAD → CEL ×180/π), `FUNCtion:RAMP:SYMMetry`, `OUTPut{ch}:IMPedance` (9.9E+37 = HiZ).
Move it to `devices/bases/tek_afg.yaml` with DGE keeping `tx "\r"` (Bench-tested). New members use `"\n"`.

### 3.3 `siglent_sdg`: SDG1000X / 2000X / 6000X / X Plus, BK 4050B / 4060B

| Command | Wire |
|---|---|
| `bswv_*` queries | each sends `C{ch}:BSWV?`; parse with `{regex: {pattern: "[ ,]KEY,([-+0-9.eE]+)", group: 1}}, number` |
| `outp_q` | `C{ch}:OUTP?` → regex `OUTP (ON\|OFF)` |
| `load_q` | regex `LOAD,(\w+)` → map `HZ → HIGHZ` |
| Setters | `C{ch}:BSWV FRQ,{hz}` / `AMP,{vpp}` / `OFST,{v}` / `PHSE,{deg}` / `DUTY,{pct}` / `SYM,{pct}` / `WVTP,{shape}` (`{SIN: SINE, SQU: SQUARE, RAMP: RAMP, PUL: PULSE, PRN: NOISE, DC: DC, ARB: ARB}`), `C{ch}:OUTP {on}` (energize), `C{ch}:OUTP LOAD,{ohm}` (`HIGHZ → HZ`) |

Old SDG800/SDG1000:
- `on_connect: CHDR OFF`: replies then have no header and no units, and the same regexes still match because they don't anchor on units.
- SDG800 has no `C{ch}:` prefix.

### 3.4 `unit_utg`: UTG900E / 1000X / 2000B / 4000A

`:CHANnel{ch}:OUTPut`, `:CHANnel{ch}:BASE:{WAVe,FREQuency,AMPLitude,OFFSet,PHASe,DUTY}`, `:CHANnel{ch}:RAMP:SYMMetry`, `:CHANnel{ch}:LOAD`. The 2000B overrides the spelling (`BASe` is the same short form `BAS`, so the short forms `:CHAN{ch}:BAS:FREQ` work for all).

### Dialect differences at a glance

| Item | Rigol `:SOURn:` | Keysight 33xxx | Siglent `Cn:BSWV` | Tek-AFG / OWON | UNI-T |
|---|---|---|---|---|---|
| Channel | `:SOUR2:FREQ` | `SOUR2:FREQ` (optional on 1-ch) | `C2:BSWV FRQ,…` | `SOUR2:FREQ` | `:CHAN2:BASE:FREQ` |
| Shape set | `FUNC SIN\|SQU\|RAMP\|PULS\|NOIS\|DC\|USER` | `FUNC SIN\|SQU\|TRI\|RAMP\|PULS\|NOIS\|DC\|ARB` | `WVTP,SINE\|SQUARE\|RAMP\|PULSE\|NOISE\|DC\|ARB` | `FUNC:SHAP SIN\|SQU\|PULS\|RAMP\|PRN` | `BASE:WAV SIN\|SQU\|PULS\|RAMP\|NOIS\|DC\|ARB` |
| Output query | `ON` / `OFF` | `0` / `1` | `C1:OUTP ON,LOAD,…` | `0` / `1` | `0` / `1` |
| HiZ load reads | `9.900000E+37` (DG4000 `INFINITY`) | `9.9E+37` | `HZ` | `9.9E+37` | `10000` |
| Phase unit | deg | deg | deg | **rad** | deg |
| Square duty | `FUNC:SQU:DCYC` | `FUNC:SQU:DCYC` | `BSWV DUTY,` | none (pulse only) | `BASE:DUTY` |
| Complete state | `APPL?` quoted, 5 fields | `APPL?` quoted, 4 fields | `BSWV?` key,value, units glued | none | none |
| Reply header | no | no | always (X), `CHDR OFF` (old) | no | no |

---

## 4. Class contract proposals (AWG)

The contract exists. Additions supported by every SCPI family above:

| Addition | Type | Bound by | Notes |
|---|---|---|---|
| operation `load` | get/set, enum or string (`"50"`, `"HIGHZ"`, optionally numeric ohms) | Rigol, Siglent, Keysight, Tek-AFG, UNI-T, GW (50 / HiZ only) | status `LOAD` already exists |
| operation `duty` + status `DUTY` | number % | Rigol, Siglent, Keysight, UNI-T, GW, FeelTech, JDS | not Tek-AFG: square is fixed 50 % |
| `features.max_freq_hz` per shape (`{SIN: 6e7, SQU: 2.5e7, …}`) | - | - | lets the panel show limits; instruments clamp anyway |
| make `output` `required: [set]` only (get optional) | - | - | would admit write-mostly units such as FY3200, but they also lack output on/off, so it is not worth changing for them |

Panel: as today (shape buttons, frequency / amplitude / offset editors, output arm switch). Add a load toggle **50 Ω / HiZ** and a duty editor when the setters exist.

---

## 5. DSL features that blocked or limited families

| # | Feature | Unblocks | Sketch |
|---|---|---|---|
| **F1** | **Caller value visible in computed params** (`from` CEL gets the caller params, e.g. `value` or the param name) | FeelTech FY6600/6800/2300 frequency (µHz), JDS6600 (all setters: ×100, mV, offset +1000, 0.1 %; output register read-modify-write `ch == 1 ? on + ',' + other : other + ',' + on`), FY3200 (centi-Hz), FeelTech per-channel waveform index (`ch == 1 ? idx1[shape] : idx2[shape]`) | `params: {raw: {type: string, from: "string(int(hz * 100.0)) + ',0'"}}` |
| F2 | `pairs` parse step (key,value list → map, optional unit strip) | Siglent / BK: one `BSWV?` per channel instead of ~6 | `{pairs: {sep: ",", skip: "^C\\d:BSWV ", strip_units: true}}` |
| F3 | Per-channel limits in setter specs (`features`/`params` override per channel) | Rigol DG1022 and Siglent SDG1000 CH2 (3 Vpp / ±1.5 V @50 Ω) | `params.vpp.max_by_ch: {2: 6}` |
| F4 | `usbtmc` with a list of PIDs (or VID-only + identify) | One Rigol definition across DG800 (0643) / DG1000Z (0642) / DG2000 / DG4000; Siglent 1101/1102 | `usbtmc: {vid: 1AB1, pid: [0642, 0643]}` |
| F5 | Regex parse "no match → null" (like `on_empty: "null"`) | Siglent `DUTY` / `SYM` only present for some waveforms | `{regex: {…, missing: null}}` |
| F6 | TCP transport (declared, not implemented) | LAN-only use of Rigol DG1000Z/DG2000/DG4000, Siglent X, Keysight, UNI-T UTG2000B (all also have USB, so not blocking) | - |
| - | Binary framing | not needed for the class (only arb upload and screenshots) | - |

Already possible today and worth knowing:
- Channel-dependent wire text via `from: "ch == 1 ? 'M' : 'F'"` (FeelTech `WM*` / `WF*`, DG1022 `:CH2`, JDS registers 21 / 22).
- Swallowing stray `\n` setter acks via `rx.skip_leading` (FeelTech).
- `reply: line` + `map` for acked setters (JDS `:ok`, OWON AG `->`).
- `before` channel-select steps (OWON AG).

## 6. First-contact checklist per family (community)

1. `cargo run -p bm-transport --example ports`: record vid:pid (Rigol DG900/DG2000/DG4000, Siglent per model, BK, UNI-T, GW, Tek, XDG).
2. `bm-probe verify` (read-only): IDN string, output query format (`ON` vs `1`), HiZ load reply, `APPL?` / `BSWV?` exact reply.
3. One setter at a time with the output **off** (frequency, amplitude), then read back. Check the gap needed after setters (DG1022, SDG1000X, FeelTech).
4. FeelTech: confirm 8N2 vs 8N1, the frequency unit per firmware (`UVE`), and the CH2 waveform indices.
