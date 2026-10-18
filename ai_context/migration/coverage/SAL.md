# SAL — Spectrum analyzers (coverage research)

Status: research report, 2026-10-18. No definition written yet; every family below is
**Awaiting confirmation** at best. The class `SAL` is reserved but has **no contract** yet — a
proposal is in §4. Manuals live outside the repo in `/home/marek/project/BenchMeshDOC/<vendor>_<family>/doc/`
(each with `SOURCES.md`); page numbers are **PDF pages** of those local files (`pN`), printed page in
parentheses where useful.

**The main finding is a DSL gap that affects every family:** spectrum analyzers return their trace
as **ASCII numbers separated by commas** (SCPI `:TRACe:DATA?`, often wrapped in an IEEE 488.2
block header) or as **newline-separated lines** (tinySA/NanoVNA `data`). Today's DSL can only produce
a `list` from **binary integer** samples (`{samples: u8|i16le|…}`). With one new parse step —
`{numbers: {sep: ","}}` (ASCII list → list of f64, §5) — the trace becomes reachable for all
SCPI families and for tinySA. Everything else (settings, markers, peak search, sweep control)
already fits today's DSL.

---

## 1. Ranking (hobby / small-lab prevalence)

| # | Family | Price band | Protocol | Transport | Doc quality | Confidence | DSL fit |
|---|---|---|---|---|---|---|---|
| 1 | **tinySA / tinySA Ultra / Ultra+** (ZS-405/406/407) | 60–200 € | text shell `ch> ` | USB CDC 0483:5740 | wiki + firmware source (authoritative) | high | settings/markers **today**; trace needs `numbers` |
| 2 | **Rigol DSA815/832/875 (-TG)** | 1–2.5 k€ (used 500 €) | SCPI | USBTMC 1ab1, LAN | full programming guide | high | today except trace (`numbers` after `block`) |
| 3 | **Siglent SSA3000X / X Plus / X-R, SVA1000X** (+SSA5000A) | 1–5 k€ | SCPI | USBTMC f4ec, LAN 5025 | full programming guides | high | today except trace (`numbers`, trailing comma) |
| 4 | **UNI-T UTS1000B/T, UTS3000A/B/T** (+UTS5000A) | 1–4 k€ | SCPI | USBTMC, LAN | full manual (one for all series) | medium-high | today except trace (`numbers` after `block`) |
| 5 | NanoVNA-H / H4 (and NanoVNA-D fw) | 30–80 € | text shell `ch> ` | USB CDC 0483:5740 | firmware source | high | **VNA class**, not SAL (§6) |
| 6 | RF Explorer (3G/6G/WSUB1G/PLUS…) | 150–400 € | binary-length-prefixed, streaming | USB-serial CP210x, 500 kbaud | library source + UART spec (wiki) | medium | **blocked** (push stream, binary) |
| 7 | **Rigol RSA3000(E/N) / RSA5000** | 4–15 k€ | SCPI | USBTMC 1ab1, LAN raw 5555 | full programming guides | high | as DSA800 |
| 8 | GW Instek GSP-9300(B)/9330, GSP-818 | 1.5–6 k€ | SCPI | **RS-232** (9300), USBTMC, LAN | complete (9300) / sparse (818) | high / medium | today except trace (`numbers`) |
| 9 | Keysight N9320B / N9322C, FieldFox | 5–30 k€ (used N9320B 1.5 k€) | SCPI | USBTMC 0957:2118 (N93xx); FieldFox LAN | good / FieldFox thorough | medium / high | N93xx today except trace; FieldFox needs **TCP** |
| 10 | R&S FPC1000/FPC1500, FPH | 1.5–10 k€ | SCPI | **LAN only** (USB is a network link) | complete user manuals | medium-high | **blocked on TCP** |
| 11 | Hantek HSA2000 / OWON XSA1000/XSA800 | 1–3 k€ | SCPI (Hantek) / unknown (OWON) | USB, LAN | thin (Hantek) / none (OWON) | low | Hantek: no marker readout; OWON: needs capture |
| 12 | LiteVNA, NanoVNA V2 / SAA-2(N) | 50–150 € | **binary register protocol** | USB CDC | user-guide appendix | high (protocol) | **blocked**; VNA anyway |
| – | Signal Hound (BB60/SM/SA44) | 1–20 k€ | proprietary C API (Spike has SCPI over TCP) | USB 3 | vendor SDK | – | out of scope (note only) |
| – | HackRF | – | SDR, not an instrument | – | – | – | skipped |

---

## 2. Families

### 2.1 tinySA (basic, ZS-304), tinySA Ultra (ZS-405), Ultra+ (ZS-406 / ZS-407)

