# Device notes: OWON XDM1x41 multimeter (`owon_xdm`, class DMM)

Input for the Phase 2 device definition. Bench unit: **XDM1241, firmware V4.3.0** (device id `owon-xdm1241`).

**Sources**
- *PM*: `BenchMeshDOC/owon_xdm/doc/XDM1000_Digital_Multimeter_Programming_Manual.pdf` (26 pp.). The title page says **"XDM1041 & XDM2041"**. Page numbers below are PDF pages; the printed page is in parentheses.
- *UM*: `BenchMeshDOC/owon_xdm/doc/OWON_XDM1141_USER_MANUAL.pdf` (42 pp.). Page numbers are PDF pages.
- *driver*: `benchmesh-serial-service/src/benchmesh_service/drivers/owon_xdm/{driver.py,manifest.json}`
- *RO capture*: `captures/20261016-201609` (passive, VOLT mode only) · *SET capture*: `captures/20261016-203514` (mode/range/temp/rate/remote setters)

Precedence: hardware > PM > UM > driver.

---

## 1. Identity

| | |
|---|---|
| Query | `*IDN?` (PM p.9 (5)) |
| PM format | `OWON,<model>,<serial number>,X.XX.XX,{3}`, e.g. `OWON,XDM2041,1546011,V1.0.0,3` |
| Observed | `OWON,XDM1241,24512010,V4.3.0,3` (RO 4596/4596, SET 4045 correct; the rest were desync artefacts, see §7) |
| Parse | `^OWON,(XDM\d{4}),([^,]+),V?([\d.]+),(\d+)$` → model, serial, firmware, trailing field (meaning unknown, always `3`) |
| Models | Manifest `id_patterns`: `OWON,XDM1041`, `OWON,XDM1141`, `OWON,XDM1241`. Only the 1241 has been verified. The UM covers the XDM1141. The 1x41 family shares one feature set (55 000 counts, UM p.39). The PM also covers the XDM2041, which has more functions (FRES, 500 MΩ). |

## 2. Link