- **Models**: tinySA (ZS-304; 100 kHz–350 MHz low / 240–960 MHz high input, ≤ 290 points, RBW
  2–600 kHz, `rbw` usage text sa_cmd.c l.580); tinySA Ultra ZS-405 (100 kHz–5.3 GHz, ≤ 450 points,
  RBW 0.2–850 kHz, l.578); Ultra+ ZS-406/ZS-407 (higher upper limit — exact per-model range to
  take from the device's `info`/menus at first contact; same shell). Points: `nanovna.h` l.195/197. All share one firmware tree (`tinySA4` build = Ultra/Ultra+, `tinySA` = basic).
  Built-in signal generator (output mode `mode low|high output`, `level` −76…−6 / −38…+13 dBm)
  — tracking-generator-like, `energize`.
- **Interface**: USB CDC-ACM (ChibiOS), **VID:PID 0483:5740** (`usbcfg.c` l.44-45) — the same
  ID as NanoVNA-H and every other ChibiOS/STM32 VCP. Baud ignored (CDC). Identity must therefore
  come from the identify reply, never from vid:pid alone (see IDENTITY.md).
- **Protocol** (source: `erikkaashoek/tinySA` `main.c`):
  - Line input ends on **`\r`** (`VNAShell_readLine` l.3113); `\n` and other control chars are
    ignored (l.3118) — so `tx.terminator: "\r"` (or `"\r\n"`, harmless).
  - Every typed character is **echoed** (l.3119), then `\r\n` on Enter (l.3114).
  - Command output lines end `\r\n`; after the command the device prints the prompt **`ch> `**
    (l.50, l.278, l.3190). Unknown command: `<cmd>?\r\nch> ` (l.3208).
  - A complete exchange therefore looks like
    `sweep\r\n` → `sweep\r\n0 800000000 450\r\nch> ` — echo line, output, prompt.
  - Commands flagged `CMD_WAIT_MUTEX` (data, sweep, trace, rbw, attenuate, …) run in the sweep
    thread: they abort the running sweep and execute at the next point (l.3163-3180). If the
    command itself is aborted (touch on the screen), **no prompt is printed** (l.276) → the host
    times out; drain and continue. "Command timeout\r\n" after 4×5 s.
- **Identify**: `version` → `tinySA4_v1.4-217-gabcdef\r\nHW Version:V0.4.5.1 \r\n` (Ultra) or
  `tinySA_v1.4-…\r\n` (basic) (`cmd_version` l.2553; `VERSION="$(PROJECT)_$(GIT_VERSION)"`
  Makefile l.87, PROJECT `tinySA4`/`tinySA` l.122/124). HW text table l.2516: `V0.4.5.1` = ZS405,
  `V0.4.6` = ZS406 (Ultra+), `V0.5.4` = ZS407 (Ultra+). Model regexes:
  `^tinySA4_` + `HW Version:V0\.5` → Ultra+ ZS407, `HW Version:V0\.4\.6` → ZS406, else Ultra;
  `^tinySA_` → basic.

| Operation | Command | Reply (after echo line, before `ch> `) | Source |
|---|---|---|---|
| sweep settings | `sweep` | `<start Hz> <stop Hz> <points>` | main.c l.2052 |
| set start/stop/center/span | `sweep start <Hz>` / `stop` / `center` / `span`; `sweep cw <Hz>` = zero span | none | l.2071-2075 |
| set start+stop+points | `sweep <start> <stop> [points]` | none | l.2084 |
| frequencies | `frequencies` | one `%U` Hz per line, `points` lines | l.2434 |
| **trace** | `data 2` (measurement), `data 1` (stored), `data 0` (temp) | one `%e` per line (`-8.312e+01`), in the **current unit** (`trace dBm` default) | l.901-916 |
| trace (one shot sweep) | `scan <start> <stop> [points] [outmask]` (mask 1=freq, 2=meas, 4=stored) | lines `freq meas 0.000000 ` | l.1621-1670 |
| trace binary | `scanraw <start> <stop> [points] [opt]` | `{` + per point `x` MSB LSB + `}`; dBm = raw/32 − 128 (basic) / −174 (Ultra) | wiki; sa_cmd.c ~l.1300 |
| RBW | `rbw <kHz>` / `rbw auto`; query `rbw` | `usage: rbw 0.2..850|auto\r\n3.0kHz` (actual RBW, SI-formatted) | sa_cmd.c l.574-582 |
| attenuation | `attenuate 0..31` / `attenuate auto`; query `attenuate` | `usage: attenuate 0..31|auto\r\n10.00` | sa_cmd.c l.354-357 |
| sweep time | `sweeptime <s>` (0.003..60); query `sweeptime` | `usage: sweeptime 0.003..60\r\n0.120s` (actual) | sa_cmd.c l.386-390 |
| ref level / scale / unit | `trace reflevel <dBm>|auto`, `trace scale <dB>|auto`, `trace dBm|dBmV|dBuV|RAW|V|Vpp|W|dBV`; query `trace` | `1: dBm -10.000000 10.000000 ` (unit, ref level, scale per enabled trace) | main.c l.2173-2236 |
| markers | `marker` (all enabled), `marker 1` (select/show), `marker 1 <Hz>` (set by freq; a value ≤ points is an **index**), `marker 1 peak` (peak search, replies), `marker 1 on|off` | `<n> <index> <freq Hz> <value %.2e>` e.g. `1 225 433920000 -4.23e+01` | l.2304-2360 |
| run/hold | `pause`, `resume`, `status` → `Paused` / `Resumed` | | l.482-510 |
| single / wait | `waitscan [n]` (do n sweeps, return when done), `wait` | none (returns when complete) | l.513-560 |
| trigger | `trigger auto|normal|single|<dBm>` | | sa_cmd.c l.747 |
| generator | `mode low output`, `level <dBm>`, `output on|off` | | wiki; sa_cmd.c |
| screenshot | `capture` → raw RGB565 frame, 2 bytes/pixel (wiki: 320×240; Ultra display 480×320), no header | binary, not framed | wiki |

- **DSL fit (precise)**:
  - **Framing works today.** `rx.mode: terminator`, `rx.terminator: "ch> "` — the framer matches
    multi-byte terminators (`bm-driver/src/framing.rs` `find`, needle of any length). The reply
    then is `<echo>\r\n<output>`; strip the echo with
    `{regex: {pattern: "(?s)^[^\r\n]*\r\n(.*)$", group: 1}}` followed by `trim`.
    The prompt string `ch> ` never appears inside data (numbers, `%e`).
  - **Setters must read the reply**: every command answers at least `<echo>\r\nch> `, so setters are
    `reply: line` (no parse) — never `reply: none`, or the prompt lands on the next command. The
    parse of a setter reply is non-empty (the echo), so `on_empty` is never hit.
  - Usage-prefixed queries (`rbw`, `attenuate`, `sweeptime`) need `regex` on the last line
    (`(?s)\r\n([^\r\n]*)$`) and an SI suffix parse (`3.0kHz` → regex number + unit map, or CEL).
  - Two-word identify (`version` has two lines): `models.match` regex over the whole reply is fine.
  - **Trace blocked today**: `data 2` is newline-separated ASCII → needs `{numbers: {sep: "\n"}}`
    (whitespace-tolerant). `scanraw` is a 3-byte stride (`x`,MSB,LSB) inside `{…}` → not
    expressible by `samples` (would need a stride/skip option); prefer `data 2` (450 × ~11 bytes =
    5 KB, ~50 ms at USB FS).
  - Connect: when USB becomes active the firmware prints the banner `\r\ntinySA Shell\r\nch> `
    (main.c l.3726-3727) — possibly still buffered when the port opens, so the first exchange could
    end on the banner's prompt and shift every later reply by one. An empty line (`\r`) is answered
    with `<garbage>?\r\nch> ` (parceLine returns NULL, l.3085-3091, l.3208). Needs a
    **drain-before-identify** (flush input until idle, e.g. 200 ms). `Engine::open`
    (`bm-driver/src/engine.rs` l.130) runs identify first without draining → small interpreter
    addition: drain once before identify when `rx.drain_idle_ms` is set. `rx.drain_idle_ms: 100`.
  - Polling a `CMD_WAIT_MUTEX` command aborts the sweep in progress; with sweep times of 50-500 ms
    poll the trace no faster than the sweep time, and settings slowly (3 s).
- **Quirks**: frequency args accept `k`/`M`/`G` suffixes (wiki); `marker 1 <n>` treats n ≤ points
  as an index (send Hz ≥ 1 kHz to be safe); units change `data` values (keep `trace dBm` in
  `on_connect`); auto-refresh mode (`refresh on`) pushes binary `bulk`/`fill` frames — never enable.
- **Docs**: `/home/marek/project/BenchMeshDOC/tinysa_tinysa/doc/SOURCES.md` (wiki URL + firmware
  commit with line numbers). **Confidence high** (source code is the spec; widely used by
  tinySA-App, QtTinySA, python `tinySA.py`).

### 2.2 Rigol DSA800 (DSA815, DSA832, DSA832E, DSA875; -TG)

- **Models**: DSA815 9 kHz–1.5 GHz, DSA832(E) 9 kHz–3.2 GHz, DSA875 9 kHz–7.5 GHz; RBW 10 Hz–1 MHz
  1-3-10 (p128; EMI/QP: 200 Hz/9 kHz/120 kHz only); ref level −100…+20 dBm (p70); attenuation 0–30 dB (p160);
  sweep points 101–3001, default 601 (p170); sweep time 20 µs–7500 s (p170); TG −40…0 dBm (p180)
  on -TG models. Detector NEG/NORM/POS/RMS/SAMP/VAV/QPE (p143).
- **Interfaces**: USBTMC (VID 1ab1; PID not in the guide — match by VID + `*IDN?` like
  `rigol_dho800`), LAN (VXI-11; raw socket **port 5555**, p224), no RS-232.
- **Identify**: `*IDN?` → `Rigol Technologies,DSA875,DSA8A134400008,00.00.00.00.03` (p94 (2-72)).

| Operation | Command | Reply | Page |
|---|---|---|---|
| center | `:SENS:FREQ:CENT <Hz>` / `?` | integer Hz `1000000` | p148 |
| span / full / zero | `:SENS:FREQ:SPAN <Hz>` / `?`, `:SENS:FREQ:SPAN:FULL` | integer Hz | p151 |
| start / stop | `:SENS:FREQ:STAR`, `:SENS:FREQ:STOP` | integer Hz | p152-153 |
| RBW / auto | `:SENS:BAND:RES <Hz>` / `?`; `:SENS:BAND:RES:AUTO ON|OFF` | integer Hz; 0/1 | p128 |
| VBW / auto | `:SENS:BAND:VID <Hz>`; `:SENS:BAND:VID:AUTO` | integer Hz | p129 |
| ref level | `:DISP:WIN:TRAC:Y:SCAL:RLEV <dBm>` / `?` | `-1.000000E+01` | p70 |
| scale | `:DISP:WIN:TRAC:Y:SCAL:PDIV <dB>` (0.1–20) | sci | p70 |
| attenuation / auto | `:SENS:POW:RF:ATT <dB>` / `?`; `:SENS:POW:RF:ATT:AUTO` | integer dB | p160 |
| detector | `:SENS:DET:FUNC NEG|NORM|POS|RMS|SAMP|VAV|QPE` | | p143 |
| sweep time / auto | `:SENS:SWE:TIME <s>`; `:SENS:SWE:TIME:AUTO` | sci s | p170 |
| points | `:SENS:SWE:POIN <n>` (101–3001) | integer | p170 |
| continuous / single | `:INIT:CONT ON|OFF` / `?` (0/1); `:INIT` (single, ignored if CONT ON) | | p97-98 |
| sweep state | `:STAT:OPER:COND?` bit 3 = SWEeping (8), bit 4 = MEASuring | integer | p185 |
| marker X | `:CALC:MARK1:X <Hz>` / `?` (also enables marker) | integer Hz | p48 (2-26) |
| marker Y | `:CALC:MARK1:Y?` | `5.960000E+00` dBm | p55 (2-33) |
| marker on | `:CALC:MARK1:STAT ON|OFF` | 0/1 | p45 |
| peak search | `:CALC:MARK1:MAX:MAX`; next `:CALC:MARK1:MAX:NEXT`; mkr→CF `:CALC:MARK1:SET:CENT` (p42), peak→CF `:CALC:MARK1:PEAK:SET:CF` (p41) | no reply | p38 |
| trace format | `:FORM:TRAC:DATA ASCii|REAL,32` (default ASCii) | | p86 (2-64) |
| **trace** | `:TRAC:DATA? TRACE1` | **ASCII with IEEE block header**: `#9000009014 -1.390530e+01, -7.108871e+01, …` (", " separated, 601 points), dBm | p210-211 (2-188) |
| trace mode | `:TRAC1:MODE WRIT|MAXH|MINH|VIEW|BLAN|VID|POW` | `WRIT`… | p217 |
| unit | `:UNIT:POW DBM|DBMV|DBUV|V|W` | | p221 |
| TG output / level | `:OUTP:STAT ON|OFF` (-TG only), `:SOUR:POW:LEV:IMM:AMPL <dBm>` | | p110, p180 |

- **DSL fit**: settings, markers, peak search, sweep control fit today (`link.usbtmc`,
  `rx.mode: message`, like `rigol_dho800`). Trace: `reply: block` already strips `#9000009014`;
  the payload is ASCII → needs `{numbers: {sep: ","}}`. Alternative `REAL,32` + `samples: f32le`
  (does not exist). Screenshot: not documented in the guide (forum: `:PRIV:SNAP? BMP`) — omit.
- **Quirks**: ASCII trace still carries a block header (unlike RSA3000, §2.7) — must be
  `reply: block`; `:INIT` is ignored in continuous mode; frequencies queried as integers.
- **Docs**: `/home/marek/project/BenchMeshDOC/rigol_dsa800/doc/DSA800_ProgrammingGuide_EN.pdf`
  (Aug 2016, 251 pp.). **Confidence high** — complete, consistent guide; Rigol SCPI parser
  behaviour known from `rigol_dho800`.

### 2.3 Siglent SSA3000X, SSA3000X Plus, SSA3000X-R, SVA1000X, SSA5000A

Docs (`/home/marek/project/BenchMeshDOC/…/doc/`): **[E04A]** `siglent_ssa3000x/SSA3000X_ProgrammingGuide_PG0703X_E04A.pdf`
(older E03D alongside); **[P02B]** `siglent_ssa3000x_plus/ProgrammingGuide_PG0703P_E02B.pdf` (one
guide for Plus, -R and SVA1000X, copied into `siglent_ssa3000x_r/` and `siglent_sva1000x/`; older
E01A copies too); **[UM]** `siglent_ssa5000/SSA5000A_UserManual_EN02A.pdf` (SCPI inside the user
manual) + datasheet **[DS]**.

- **Models**: SSA3021X/3032X (9 kHz–2.1/3.2 GHz, RBW 10 Hz–1 MHz 1-3-10 [E04A] p31-32, VBW 1 Hz–1 MHz
  p32, ref level −100…+30 dBm p27, att 0–50 dB p27, preamp `:POW:GAIN`); Plus (SSA3015X+/3021X+/3032X+/
  3050X+/3075X+, RBW 1 Hz–1 MHz [P02B] p45, att 0–51 dB p34, 8 markers p52-55); SSA3000X-R (adds
  real-time mode, [P02B] ch. 8 p149-160); SVA1015X/1032X (SA + VNA, `:INST VNA`, [P02B] p94-103);
  SSA5083A/5085A (9 kHz–13.6/26.5 GHz [DS] p8, RBW 1 Hz–10 MHz, ref −170…+23 dBm [UM] p61, att 0–50 dB
  even [UM] p62, points 201–10001 [UM] p58). Trace points on SSA3000X/Plus: 751 (not stated in the
  guides; used by LibreVNA's SSA3000X driver — confirm with `:SWE:POIN?`). TG on -TG models.
- **Interfaces**: USBTMC (VID **f4ec**; PID not documented — 0x1305 reported for the Plus family on
  EEVblog, low confidence), LAN raw socket **5025**, telnet 5024 with `>>` prompt ([E04A] p7, [P02B] p9,
  [UM] p363). No RS-232.
- **Identify**: `*IDN?` → `Siglent Technologies,SSA3032X,<serial>,<fw>` (manual examples show
  `SSA3032` [E04A] p14 and `Siglent,SVA1015,…` [P02B] p17 — real units report the X/X-Plus name; match
  with `^Siglent[^,]*,(SSA3\d{3}X|SVA10\d{2}X|SSA50\d{2}A)`).

| Operation | Command | Reply | Page |
|---|---|---|---|
| center/span/start/stop | `[:SENS]:FREQ:CENT|SPAN|STAR|STOP <f>[Hz|kHz|MHz|GHz]` / `?` | float Hz | [E04A] p23-24; [UM] p49-50 |
| RBW / auto | `[:SENS]:BWID[:RES] <Hz>`, `:BWID:AUTO ON|OFF` | | [E04A] p31-32; [P02B] p45; [UM] p54 |
| VBW / ratio | `[:SENS]:BWID:VID <Hz>`, `:BWID:VID:RAT` | | [E04A] p32 |
| ref level | `:DISP:WIND:TRAC:Y:RLEV <dBm>` | | [E04A] p27; [P02B] p34 |
| attenuation / auto / preamp | `[:SENS]:POW:ATT <dB>`, `:POW:ATT:AUTO`, `:POW:GAIN ON|OFF` | | [E04A] p27 |
| detector | `[:SENS]:DET:TRAC1 NEG|POS|SAMP|AVER|NORMAL|QUASi` | | [E04A] p36; [P02B] p50; [UM] p77 |
| sweep time | `[:SENS]:SWE:TIME <s>` (917 µs–1000 s), `:SWE:TIME:AUTO` | | [E04A] p38-39; [UM] p59 |
| continuous / single | `:INIT:CONT ON|OFF`, `:INIT[:IMM]`; Plus also `:INIT:REST|PAUS|RES` | | [E04A] p22; [P02B] p41-42; [UM] p60 |
| sweep done | `:TRAC:SWE:STAT?` → 1/0 (SSA3000X only); else `*OPC?` after `:INIT` | | [E04A] p34, p15 |
| marker on / X / Y | `:CALC:MARK1:STAT ON`, `:CALC:MARK1:X <f>` / `?` (integer Hz), `:CALC:MARK1:Y?` (dBm) | | [E04A] p41-45; [UM] p84-85 |
| peak search | `:CALC:MARK1:MAX`, `:MAX:NEXT|LEFT|RIGH` | | [E04A] p49; [P02B] p60; [UM] p93 |
| format | `:FORM ASCii|REAL` (E04A default REAL!); Plus `ASCii|REAL,32|REAL,64` (default ASC, `REAL` = 64-bit); SSA5000 `ASCii|REAL32|REAL` (REAL = 64-bit) | | [E04A] p34; [P02B] p48; [UM] p74 |
| **trace** | `:TRAC[:DATA]? 1` (Plus also `TRACE1`, SSA5000 `:TRAC1:DATA?`) | ASCII comma list **with a trailing comma**, no header (LibreVNA driver drops the last empty field); binary = IEEE block | [E04A] p34; [P02B] p48; [UM] p74 |
| trace mode | `:TRAC1:MODE WRIT|MAXH|MINH|VIEW|BLAN|AVER`; **SSA5000: `:TRAC1:TYPE`** | | [E04A] p33; [UM] p75 |
| TG | `:OUTP ON|OFF`, `:SOUR:POW <dBm>` (0…−20) | | [E04A] p68-69; [P02B] p90-91 |
| screenshot | `:HCOP:SDUM:DATA?` → BMP (SSA3000X only); others `:MMEM:STOR PNG,"f"` to internal storage | block | [E04A] p21; [P02B] p26 |
| mode | `:INST SA|CAT` (E04A); `:INST SA|MA|DTF|VNA` (Plus/SVA) | | [E04A] p21; [P02B] p28 |

- **DSL fit**: settings/markers today (USBTMC, `rx.mode: message`). Trace: ASCII `numbers` step
  (must ignore the **trailing comma** / empty last field); send `:FORM ASC` in `on_connect` because
  the default differs between guides. SVA1000X VNA mode → VNA class (`:CALC:DATA:FDAT?` pairs, [P02B]
  p102-103).
- **Quirks**: format default and `REAL` width differ per document (always set explicitly); newer
  guides drop `:TRAC:SWE:STAT?` → use `*OPC?`; RTSA selection on -R not documented.
- **Confidence**: high for SSA3000X/Plus/SVA (good guides, LibreVNA/QMI/pysiglent clients exist),
  medium-high for SSA5000A (commands scattered in a 402-page manual, copied examples).

### 2.4 UNI-T UTS1000B/T, UTS3000A/B/T, UTS3000T+ (and UTS5000A)

- **Models**: UTS1015B/1032B (≤ 3.2 GHz, 4 traces, ≤ 10001 points), UTS3021B/3036B/3084B (≤ 8.4 GHz,
  6 traces, ≤ 40001 points), UTS5026A (≤ 100001 points) (p107, p117); -T = tracking generator.
  RBW 1 Hz – max, 1-3-10 (p80).
- **Interfaces**: USBTMC and LAN (SCPI via `:SYST:COMM:LAN:*`, p10-11); vid:pid not documented
  (UNI-T USBTMC instruments commonly 5345:xxxx or 6656:xxxx — verify on first contact).
- **Identify**: `*IDN?` → `UNI-T Technologies, UTS3036B, 000000001, 00.00.01` (p7; note the spaces
  after the commas).

| Operation | Command | Reply | Page |
|---|---|---|---|
| center / span / start / stop | `[:SENS]:FREQ:CENT <freq>` / `SPAN` / `STAR` / `STOP` | sci Hz | p93-95 |
| RBW / auto | `[:SENS]:BAND[:RES] <Hz>` / `:AUTO ON|OFF` | | p80-81 |
| VBW / auto | `[:SENS]:BAND:VID <Hz>` / `:AUTO` | | p81-82 |
| ref level | `:DISP:WIND:TRAC:Y[:SCAL]:RLEV <dBm>` | `-1.000000e+01` | p51 |
| attenuation / auto | `[:SENS]:POW[:RF]:ATT <dB>` / `:AUTO` | | p98, p104 |
| detector | `[:SENS]:DET[:FUNC]`, per trace `[:SENS]:DET:TRAC<n>` | | p91-92 |
| sweep time / points | `[:SENS]:SWE:TIME <s>` / `:AUTO`; `[:SENS]:SWE:POIN <n>` | | p107-108 |
| continuous | `:INIT:CONT ON|OFF` / `?` | 0/1 | p62 |
| done | `*OPC?` | 1 | p7 |
| marker X / Y | `:CALC:MARK<n>:X <freq>` / `?` → `1.000000e+09`; `:CALC:MARK<n>:Y?` → `-5.000000e+01` | | p41-43 |
| peak search | `:CALC:MARK<n>:MAX[:MAX]`, `:MAX:NEXT`, `:MAX:LEFT|RIGH` | | p36-37 |
| trace format | `:FORM[:TRAC][:DATA] ASCii|REAL32|REAL64` (default ASCii, query `ASC8`) | | p62 |
| **trace** | `:TRAC<n>:DATA?` | **ASCII with IEEE block header**: `#514013-5.948681e+01,-7.703760e+01,…` (comma, no space), dBm | p117-118 |
| trace type | `:TRAC<n>:TYPE WRIT|AVER|MAXH|MINH` | | p118 |
| TG | `:OUTP[:EXT][:STAT] ON|OFF`; `:SOUR:POW <dBm>` | | p66, p115 |

- **DSL fit**: as DSA800 — `reply: block` + `numbers`. Large point counts (40001 × 14 bytes ≈
  560 KB) need `max_bytes`, `timeout_ms` and a capped `:SWE:POIN` in `on_connect` (e.g. 1001).
- **Quirks**: manual typos (`ASCll`; "REAL32: 32-bit binary system integer" although IEEE 754, p62/117;
  `:INIT:CONT` description lists `1|ON` twice). One manual for four series — per-model limits
  scattered in prose.
- **Docs**: `/home/marek/project/BenchMeshDOC/unit_uts/doc/UTS_Spectrum_Analyzer_Programming_Manual_V1.0.pdf`
  (245 pp.). **Confidence medium-high** (complete command set, sloppy wording, no UNI-T SA on
  any bench we know of).

### 2.5 NanoVNA-H / H4 / NanoVNA (edy555 lineage, NanoVNA-D and hugen firmware), NanoVNA-F

- **This is a VNA, not a spectrum analyzer** — recommendation in §6. Documented here because the
  transport/framing is identical to tinySA and a VNA definition can reuse it.
- **Interface**: USB CDC 0483:5740 (`usbcfg.c` l.45-46) — identical to tinySA.
- **Framing**: same ChibiOS shell: echo, `\r\n`, prompt `ch> ` (`main.c` l.44, l.3198-3205).
- **Identify**: `version` returns only the git tag (`1.2.40`) — useless for models; use `info`
  (first line `Board: NanoVNA-H 4` / `Board: NanoVNA`, l.131-132).
- **Commands**: `sweep` → `start stop points` (l.1614); `sweep <start> <stop> [points]`;
  `frequencies`; `data 0` (S11) / `data 1` (S21) → one `"%f %f"` (real imag) per point (l.766);
  `scan <start> <stop> [points] [outmask]` (outmask 1 freq, 2 S11, 4 S21, 0x80 binary float32)
  (l.1384-1452); `marker`, `pause`/`resume`, `capture`, `power`, `bandwidth` (IF BW), `cal`.
  Points max 401 (H4, F303) / 101 (H, F072) (`nanovna.h` l.139/160).
- **NanoVNA-F (BH5HNU, V2/V3)**: NanoVNA-Saver-compatible text shell of the same lineage
  (not independently verified here — confidence medium).
- **DSL fit**: framing today; `data 0` needs `numbers` (interleaved re/im). The binary
  `scan … 0x80` needs `samples: f32le` + header skip → not today.
- **Docs**: `/home/marek/project/BenchMeshDOC/nanovna_nanovna/doc/SOURCES.md`. **Confidence high**.

### 2.6 RF Explorer (3G Combo, 6G Combo, WSUB1G, ISM Combo, PLUS models)

- **Interface**: USB-serial (Silicon Labs CP210x; 10c4:ea60 is the chip default — to confirm),
  **500000 baud** (`RFExplorer-for-Python` examples, `BAUDRATE = 500000`; 2400 is the alternative
  device setting). 8N1, no flow control.
- **Protocol**: commands framed as `#` + one **binary length byte** (`len(cmd)+2`) + command
  (`RFExplorer.py` l.1681-1688), e.g. `#\x04C0` (request config + start streaming), `#\x04CH`
  (hold), `#\x05C+\x04` (max-hold calculator), `#\x05CJ<n>` (points, binary byte),
  `C2-F:sssssss,eeeeeee,ttt,bbb` (start/end kHz, top/bottom dBm). Replies: `#C2-F:…` config line;
  **sweeps are pushed continuously** as `$S` + binary count + one byte per point (dBm = −byte/2)
  + `\r\n` (`RFESweepData.py` l.101-113); the payload bytes may contain `\r\n`.
- **DSL fit: blocked.** (a) unsolicited streaming — no request/response; (b) binary length
  prefix inside sweep frames that may contain the terminator; (c) binary bytes in the command
  (a YAML `\x04` escape could produce them, but numeric params cannot be encoded as raw bytes).
  Would need a "stream" link mode with frame parser — not worth it now.
- **Docs**: `/home/marek/project/BenchMeshDOC/rfexplorer_rfexplorer/doc/SOURCES.md` (library source;
  UART API spec on the vendor wiki). **Confidence medium.**

### 2.7 Rigol RSA3000 (RSA3015/3030/3045, E/N variants) and RSA5000 (RSA5032/5065)

- **Models**: RSA3015/3030/3045 9 kHz–1.5/3/4.5 GHz; RSA5032/5065 to 3.2/6.5 GHz. Two modes:
  `:INST:SEL SA|RTSA` (wait ~8 s after switching) (p107). Points 101–10001, default 801 (p178);
  sweep time 1 ms–4000 s (p178).
- **Interfaces**: USBTMC (VID 1ab1), LAN, raw socket **port 5555** (p232; RSA5000 p236).
- **Identify**: `*IDN?` → `Rigol Technologies,RSA3045,RSA3A200100001,00.01.00` (p101 (2-83)).
- **Commands**: same tree as DSA800 (`[:SENS]:FREQ:*`, `[:SENS]:BAND[:RES]`,
  `:DISP:WIN:TRAC:Y[:SCAL]:RLEV`, `[:SENS]:POW[:RF]:ATT` p169, `[:SENS]:DET:TRAC<n>` p151,
  `:CALC:MARK<n>:X` p53, `:CALC:MARK<n>:Y?` p55, `:INIT:CONT` p105, `:STAT:OPER:COND?` p189,
  `:MMEM:STOR:SCR` p113). Differences:
  - `:FORM[:TRAC][:DATA] ASCii|INTeger,32|REAL,32|REAL,64`, query → `ASC,8` (p98).
  - `:TRAC[:DATA]? TRACE1..TRACE6` — the ASCII example has **no block header**:
    `-1.390530e+01, -7.108871e+01, …` up to 801 points (p206-207). Binary: `#9…` block.
  - `:TRAC<n>:MODE|TYPE WRIT|AVER|MAXH|MINH` (p212).
- **DSL fit**: as DSA800 but trace via `reply: line`/`message` + `numbers` (header presence must be
  confirmed live — make `numbers` tolerate an optional `#<n><len>` prefix, §5).
- **Docs**: `/home/marek/project/BenchMeshDOC/rigol_rsa3000/doc/RSA3000_ProgrammingGuide_EN.pdf`
  (Jan 2018, 264 pp.), `/home/marek/project/BenchMeshDOC/rigol_rsa5000/doc/RSA5000_ProgrammingGuide_EN.pdf`
  (Jan 2018, 268 pp., same layout: FORMat p98, *IDN? p101, POINts p179, TRACe p210).
  **Confidence high** (doc); low prevalence among hobbyists.

### 2.8 GW Instek GSP-9300 / GSP-9300B / GSP-9330 and GSP-818

Docs: **[G93]** `/home/marek/project/BenchMeshDOC/gwinstek_gsp9300/doc/201501_PM_GSP-9300_VB_E.pdf`
(+ `201701_PM_GSP-9300B_VA_E.pdf`, `201707_PM_GSP-9330_VD_E.pdf`, same command set);
**[G818]** `/home/marek/project/BenchMeshDOC/gwinstek_gsp818/doc/GSP-818_Programming_Manual_EN_Ver_B_20230111.pdf`
(+ user manual, datasheet `DS_GSP-818_E.pdf`).

**GSP-9300 family** — 9 kHz–3 GHz (9330: 3.25 GHz), VBW 1 Hz–1 MHz, att 0–50 dB/1 dB ([G93] p10),
**601 points** (p219), TG option −50…0 dBm (p13, p18). Interfaces: **RS-232 8N1, 300–115200 baud,
RTS/CTS** (p12, p37), USBTMC (p28), LAN/LXI HiSLIP 4880 (p31), optional GPIB; no raw socket port and no
vid:pid documented. Terminator LF (p55). `*IDN?` → `GWINSTEK,GSP9300,XXXXXXXX,T.X.X.X.X` (p38; p77 shows
`GSP-9300` — regex `GSP-?9300`), 9300B `GWINSTEK,GSP9300B,…`.

| Operation | Command | Reply | Page |
|---|---|---|---|
| center/span/start/stop | `:FREQ:CENT|SPAN|STAR|STOP <NRf>` | NR3 Hz | [G93] p163-165 |
| RBW / auto, VBW | `:BAND <f>`, `:BAND:AUTO`, `:BAND:VID` | | p154-155 |
| ref level | `:DISP:TRAC:Y:RLEV` | | p130 |
| attenuation | **`:INP:ATT`** 0–50 | | p134 |
| detector | `:DET AVER|SAMP|POS|NEG|NORM` | | p161 |
| sweep time | `:SWE:TIME` | | p191 |
| continuous / single / state | `:INIT:CONT`, `:INIT`; `:STAT:OPER:COND?` bit 3 (8) = sweeping; `*OPC?` | | p133, p205, p77 |
| markers | `:CALC:MARK1:STAT`, `:CALC:MARK1:X`, `:CALC:MARK1:Y?` (NR3); peak `:CALC:MARK1:PEAK MAX|MIN|NEXT|RIGH|LEFT`; `:CALC:MARK:PEAK:DATA?` (10 peaks CSV) | | p106, p108, p102-103 |
| **trace** | `:TRAC? TRAC1` → 601 NR3 values, ASCII CSV, no binary option | | p219-220 |
| trace mode | `:DISP:TRAC1:MODE WRIT|VIEW|BLAN|MAXH|MINH` (**set only**) | | p127 |
| TG | `:OUTP`, `:SOUR:POW <ampl>` | | p144, p193 |
| screenshot | `:MMEM:STOR:SCR x.jpg` (to storage only) | | p142 |

**GSP-818** — 9 kHz–1.8 GHz, RBW 10 Hz–500 kHz (1-10) + 1/3 MHz, VBW 10 Hz–3 MHz, ref −80…+30 dBm,
att 0–40 dB ([G818DS] p1; [G818] p90), TG option 100 kHz–1.8 GHz −30…0 dBm (p94). LAN + USB device (p12),
no RS-232. LF terminator (p23). `*IDN?` → `GWINSTEK, GSP-818, GSPXXXXXX, VX.X.X` (p31); **no `*OPC?`, no
status registers**. Commands: `:FREQ:CENT|SPAN|STAR|STOP` (p78-81), `:BAND[:AUTO]`, `:BAND:VID` (p69-71),
`:DISP:WIN:TRAC:Y:RLEV` (p55), `:POW:ATT` (p90), `:DET AUTO|NORM|POS|NEG|SAMP` (p77), `:SWE:TIME` (set
in **ns**, reply in **ms**!) and `:SWE:POIN` (p92), only `[:INIT]:CONT` (no single, p60), peak
`:CALC:MARK1:MAX` (p37), `:CALC:MARK1:X?` → `300000000Hz`, `:Y?` → `9.8dBm`, inactive marker → `ERR`
(p44-45), `:TRAC? TRACE1` ASCII CSV (p100), `:TRAC1:MODE` (p100-101), TG `:OUTP:TRAC`, `:SOUR:POW:TRAC`
(p65, p94).

- **DSL fit**: GSP-9300 is the only SA family here usable over **RS-232** today
  (`link.serial: {baud: 115200, format: 8N1, flow: rts_cts}`, `rx.terminator: "\n"`); settings and
  markers today, trace needs `numbers`. GSP-818: unit-suffixed replies need `regex` before `number`,
  `null_if: [ERR]` for inactive markers; sweep-time unit mismatch handled with `from`/CEL scaling.
- **Confidence**: GSP-9300 high (complete manual incl. RS-232 and status registers);
  GSP-818 medium (sparse, odd units). No open-source clients found.

### 2.9 Keysight N9320B, N9322C (BSA), FieldFox (N99xxA/B SA mode)

Docs: `/home/marek/project/BenchMeshDOC/keysight_n9320b/doc/N9320B_Programmers_Guide_9018-02757.pdf`
**[PG20]** (+ user's guide **[UG20]**); `/home/marek/project/BenchMeshDOC/keysight_n9322c/doc/N9322C_Programmers_Guide_N9322-90023.pdf`
**[PG22]** (xdevs mirror of the Agilent guide, FW A.04.20) + `N9321C-24C_Users_Guide_9018-03838.pdf`;
`/home/marek/project/BenchMeshDOC/keysight_fieldfox/doc/FieldFox_Programming_Guide_FFProgrammingHelp.pdf`
**[FF]** (858 pp., 2022) + A-Series user's guide **[FFUG]**.

| | N9320B | N9322C | FieldFox SA |
|---|---|---|---|
| Range | 9 kHz–3 GHz ([UG20] p10) | 9 kHz–7 GHz (N9321C 4 GHz … N9324C 20 GHz) | 9 kHz–4…50 GHz by model ([FFUG] p35) |
| RBW / VBW | 10 Hz–3 MHz / 1 Hz–3 MHz ([PG20] p151) | 10 Hz–3 MHz 1-3-10 ([PG22] p88) | 10 Hz–2 MHz / 1 Hz–2 MHz ([FF] p568-569) |
| Ref level / att | −100…+50 dBm (p112) / 0–70 dB (p156) | −140…+20 dBm / 0–50 dB (p83) | – / 0–30 dB in 5 dB ([FFUG] p174) |
| Points | not documented | not documented | 2–10001, SA default 401 ([FF] p736) |
| TG | option TG3 | option TG4/TG7 | independent source/TG option, −45…0 dBm ([FF] p758) |
| USB | USBTMC **0957:2118** (`USB0::2391::8472`, [PG20] p48, p53; rear USB-B) | USBTMC 0957:2118 ([PG22] p42, p47) | mini-USB only on N995x/N996x and newer N991x–N993x (serial ≥ 5607, [FFUG] p53, p843); vid:pid not documented |
| LAN | socket 5025, telnet 5025 with `SCPI>` prompt ([PG20] p13-16) | VISA TCPIP, port not stated | socket 5025, telnet 5024 (no binary blocks on telnet) ([FF] p42, p451) |
| `*IDN?` | `Keysight Technologies, N9320B, 45310116, A.01.02` (p67) | `Agilent Technologies, N9322C, 45310116, A.04.20` (p59) | no example |

| Operation | N9320B [PG20] | N9322C [PG22] | FieldFox [FF] |
|---|---|---|---|
| mode | – | TG needs `INST TGEN` (p77, p220) | `INST "SA";*OPC?` (case-sensitive, overlapped, p460-461) |
| freq | `[:SENS]:FREQ:CENT|SPAN|STAR|STOP`, `:SPAN:FULL` (p154-156) | same + `SPAN:ZERO|PREV` (p79-82) | same + MIN/MAX (p636-642) |
| RBW / VBW | `BAND[:RES]`, `BAND:AUTO`, `BAND:VID[:AUTO|:RAT]` (p151-152) | same (p88-89) | `BAND[:RES]`, `BAND:AUTO 0|1`, `BAND:VID` (p568-569) |
| ref level | `DISP:WIND:TRAC:Y[:SCAL]:RLEV` (p112) | same (p83) | same, all traces (p438) |
| attenuation / preamp | `POW[:RF]:ATT`, `:AUTO`; `POW:GAIN ON` (opt PA3) (p156-157) | `POW:ATT`, `:AUTO` (p83) | `POW:ATT` (sets AUTO OFF), `POW:ATT:AUTO` (p701) |
| detector | `DET[:FUNC] NEG|POS|SAMP|AVER|RMS|NORMAL` (p153) | per trace `DET:TRAC1[:FUNC]` (p92) | `DET:FUNC AUTO|NORM|POS|NEG|SAMP|AVER` (p632) |
| sweep time | `SWE:TIME`, `:AUTO` (p158) | `SWE:TIME`, `:AUTO`, `SWE:SPE` (p94) | non-zero span: relative `SWE:ACQ 1..5000` (p734); `SWE:TIME` zero span only (p738); actual `SWE:MTIM?` |
| cont / single / done | `INIT:CONT`, `INIT`; `*OPC?` valid only right after `INIT` (p67-68, p117) | `INIT:CONT`, `INIT`, `*OPC?` (p59, p109) | `INIT:CONT 0;*OPC?`, `INIT;*OPC?` (overlapped, p453) |
| marker X / Y | `CALC:MARK1:X`, `:Y?` (p85, p87) | `CALC:MARK1:X`, `:Y?` (p107-108) | `CALC:MARK1:X`, `:Y?` (no `[:SEL]` node in SA, p323) |
| peak | `CALC:MARK1:MAX[:NEXT|LEFT|RIGH]` (p78) | `CALC:MARK1:MAX` (p103) | `CALC:MARK1:FUNC:MAX` (p310) |
| **trace** | `TRAC[:DATA]? TRACE1` (p169) — **no FORMat subsystem, reply format undocumented** | `TRAC? TRACE1` (p91); `FORM ASC|REAL` (ASC comma list; REAL 4-byte floats) | `TRAC1:DATA?` comma list (p801); `FORM ASC,0|REAL,32|REAL,64` (block) (p451-452); `FORM:BORD NORM` = LSB first (p450) |
| trace mode | `TRAC1:MODE WRIT|MAXH|MINH|VIEW|BLAN` (+AVER, p169-171) | `TRAC1:MODE …`, `AVER:TRAC1:COUN` (p90, p93) | `TRAC1:TYPE CLRW|MAXH|MINH|AVG|VIEW|BLAN` (p827) |
| TG | `OUTP`, `SOUR:POW` (p123, p159-161) | `OUTP`, `SOUR:POW` 0…−30 dBm (p220-221) | `SOUR:ENAB`, `SOUR:TRAC TSWP`, `SOUR:POW` (p753-763) |
| screenshot | `MMEM:STOR:SCR` to storage only (p122) | `MMEM:STOR JPG,"…"` + `MMEM:DATA?` block (p74) | `MMEM:STOR:IMAG "x.png"` + `MMEM:DATA? "x.png"` (p487, p468) |

- **DSL fit**: N9320B/N9322C: USBTMC today (vid:pid known); trace needs `numbers` (N9320B: format to
  observe on the bench). FieldFox: mostly **LAN-only** → needs the TCP transport (`link.tcp: {port: 5025}`);
  trace `numbers`; long `*OPC?` after `INIT` → per-command `timeout_ms`.
- **Confidence**: FieldFox high (thorough guide), N9320B/N9322C medium (gaps: trace format, points, LAN port).

### 2.10 R&S FPC1000 / FPC1500 and Spectrum Rider FPH

Docs: `/home/marek/project/BenchMeshDOC/rs_fpc/doc/FPC_UserManual_en_13.pdf` **[FPC]** (426 pp., FW 1.90,
remote commands inside); `/home/marek/project/BenchMeshDOC/rs_fph/doc/FPH_User_Manual_en_21.pdf` **[FPH]**
(553 pp., FW 2.70).

- **Models**: FPC1000/FPC1500 (5 kHz–1 GHz, key-code upgradable to 2/3 GHz — range "per datasheet",
  [FPC] p255-256; RBW 1 Hz–3 MHz 1-3 + 200 kHz p264; VBW 1 Hz–3 MHz p265; att 0–40 dB/5 dB p260;
  tracking generator/signal source FPC1500 only p14). FPH 02/06/13/23/26/36: 2…26.5 GHz (+upgrades to
  31 GHz), model 23 with TG ([FPH] p179); RBW/VBW 1 Hz–3 MHz (p348); att 0–40 dB (p343); **711 points**
  (p358).
- **Interfaces**: **LAN (VXI-11)** ([FPC] p200-201, [FPH] p315); the USB-B port is **not USBTMC** — it is
  a network link via the InstrumentView driver to fixed IP 172.16.10.10 ([FPC] p201, [FPH] p316); raw
  port 5025 not documented. Wi-Fi option (FPC).
- **Identify**: `Rohde&Schwarz,FPC1000,1328.6660K02/xxxxxx,1.00` ([FPC] p33, p206);
  `Rohde&Schwarz,FPH,1321.1111.02/100001,V2.21` ([FPH] p334; `SYST:FORM:IDEN LEG|NEW` p463).

| Operation | FPC [FPC] | FPH [FPH] |
|---|---|---|
| mode | `INST[:SEL] SPECtrum|…` (example uses `INST SAN`, p236) | `INST SAN|PM|…` (*RST documented as PM! p452) |
| freq | `FREQ:CENT|SPAN|STAR|STOP`, `SPAN:FULL` (p252-256) | same; CENT default unit MHz (p337-339) |
| RBW / VBW | `BAND[:RES]`, `BAND:AUTO`, `BAND:VID[:AUTO]` (p264-265) | same; **default unit GHz** — always send units (p348) |
| ref level / att | `DISP:TRAC:Y:RLEV` (p258); **`INP:ATT`**, `INP:ATT:AUTO` (p260) | `DISP:TRAC:Y:RLEV` (p341); `INP:ATT[:AUTO]` (p343) |
| detector | `DET POS|NEG|SAMP|RMS|AVER|APE` (p274) | same (p354) |
| sweep | `SWE:TIME[:AUTO]`, `SWE:COUN` (p266-267); `INIT:CONT OFF`, `INIT;*OPC?` (sync in single sweep only, p267-268) | same (p349-351) |
| marker X / Y / peak | `CALC:MARK1:X` (p284), `:Y?` (single sweep + sync, p287), `:MAX` (p283) | p365, p367, p364 |
| **trace** | `TRAC? TRACE1` → **1183** values (one per pixel), x via `TRAC:DATA:X?` (p275-276); `FORM ASC|REAL,32`, *RST ASC (example says 631 points, p274) | `TRAC? TRACE1` → 711 values (p357-358); `FORM ASC|REAL`; `FORM:BORD` *RST SWAP = little endian (p356-357) |
| trace mode | `DISP:TRAC1:MODE WRIT|AVER|MAXH|MINH|VIEW|BLAN` (p272-273) | `DISP:TRAC:MODE WRIT|AVER|MINH|MAXH|VIEW` (p356) |
| TG | `SOUR:TG ON`, `SOUR:TG:AUTO ON`, `SOUR:TG:POW` (p230-232) | `SOUR:TG[:STAT]`, `:POW` 0…−40 dBm, `:ATT` (p345-347) |
| screenshot | **`DISP:FETC?`** → JPG binary directly (p220) | `DISP:WIND:STOR 'x.png'` + `MMEM:DATA?` (p441, p446) |

- **DSL fit**: **blocked on TCP** (no USBTMC at all); otherwise SCPI like the baseline, trace via
  `numbers`. Mode `INST` must be set in `on_connect`.
- **Confidence**: medium-high (complete reference, but point counts and defaults contradict
  themselves in places).

### 2.11 Hantek HSA2000 (HSA2016/HSA2030 A–E) and OWON XSA1000 / XSA800

**Hantek HSA2000** — doc **[HSA]** `/home/marek/project/BenchMeshDOC/hantek_hsa2000/doc/HSA2000_User_Manual_V1.0.1_EN.pdf`
(SCPI = chapter 5, 15 pages). 9 kHz–1.6/3.2 GHz, TG on B/C/D variants (p6), RBW 10 Hz–1 MHz 1-3-10
(p65), VBW 1 Hz–1 MHz, **461 points**, ref −100…+30 dBm, att 0–51 dB (p66), 4 traces/4 markers. USB
device + LAN (p14, p37), no port/vid:pid/LAN details. `*IDN?` fields only, no example (p49). Commands
(printed with stray spaces): `SENS:FREQ:STAR|CENT|STOP|SPAN <Hz>` integer Hz (p56-57),
`SENS:BAND:RES[:AUTO]`, `:VID[:AUTO]` (p58-59), `SENS:POW:RF:ATT[:AUTO]` (p59-60), `SENS:SWE:TIME?`
query-only in µs (p57), `INIT:CONT`, `INIT:IMM` (p60-61), detector (p57-58), `TRAC[:DATA]:TRACE<n>?`
(p54), `FORM:TRAC:DATA ASCii|REAL` (p55), `TRACE<n>:MODE` (p54), `CALC:MARK:CUR 0-3`,
`CALC:MARK:MAX:MAX|NEXT|LEFT|RIGH` (p50-51). **No marker X/Y readout** (only the counter
`CALC:MARK:FCO:X?`), no TG, `*OPC`, status or screenshot commands. **Confidence low**; DSL fit like the
SCPI families (trace via `numbers`), marker readout must be computed client-side from the trace.

**OWON XSA1000 / XSA800 / XSA3000-R** — no SCPI/programming manual is published on owon.com.hk
(download centre lists only user manuals, quick guides and PC software, checked 2026-10-18), and the
OWON file server was unreachable for direct probes. The PC software (`OWON_Spectrum_pco_sofware.zip`)
proves a USB/LAN protocol exists. **Not researchable from documents now** — candidate for a
wire capture (`scripts/wire_capture/`) when a community member has one. No local docs.

### 2.12 LiteVNA 62/64/67, NanoVNA V2 / S-A-A-2(N)

- **Protocol**: binary register protocol over USB CDC, "same as SAA2" (LiteVNA guide p26-30):
  opcodes `0x0d` INDICATE (→ `'2'`), `0x10/11/12` READ1/2/4, `0x18 AA NN` READFIFO, `0x20-0x23`
  WRITE1/2/4/8 (no reply), registers little-endian: `0x00` sweepStartHz u64, `0x10` sweepStepHz u64,
  `0x20` sweepPoints u16, `0x30` valuesFIFO (32-byte records of int32 fwd/rev re/im + freq index),
  `0xf0` deviceVariant (0x02 = LiteVNA). Writing sweep registers takes the device out of its UI
  ("USB data mode").
- **DSL fit: blocked** — no terminators, binary integer parameters (u64 LE) in commands, 32-byte
  struct records, raw (uncalibrated) data needing complex division. VNA class anyway.
- **Docs**: `/home/marek/project/BenchMeshDOC/zeenko_litevna/doc/LiteVNA_User-Guide.pdf` (31 pp.,
  appendix 5 p26-31). **Confidence high** (protocol), **fit none**.

### 2.13 Signal Hound (note only)

USB 3.0 receivers (SA44B/SA124B, BB60C/D, SM200/435) without a front panel or text protocol —
controlled through the vendor C API. SCPI exists only through the **Spike** PC application, raw SCPI
on a TCP socket (`TCPIP::localhost::5025::SOCKET`, one connection at a time) — tech brief
`/home/marek/project/BenchMeshDOC/signalhound/doc/TB-SCPI-Automation-of-Signal-Hound-Spectrum-Analyzers.pdf` p2.
Would need TCP; full Spike command reference not obtained. Out of scope, confidence low.

---

## 3. Shared SCPI baseline (`scpi_sa`) and dialects

All SCPI families share the IEEE-488.2 / SCPI-99 "swept SA" tree (originally HP/Agilent ESA/PSA).
A base definition (`extends: scpi_sa`) can carry:

| Operation | Baseline command | Reply |
|---|---|---|
| identify | `*IDN?` | `vendor,model,serial,fw` |
| center / span / start / stop | `:SENS:FREQ:CENT|SPAN|STAR|STOP <Hz>` / `?` | number (Hz) |
| full span | `:SENS:FREQ:SPAN:FULL` | – |
| RBW / auto | `:SENS:BAND:RES <Hz>`, `:SENS:BAND:RES:AUTO ON|OFF` | number |
| VBW / auto | `:SENS:BAND:VID <Hz>`, `:SENS:BAND:VID:AUTO` | number |
| ref level | `:DISP:WIN:TRAC:Y:SCAL:RLEV <dBm>` | number |
| scale | `:DISP:WIN:TRAC:Y:SCAL:PDIV <dB>` | number |
| attenuation / auto | `:SENS:POW:RF:ATT <dB>`, `:SENS:POW:RF:ATT:AUTO` | number |
| detector | `:SENS:DET[:FUNC] POS|NEG|SAMP|RMS|…` | keyword |
| sweep time / auto | `:SENS:SWE:TIME <s>`, `:SENS:SWE:TIME:AUTO` | number |
| points | `:SENS:SWE:POIN <n>` | integer |
| continuous / single | `:INIT:CONT ON|OFF`, `:INIT[:IMM]` | 0/1 |
| complete | `*OPC?` | 1 |
| marker | `:CALC:MARK1:STAT ON`, `:CALC:MARK1:X <Hz>`, `:CALC:MARK1:Y?` | number |
| peak / next / mkr→CF | `:CALC:MARK1:MAX`, `:CALC:MARK1:MAX:NEXT`, `:CALC:MARK1:SET:CENT` (Rigol `[:SET]` optional) | – |
| trace format | `:FORM[:TRAC][:DATA] ASC|REAL,32` | |
| trace | `:TRAC[:DATA]? TRACE1` | comma list (± block header) |
| trace mode | `:TRAC1:MODE WRIT|MAXH|MINH|AVER|VIEW|BLAN` | keyword |
| unit | `:UNIT:POW DBM` | |
| TG | `:OUTP[:STAT] ON|OFF`, `:SOUR:POW[:LEV:IMM:AMPL] <dBm>` | |

Dialect differences (what each definition overrides):

| Family | Trace query | ASCII reply shape | Trace mode keyword | Other |
|---|---|---|---|---|
| Rigol DSA800 | `:TRAC:DATA? TRACE1` | `#9…` header + `, ` separated | `:TRAC1:MODE WRIT|MAXH|MINH|VIEW|BLAN|VID|POW` | points 101–3001; socket 5555 |
| Rigol RSA3000/5000 | `:TRAC:DATA? TRACE1` | no header (per example) | `:TRAC1:TYPE|MODE WRIT|AVER|MAXH|MINH` | `:INST:SEL SA` first; raw socket 5555 |
| UNI-T UTS | `:TRAC1:DATA?` | `#5…` header + `,` separated | `:TRAC1:TYPE` | `*IDN?` fields with spaces |
| Siglent SSA3000X/Plus | `:TRAC:DATA? 1` | no header, **trailing comma** | `:TRAC1:MODE WRIT|MAXH|MINH|VIEW|BLAN|AVER` | `:BWID` spelling; set `:FORM ASC` (default differs) |
| Siglent SSA5000A | `:TRAC1:DATA?` | as above | **`:TRAC1:TYPE`** | |
| GW GSP-9300 | `:TRAC? TRAC1` | plain CSV, 601 pts | `:DISP:TRAC1:MODE` (set only) | att `:INP:ATT`; peak `:CALC:MARK1:PEAK MAX` |
| GW GSP-818 | `:TRAC? TRACE1` | CSV | `:TRAC1:MODE` | marker replies with units (`9.8dBm`), `ERR`; no `*OPC?` |
| Keysight N9320B/N9322C | `:TRAC? TRACE1` | CSV (N9320B unverified) | `:TRAC1:MODE` | N9322C TG via `INST TGEN` |
| Keysight FieldFox | `:TRAC1:DATA?` | CSV | `:TRAC1:TYPE CLRW|…` | `INST "SA"`; sweep time `SWE:ACQ`; peak `CALC:MARK1:FUNC:MAX` |
| R&S FPC/FPH | `:TRAC? TRACE1` | CSV (1183 / 711 pts) | `:DISP:TRAC1:MODE` | att `INP:ATT`; FPH default units MHz/GHz |
| Hantek HSA2000 | `TRAC:DATA:TRACE1?` | ? | `TRACE1:MODE` | no marker X/Y |

---

## 4. Proposed class contract `SAL`

Design choices: one status block per instrument (channels: 1). Frequencies and times are `si`
(like `OSC.TIMEBASE`); levels stay `number` in **dBm / dB** (SI prefixes on dBm are meaningless).
The trace is a `list` of levels in `TRACE_UNIT` (dBm), one value per point; the x axis is
implicit: `f_i = START + i·(STOP−START)/(POINTS−1)` (POINTS = list length when the device does not
report it). Instruments with fewer capabilities (tinySA has no VBW, detector, or SCPI sweep state)
simply leave optional keys unbound.

```yaml
schema: benchmesh.class/v1
class: SAL
description: >
  Swept spectrum analyzer (bench, handheld, or USB "hobby" analyzer such as the tinySA). One status
  block per instrument. The screen trace is a list of levels in TRACE_UNIT (normally dBm), one per
  sweep point, equally spaced from START to STOP (f_i = START + i*(STOP-START)/(len-1)).
  Features describe the model: freq_range {min, max} (Hz), rbw (list of Hz, or {min, max, steps:
  "1-3-10"}), vbw (same), ref_level {min, max} (dBm), attenuation {min, max, step} (dB),
  points {min, max} (or a fixed number), detectors [..], trace_modes [..], divisions
  {horizontal: 10, vertical: 10}, tg {min, max} (dBm) when a tracking generator / generator exists,
  preamp: bool.
status:
  CENTER:       {type: si, required: true, description: "Center frequency (Hz)"}
  SPAN:         {type: si, required: true, description: "Frequency span (Hz); 0 = zero span"}
  START:        {type: si, required: true, description: "Start frequency (Hz)"}
  STOP:         {type: si, required: true, description: "Stop frequency (Hz)"}
  RBW:          {type: si, required: true, description: "Resolution bandwidth (Hz), actual value"}
  RBW_AUTO:     {type: enum, values: [ON, OFF]}
  VBW:          {type: si, description: "Video bandwidth (Hz)"}
  VBW_AUTO:     {type: enum, values: [ON, OFF]}
  REF_LEVEL:    {type: number, required: true, description: "Reference level (top graticule line), dBm"}
  SCALE:        {type: number, description: "Vertical scale, dB/div"}
  ATTEN:        {type: number, description: "Input attenuation (dB)"}
  ATTEN_AUTO:   {type: enum, values: [ON, OFF]}
  PREAMP:       {type: enum, values: [ON, OFF]}
  DETECTOR:     {type: string, description: "POS, NEG, NORM, SAMP, RMS, AVER, QPE"}
  SWEEP_TIME:   {type: si, description: "Sweep time (s)"}
  SWEEP_MODE:   {type: string, description: "CONT, SINGLE or HOLD"}
  SWEEPING:     {type: bool, description: "A sweep is in progress (single sweep not yet complete)"}
  POINTS:       {type: integer, description: "Sweep points"}
  TRACE:        {type: list, description: "Trace levels in TRACE_UNIT, START..STOP; null when unavailable"}
  TRACE_MODE:   {type: string, description: "WRITE, MAXHOLD, MINHOLD, AVERAGE, VIEW"}
  TRACE_UNIT:   {type: string, description: "dBm (default), dBmV, dBuV, V, W"}
  MARKER:       {type: enum, values: [ON, OFF], description: "Marker 1 shown"}
  MARKER_FREQ:  {type: si, description: "Marker 1 frequency (Hz)"}
  MARKER_LEVEL: {type: number, description: "Marker 1 level (TRACE_UNIT)"}
  TG_OUTPUT:    {type: enum, values: [ON, OFF], description: "Tracking generator / generator output"}
  TG_LEVEL:     {type: number, description: "Tracking generator level (dBm)"}
operations:
  center:       {get: true, set: true, value: si, required: [get, set], description: "Center frequency (Hz)"}
  span:         {get: true, set: true, value: si, required: [get, set], description: "Span (Hz); 0 = zero span"}
  start:        {get: true, set: true, value: si, required: [get, set]}
  stop:         {get: true, set: true, value: si, required: [get, set]}
  full_span:    {set: true, value: string, description: "Any value: set the full frequency range"}
  rbw:          {get: true, set: true, value: si, required: [get, set], description: "Resolution bandwidth (Hz)"}
  rbw_auto:     {get: true, set: true, value: enum, values: [ON, OFF]}
  vbw:          {get: true, set: true, value: si}
  vbw_auto:     {get: true, set: true, value: enum, values: [ON, OFF]}
  ref_level:    {get: true, set: true, value: number, required: [get, set], description: "Reference level (dBm)"}
  scale:        {get: true, set: true, value: number, description: "dB/div"}
  attenuation:  {get: true, set: true, value: number, description: "Input attenuation (dB)"}
  atten_auto:   {get: true, set: true, value: enum, values: [ON, OFF]}
  preamp:       {get: true, set: true, value: enum, values: [ON, OFF]}
  detector:     {get: true, set: true, value: string}
  sweep_time:   {get: true, set: true, value: si}
  points:       {get: true, set: true, value: integer}
  sweep:        {set: true, value: string, required: [set], description: "CONT, SINGLE, HOLD (pause) or RESTART"}
  trace:        {get: true, value: list, description: "Trace levels (see TRACE)"}
  trace_mode:   {get: true, set: true, value: string, description: "WRITE, MAXHOLD, MINHOLD, AVERAGE"}
  marker:       {get: true, set: true, value: enum, values: [ON, OFF]}
  marker_freq:  {get: true, set: true, value: si, description: "Place marker 1 (Hz)"}
  marker_level: {get: true, value: number}
  peak_search:  {set: true, value: string, description: "PEAK, NEXT, LEFT, RIGHT; MKR_CF = marker to center"}
  tg_output:    {get: true, set: true, value: enum, values: [ON, OFF], description: "Generator output (energizes)"}
  tg_level:     {get: true, set: true, value: number, description: "Generator level (dBm)"}
  screenshot:   {get: true, value: bytes}
```

Binding notes: `tg_output` setters carry `safety: energize` (two presses in the UI);
`SWEEPING` from `:STAT:OPER:COND?` bit 3 on SCPI (`{bits: {sweeping: "3"}}`), from `status`
(`Paused`/`Resumed`) on tinySA; `peak_search` must re-poll `MARKER_FREQ`/`MARKER_LEVEL` (a set makes
all poll groups due — already the engine's behaviour). Suggested poll: trace group at
`max(250 ms, sweep time)`, marker group 500 ms, settings 3 s.

### Proposed panel (`SalPanel`, profile `salProfile()`)

As on the instruments' front panels (Rigol DSA, Siglent SSA, tinySA screen layout):

- **Screen** (kit `ScopeScreen` sibling, `SpectrumScreen`): 10 × 10 graticule (features.divisions);
  y axis top = `REF_LEVEL`, `SCALE` dB/div (default 10), labels in dBm on the left; x axis START…STOP;
  trace drawn as a polyline from `TRACE` in the skin's trace colour; optional max-hold ghost trace
  kept client-side. Marker 1 as a diamond on the trace with a readout box top-right:
  `MKR1 433.920 MHz  −42.31 dBm`. Click on the screen = place marker (`marker_freq`); wheel over
  the x axis = center step, shift-wheel = span 1-2-5.
- **Annotation bar** (under the screen, like every SA): `CENTER 433.920 000 MHz  SPAN 10.000 MHz`,
  `RBW 30 kHz  VBW 30 kHz  SWT 50 ms`, `REF −10.0 dBm  ATT 10 dB  DET POS  ●SWEEPING`.
- **Soft-key groups** (kit components, shown only when the setter exists):
  - FREQ: digit-wise editors for Center / Span / Start / Stop (`DigitEditor`, Hz with SI),
    Full span, Zero span.
  - BW: RBW and VBW `Stepper` over features.rbw (1-3-10), `MicroSwitch` AUTO.
  - AMPT: Ref level `Stepper` (1 dB / 10 dB), scale (1/2/5/10 dB/div), attenuation (steps from
    features), preamp switch.
  - SWEEP: `Selector` CONT / SINGLE / HOLD, sweep time (auto), points.
  - TRACE: `Selector` WRITE / MAXHOLD / MINHOLD / AVERAGE; detector `Selector`.
  - MARKER: ON/OFF, frequency editor, buttons PEAK, NEXT, MKR→CF (sets `center` to MARKER_FREQ).
  - TG / GEN: `ArmSwitch` (two presses, energizes) + level `Stepper`.
- **Recordable**: `MARKER_LEVEL` and `MARKER_FREQ` (right-click → record, like DMM readings);
  confirmation (`kit/confirm.ts`) compares read-back center/span/rbw/ref level.
- Profile: `salProfile(features, setters)` → freq range, rbw/vbw lists, ref-level range, points,
  detectors, trace modes, has TG/preamp/VBW; tests against bench fixtures from a simulated tinySA
  and DSA815 capture.

---

## 5. DSL features this class needs

| Feature | Blocks | Proposal |
|---|---|---|
| **ASCII number list** (critical) | trace of **every** family (SCPI `:TRAC:DATA?` ASCII, tinySA/NanoVNA `data`) | parse step `{numbers: {sep: ","}}` → `list` of f64: splits on `sep` **and** surrounding whitespace / newlines, ignores a trailing separator and empty fields, accepts `1.2e+01`/`-8.3e+01`; as first step after `reply: block` it parses the block payload; when used on a `line`/`message` reply it skips an optional leading IEEE `#<n><len>` header (RSA3000 vs DSA800 ambiguity). `sep: "\n"` or `" "` for shells |
| float32 samples (nice to have) | faster/binary trace (`FORM REAL,32`, NanoVNA `scan … 0x80`) | `samples: f32le|f32be` (and `f64le`) |
| TCP link | FieldFox, **R&S FPC/FPH (LAN-only, USB is a network link)**, Signal Hound Spike; optional for all SCPI families (5025 Siglent/Keysight, 5555 Rigol) | `link.tcp` is in the schema — interpreter transport needed |
| Stride/skip in samples | tinySA `scanraw` (`x`,MSB,LSB) | not needed if `numbers` exists |
| Streaming / push frames | RF Explorer | out of scope |
| Binary command encoding (u64 LE params) + struct records | NanoVNA V2 / SAA-2 / LiteVNA | out of scope |
| Shell prompt framing | tinySA, NanoVNA | **already expressible**: `rx.terminator: "ch> "` + echo-stripping `regex`; setters `reply: line` |
| Drain before identify | tinySA/NanoVNA connect banner (`tinySA Shell\r\nch> `) | `Engine::open` drains once (when `drain_idle_ms` is set) before the identify exchange |

---

## 6. VNA: separate class, not SAL

Recommendation: **a separate class `VNA`**, written later. Reasons: a VNA measures complex
S-parameters (S11, S21 as re/im pairs per point), its displays are log-mag/phase/SWR/Smith chart/
impedance, it needs calibration state (OSL/thru), electrical delay and IF bandwidth instead of
RBW/ref level/attenuation, and its markers read impedance/SWR. Forcing that into SAL would make
most SAL keys meaningless and most VNA quantities unrepresentable. Shared parts are only the
frequency axis (START/STOP/POINTS) and the trace transport.

Sketch for later: status `START, STOP, POINTS, IFBW, POWER, CAL_STATE, S11 (list, interleaved re,im),
S21 (list)`, derived in the UI (|S|dB = 20·log10|Γ|, phase, SWR, R+jX); operations `start, stop,
points, sweep, s11, s21`. NanoVNA-H/H4 and NanoVNA-F fit that with the `numbers` step (`data 0`/`data 1`);
LiteVNA/V2 needs a binary protocol; Siglent SVA1000X can bind both SAL (SA mode) and VNA (VNA mode).