| | Manual | Observed / Python |
|---|---|---|
| Physical | RS232 or USB device (UM p.40). The bench unit uses a CH340 USB-serial adapter (`1a86`). | `/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0` |
| Baud | 2400/4800/9600/19200/38400/57600/**115200** (default). Parity None/Odd/Even. Stop bits 1/2. Data bits fixed at 8 (UM p.34, defaults p.35) | 115200 8N1, no flow control. Python forces DTR/RTS low (harmless). |
| TX terminator | not specified in the PM | Python sends `\r`, which works. `\n` and `\r\n` are untested (§8). |
| RX terminator | not specified in the PM | **`\r\n`** (`<payload>\r\n`). The first reply after open has no leading `\n`. Python stops at `\r` and skips the leftover `\n` as noise on the next read. |
| Timing | - | first byte 0.16-0.22 ms (median), exchange 4-11 ms (`FUNCtion?` ≈6 ms, `MEAS1?` ≈8 ms, `*IDN?` ≈11 ms). Outliers up to ~100 ms. No inter-command gap was needed at 0.2 ms spacing. |
| Setters | - | **No reply, ever** (all 14 setters in the SET capture). The only byte read was the stale `\n` from the previous reply. Use `reply: none`. |

DSL: `tx.terminator: "\r"`, `rx.terminator: "\r\n"` (or `"\n"` + trim), `rx.skip_leading: ["\r","\n"]`. Before every command, **drain the input buffer** so that a late reply cannot shift the stream (§7.2).

## 3. Command table

Mnemonics are case-insensitive. Short forms are the capital letters (PM p.7 (3)). Optional `[SENSe:]` and `[:SCALar]` nodes may be left out. "RO"/"SET" = seen in the capture of that name.

### 3.1 Function (mode) selection: use `CONFigure`

| Command | Params | Reply | Effects / notes | Cite |
|---|---|---|---|---|
| `CONF:VOLT:DC [<r>\|MIN\|MAX\|DEF\|AUTO]` | DC: 500E-3, 5, 50, 500, 1000 (PM table). 50E-3 is in the `RANGE` table and is the UM "mV" sub-function. | none | Switches to DCV and **"restores all measurement and trigger parameters to default"**, then sets the range. SET: `CONF:VOLT:DC AUTO` → `FUNCtion?`=`"VOLT"`. | PM p.13-14 (4-5) |
| `CONF:VOLT:AC [<r>\|…\|AUTO]` | 500E-3, 5, 50, 500, 750 | none | SET → `"VOLT AC"` | PM p.14 (5) |
| `CONF:CURR:DC [<r>\|…\|AUTO]` | 500E-6, 5E-3, 50E-3, 500E-3, 5, 10. The PM table under CONF:CURR is a copy-paste of the voltage table, so the values come from the `RANGE` table, PM p.24. | none | SET → `"CURR"` | PM p.14 (5) |
| `CONF:CURR:AC [<r>\|…\|AUTO]` | same as DC | none | not captured | PM p.14 (5) |
| `CONF:RES [<r>\|…\|AUTO]` | 500, 5E3, 50E3, 500E3, 5E6, 50E6, (500E6: PM only; UM spec max is 50 MΩ) | none | SET: `CONF:RES AUTO`, `CONF:RES 5000.0` → `"RES"` | PM p.14-15 (5-6), UM p.39 |
| `CONF:FRES [<r>\|…]` | same, max 50E3 | none | **Probably unsupported on the 1x41.** The UM has no 4-wire mode. | PM p.15 (6) |
| `CONF:CAP [<r>\|…\|AUTO]` | 50E-9 … 50E-3 (7 ranges) | none | SET → `"CAP"` | PM p.15-16 (6-7) |
| `CONF:FREQ` / `CONF:PER` | - | none | SET (`FREQ`) → `"FREQ"`. PER not captured. | PM p.15 (6) |
| `CONF:DIOD` / `CONF:CONT` | - | none | not captured | PM p.16-17 (7-8) |
| `CONF:TEMP:RTD [KITS90\|PT100]` | sensor | none | SET → `"TEMP"`. **Device stalls for ≈2.8 s afterwards**: the next `MEAS1?` got no reply within 1 s, and its reply arrived ≈2.85 s after the CONF (§7.2). Overwrites the sensor choice. | PM p.16 (7) |
| `[SENS:]FUNC[1\|2] "<fn>"` | PM table lists only FREQ/NONe (sub display) | none | Using it to switch the main function is documented ambiguously (PM p.10 (1)). Not used. | PM p.10-11 (1-2) |

### 3.2 State queries

| Command | Reply (observed) | Notes | Cite |
|---|---|---|---|
| `FUNC?` (= `FUNCtion?`, `FUNC1?`) | `"VOLT"` (quoted, `\r\n`) | Values in §4. Use `unquote`. | PM p.11 (2) |
| `FUNC2?` | `NONe` when dual display is off (PM) | not captured | PM p.11 (2) |
| `MEAS1?` | `-7.941000E-05` | Main display value: `[-]d.ddddddE±dd`, 7 significant digits, no `+` on positive mantissa, zero is `0.000000E+00`. Value is in the **selected unit** (TEMP in °F: `6.882211E+01` ≈ 20.5 °C). | PM p.25-26 (16-17) |
| `MEAS?` | main, or `main,sub` when dual display is on | Avoid: the shape depends on dual-display state. | PM p.25 (16) |
| `MEAS2?` | sub display value | - | PM p.26 (17) |
| `READ?` | - | **Not in the PM.** Do not use. | - |
| `RATE?` | `S` (RO 3/3 in VOLT, SET 3/3 in TEMP) | The PM documents only `F`/`M`/`L`. The firmware reports **`S`** for slow. | PM p.24-25 (15-16) |
| `AUTO?` | `1` auto / `0` manual (PM) | not captured | PM p.23 (14) |
| `TEMP:RTD:TYP?` | `KITS90`/`PT100` ("by character") | not captured | PM p.12 (3) |
| `TEMP:RTD:UNIT?` | `C`/`F`/`K` (implied by "return the query result by character") | not captured | PM p.12 (3) |
| `SYST:BEEP:STAT?` | `0`/`1` | not captured | PM p.22 (13) |
| `CALC:FUNC?`, `CALC:DB:REF?`, `CALC:DBM:REF?`, `CALC:NULL:OFFS?` | - | not captured | PM p.19-21 (10-12) |

### 3.3 Setters (all `reply: none`)

| Command | Params | Notes | Cite |
|---|---|---|---|
| `RANGE <n>\|DEF` | **1-based index** within the current function. DCV 1..6 = 50m/500m/5/50/500/1000 V · ACV 1..5 = 500m/5/50/500/750 V · DCI/ACI 1..6 = 500µ/5m/50m/500m/5/10 A · RES 1..6 = 500/5k/50k/500k/5M/50M Ω · CAP 1..7 = 50n…50m F · TEMP 1/2 = KITS90/PT100 | Changes the range **without** resetting other parameters. Preferred over `CONF:<fn> <r>` for range-only changes. Not captured. | PM p.24 (15) |
| `AUTO` | - | Enables autorange. Not captured. | PM p.23 (14) |
| `RATE F\|M\|L` | F 65 rdg/s, M 16 rdg/s, L 4 rdg/s (UM) | **Selectable only in DCV, ACV, DCI, ACI and RES** (UM p.18). The SET capture sent `RATE F`/`RATE M`/`RATE S` while in **TEMP** mode, which is the likely reason they had no effect. Also, every `CONF:*` resets "measurement parameters to default" and may reset the rate. To verify: in VOLT mode, `RATE F` then `RATE?` (§8). | PM p.24-25, UM p.18 |
| `TEMP:RTD:TYP KITS90\|PT100` | sensor | Changes the sensor without switching function (PM p.10 (1)). Prefer this over `CONF:TEMP:RTD` for "set sensor". | PM p.12 (3) |
| `TEMP:RTD:UNIT C\|F\|K` | unit | SET: `F` changed `MEAS1?` to °F within ≈200 ms. | PM p.12 (3) |
| `TEMP:RTD:SHOW TEMP\|MEAS\|ALL` | display | not needed by the class | PM p.13 (4) |
| `CONT:THRE <Ω>` | 1-1000 Ω, default 50 | - | PM p.13 (4), UM p.24 |
| `SYST:REM` / `SYST:LOC` | - | "Enter/Exit SCPI mode". Queries work **without** `SYST:REM`: both captures ran in local mode. Front-panel lock behaviour is unknown. | PM p.22-23 (13-14) |
| `SYST:BEEP:STAT ON\|OFF` | bool | default (UM p.35): Beeper Off | PM p.22 (13) |
| `CALC:FUNC NULL\|DB\|DBM\|AVER`, `CALC:STAT OFF`, `CALC:NULL:OFFS <v>`, `CALC:DB[M]:REF <Ω>` (17 discrete values) | - | Maps to class features `delta`, `DB:REFerence`, `DBM:REFerence` (unused by UI). | PM p.17-21 |
| `*RST` | - | defaults: DCV, Auto, Speed Low, °C, KITS90 (UM p.35) | PM p.9 (5) |

**Hold:** there is no SCPI command. Hold is the front-panel Run/Stop key only (UM p.30). Class feature `hold: true` cannot be implemented and should be dropped from the definition.

## 4. `FUNC?` replies → manifest mode keys

The reply is the quoted short form (PM p.11 (2)). Rule: unquote, then map.

| `FUNC?` reply | Mode key | Set via | Seen |
|---|---|---|---|
| `"VOLT"` | `VOLT` | `CONF:VOLT:DC AUTO` | RO, SET |
| `"VOLT AC"` | `VOLT_AC` | `CONF:VOLT:AC AUTO` | SET |
| `"CURR"` | `CURR` | `CONF:CURR:DC AUTO` | SET |
| `"CURR AC"` | `CURR_AC` | `CONF:CURR:AC AUTO` | PM only |
| `"RES"` | `RES` | `CONF:RES AUTO` | SET |
| `"FRES"` | `FRESistance` ⚠️ Python's normalisation gives `FRES`, which does not equal the key | `CONF:FRES AUTO` | PM only (likely absent on 1x41) |
| `"CAP"` | `CAP` | `CONF:CAP AUTO` | SET |
| `"DIOD"` | `DIOD` | `CONF:DIOD` | PM only |
| `"CONT"` | `CONT` | `CONF:CONT` | PM only |
| `"TEMP"` | `TEMP` | `CONF:TEMP:RTD <sensor>` | SET |
| `"FREQ"` | `FREQ` | `CONF:FREQ` | SET |
| `"PER"` | `PER` | `CONF:PER` | PM only |
| (DC mV sub-function, UM p.18) | ? | ? | unknown: it may report `"VOLT"` with range 50 mV |

Unknown replies → `MODE: null` plus a log entry. Do not drop the link.

**Measurement edge cases (SET capture, nothing connected):**
- FREQ, CAP and VOLT AC with no signal → `0.000000E+00`.
- **Stale mantissa after a mode change.** For ~0.5-1.5 s after `CONF:*`, `MEAS1?` returns the *previous function's mantissa with the new range's exponent*:
  - `-9.401413E-05` V becomes `-9.401413E-11` A.
  - The CURR reading `-1.425438E-08` becomes RES `-1.425438E-02`, then `-1.425438E+01`.
  - Mark the first readings after a mode/range change as invalid (settle window). The UM says the same for auto-range relay switching (UM p.33).
- **Overload/open circuit is not documented in the PM.** The UM displays "overload" (UM p.17). RES open circuit (AUTO, then 5 kΩ) returned the **stale `-1.425438E+01`** for >2 s rather than a sentinel. Either the device keeps the last value on overload, or the sentinel comes later. **Must be verified** (§8). Until then, the definition needs an `overload` rule hook (e.g. `|v| ≥ 9.9E+37` or a literal string) that is left empty.
- The Python driver's SI dict changes only on value change. The device updates `MEAS1?` only about **once per second** at rate `S` in DCV-auto (RO: median 1.04 s between distinct values over 487 changes). This is slower than the UM's 4 rdg/s for "Low", which is consistent with `S` ≠ `L`.

## 5. Can the active range be queried?

**Not by any documented command.** The PM has no `RANGE?` and no `[SENS:]<fn>:RANGe?` entry. The only documented range-related query is **`AUTO?` → `1`/`0`** (PM p.23 (14)). However, the PM uses `VOLTage:{AC|DC}:RANGe {<range>|MIN|MAX}` (p.6 (2)) and `VOLT:AC:RANGE:AUTO ON` (p.10 (1)) as syntax examples, so the SENSe range tree may exist in firmware. The sibling OWON SPM answers `CONFigure?` with `<fn> <range>`.

Candidates to probe (read-only, in order): `RANGE?`, `CONF?`, `VOLT:DC:RANG?`, `SENS:VOLT:DC:RANG?`, `VOLT:DC:RANG:AUTO?`, `AUTO?`.

- If one works → `RANGE` is a polled value. Drop the Python shadow state.
- Otherwise → keep a minimal shadow:
  - `AUTO?` gives auto vs manual. Manual range = last value set by us.
  - Reset to `AUTO` whenever a `FUNC?` change is detected (CONF and power-on default to Auto, UM p.17).
  - Report `null` if a front-panel change makes it unknown (manual mode we did not set).

## 6. Recommended poll strategy

| Item | Command | Cadence |
|---|---|---|
| MEAS | `MEAS1?` (not `MEAS?`, whose shape depends on dual display) | every poll |
| MODE | `FUNC?` (≈6 ms) | every poll. It is cheap, and it is the only way to track front-panel changes. It also gates the settle window. |
| RANGE | the query from §5 if found; else `AUTO?` | every Nth poll (e.g. 1 s) or on MODE change |
| Presence | **none extra**. Any well-formed reply to `MEAS1?`/`FUNC?` within timeout = alive. N consecutive timeouts = link down. | - |
| Identity | `*IDN?` at connect/reconnect only (model match + serial) | - |

- **Interval:** 250-500 ms is enough. The device refreshes ~1×/s at rate `S`. Python's 100 ms poll × 3 queries ≈ 27 ms of every 100 ms is wasteful. Optionally tie the interval to `RATE?` (F → 100 ms).
- **Reply timeout:** 200 ms normally. Use **≥3 s for the first query after `CONF:TEMP`** (other CONF settle times are unmeasured: Python always waited 1 s), or add `post_delay_ms` on mode setters.
- **On any timeout:** drain RX until idle (≥20 ms) before the next command.
- **Cache:** none needed. Setters invalidate MODE/RANGE and start a settle window during which MEAS is reported as stale/null.

## 7. Discrepancies

### 7.1 Manual vs device
- The PM is for XDM1041/2041, not the 1241. Its FRES and 500 MΩ entries are probably 2041-only (UM has neither).
- `RATE?` returns `S`. The PM documents `L` for low. Whether `RATE L` or `RATE S` is the accepted setter is unknown.
- The PM CONF:CURR range table is a copy-paste of the voltage table (p.14). Use the `RANGE` table (p.24).
- The PM CONF:VOLT:DC table lacks 50 mV, but the `RANGE` table (p.24) and the manifest include it.
- No terminator, overload format or `READ?` is documented.

### 7.2 Capture findings
- **Reply-stream shift after a slow mode change (SET capture 296.4-300.4 s):**
  - `CONF:TEMP:RTD KITS90` → the next `MEAS1?` timed out (1 s). Its reply arrived late, so every following reply was off by one: `FUNC?`→`1.788629E+01`, `*IDN?`→`"TEMP"`, `MEAS1?`→`OWON,XDM1241,…`.
  - The shift lasted ≈1 s, until the no-reply setter `TEMP:RTD:UNIT F` consumed one reply (logged as its "reply").
  - The API returned 400 `could not convert string to float: 'OWON,XDM1241,…'`, and the registry carried garbage.
  - Rust: drain on timeout, plus a per-command reply validator (number / quoted word / IDN regex) that triggers a resync.
- `SYST:REM` and `SYST:LOC` have no visible effect on queries.

### 7.3 Python driver / manifest bugs (do not replicate)
1. **Inverted cache** in `query_measurement`/`query_function` (`driver.py`, see plan "Bugs"): re-queries on a cache hit and returns `None`/crashes on a miss. `query_measurement` uses `si_to_value` while `poll_status` uses `sci_to_value`, so the two produce different shapes.
2. `*IDN?` every poll (4 596× in RO), and it only checks truthiness. It did not detect the shifted stream.
3. Reads after every setter, adding ≈1 s each and enabling the shift above.
4. RANGE shadow `_preserve_range` is keyed by the **old** cached function. `set_mode` sends `CONF:<new> AUTO` before the function cache updates, so `VOLT→VOLT_AC` stores `VOLT:RANGE=AUTO`. Front-panel changes are never reflected.
5. `set_mode("TEMP")` always forces `KITS90`. `set_temp_sensor` uses `CONF:TEMP:RTD`, which also resets the function. `TEMP:RTD:TYP` is the correct command.
6. Range ops are `set_<fn>_range` and each issues `CONF:<fn> <r>`, which resets other parameters. There is no `set_range` (class contract ⚠️). Use `RANGE <n>` / `AUTO`.
7. Manifest problems:
   - `connection.reol: "\r"` (actual `\r\n`).
   - Mode key `FRESistance` ≠ `FRES`.
   - `TEMP.unit: "K"`, but the actual unit is whatever `TEMP:RTD:UNIT` says (default °C). The unit should be read, not fixed.
   - `rate` lists `L` "slow", but the device reports `S`.
   - `hold: true` cannot be implemented over SCPI.
   - `RES`/`FRESistance` ranges include 500E6, which the 1x41 does not have. `FRESistance` ranges are not capped at 50E3.
8. CLASS_CONTRACT example `MODE` = `VOLT_DC` is wrong. The device reports `VOLT`.

## 8. Open questions for hardware

| # | Question | Probe | Kind |
|---|---|---|---|
| 1 | Range query exists? | `RANGE?`, `CONF?`, `VOLT:DC:RANG?`, `SENS:VOLT:DC:RANG?`, `VOLT:DC:RANG:AUTO?`, `AUTO?` | read-only |
| 2 | TX terminator tolerance | `*IDN?\n`, `*IDN?\r\n` | read-only |
| 3 | Error queue | `SYST:ERR?` after a bad command such as `FOO?` (bad command is harmless) | read-only |
| 4 | Untested queries' reply formats | `TEMP:RTD:UNIT?`, `TEMP:RTD:TYP?`, `SYST:BEEP:STAT?`, `CALC:FUNC?`, `FUNC2?`, `MEAS?` | read-only |
| 5 | Correct RATE syntax | in VOLT mode: `RATE F`→`RATE?`, `RATE M`, `RATE L`, `RATE S`. Then check whether `CONF:VOLT:DC AUTO` resets the rate. Restore `S`. | state-changing |
| 6 | Index range setter | `CONF:VOLT:DC AUTO`, `RANGE 3`, `AUTO?` (expect 0), then `AUTO`, `AUTO?` | state-changing |
| 7 | Settle time per mode | after each `CONF:*`: `MEAS1?` immediately and every 50 ms; record first-reply latency and the stale-mantissa duration | state-changing |
| 8 | Overload / open-circuit format | `CONF:RES 500` with open leads → `MEAS1?` for 5 s; `CONF:CONT` and `CONF:DIOD` open | state-changing (safe, nothing connected) |
| 9 | Uncaptured `FUNC?` replies | `CONF:CURR:AC`, `CONF:DIOD`, `CONF:CONT`, `CONF:PER`, `CONF:FRES` (does it error or get ignored?), DC mV sub-function (`CONF:VOLT:DC 50E-3`) | state-changing |
| 10 | Remote lock semantics | `SYST:REM` → are front-panel keys locked? Does the display show REM? Then `SYST:LOC`. | state-changing |
| 11 | `FUNC "VOLT"` as a mode setter | `FUNC "CURR"` → `FUNC?` | state-changing |

After probing, restore the bench to `CONF:VOLT:DC AUTO`, rate `S`, `TEMP:RTD:UNIT C`.

## Bench verification (Rust bm-probe, 2026-10-16)
- `RATE F` / `RATE M` / `RATE S` all work in VOLT and VOLT_AC; **`RATE L` (manual token) is silently ignored** → the device's slow token is `S`. The Python failure was only because the meter was in TEMP mode.
- A mode change (`CONF:...`) does **not** reset the rate (stayed F after `CONF:VOLT:DC AUTO`).
