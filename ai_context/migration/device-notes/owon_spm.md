# OWON SPM3103 (driver `owon_spm`) - device notes

Combined power supply + bench multimeter on **one serial port** → classes **PSU** and **DMM**, 1 channel each.
Sources: Python driver `benchmesh-serial-service/src/benchmesh_service/drivers/owon_spm/{driver.py,manifest.json}`,
captures `captures/20261016-201609` (read-only) and `captures/20261016-203514` (setters), device id `owon-spm3103`.

**Citations:** `PM pN` = `SPM_Series_programming_manual.pdf` **PDF page** N (printed page = N−2);
`UM pN` = `SPM_Series_User_Manual.pdf` PDF page N. Manuals live in `/home/marek/project/BenchMeshDOC/owon_spm/doc/`.
**Rule: where capture and manual disagree, the capture wins.**

---

## 1. Identity

| Item | Value |
|---|---|
| Probe | `*IDN?` (PM p6) |
| Manual format | `manufacturer,<model>,<serial number>,FV:X.XX.XX` (PM p6) |
| Observed | `OWON,SPM3103,24460763,FV:V2.1.0` (32 B incl. `\n`, first byte 3.4-4.3 ms) |
| Match pattern | `^OWON,SPM3103,` (manifest `id_patterns: ["OWON,SPM3103"]`); family-wide `^OWON,SPM\d+,` is plausible but only SPM3103 is verified |
| Parse | split `,` → vendor, model, serial, firmware (`FV:` prefix; note firmware string is `V2.1.0`, not `X.XX.XX`) |
| `*RST` | documented (PM p6) - factory reset, state-changing, never send automatically |

## 2. Link

| Item | Manual | Observed / Python |
|---|---|---|
| Physical | RS232 / USB / LAN depending on rear-panel variant (UM p10-12) | serial adapter at `/dev/serial/by-path/...-port0` |
| Serial params | **not specified in either manual** | **115200 8N1**, no flow control; Python forces DTR/RTS low on open (works) |
| TX terminator | not specified | Python sends `\r` → accepted by every command |
| RX terminator | not specified | every reply ends with a single **`\n`** (no `\r`) |
| Python framing bug | - | manifest `reol: "\r"` → every read ends on the 1 s pyserial timeout (exchange ≈1015-1040 ms vs real 4-23 ms) |
| Reply latency (first byte) | - | IDN 3-4 ms; VOLT?/CURR? ≈5 ms; *:LIM?/OUTP? 6-10 ms; MEAS:* 8-12 ms; MEAS:ALL:INFO? 7-23 ms; CONFigure? 9-40 ms (median 13) |
| Setters | manual shows no reply | **no setter ever replies** (`no_reply` ×all, 1 s timeout in Python) → `reply: none` |
| `\n` as TX terminator | manual silent → unknown | **not tested** (open question Q1) - keep `\r` |

DSL: `tx.terminator: "\r"`, `rx.mode: terminator "\n"`, `rx.timeout_ms ≈ 200`, setters `reply: none`.

## 3. Command table

All commands accept SCPI short/long forms (PM p3-5). Numeric parameters accept decimal and scientific notation and `MIN|MAX|DEF` (PM p4).
"Seen" = present in captures; examples are verbatim device replies (terminator `\n` omitted).

### 3a. PSU

| Function | Syntax | Param | Reply format | Real example | Notes | Cite |
|---|---|---|---|---|---|---|
| Set voltage | `VOLT <v>` (`[SOUR:]VOLT[:LEV][:IMM][:AMPL]`) | real, V; manifest 0.01-31 | none | `VOLT 5.0` → (no reply) | seen; read-back `VOLT?` = `5.000` | PM p11 |
| Query voltage setpoint | `VOLT?` | - | `%.3f` V | `9.000` | seen | PM p11 |
| Set current | `CURR <a>` | real, A; manifest 0.01-10.1 | none | `CURR 6.55` → (no reply) | seen | PM p10 |
| Query current setpoint | `CURR?` | - | `%.3f` A | `6.550` | seen | PM p10 |
| Set OVP level | `VOLT:LIM <v>` | real, V; manifest 0.1-32 | none | `VOLT:LIM 20.0` | seen; read-back `20.000` | PM p11 |
| Query OVP level | `VOLT:LIM?` | - | `%.3f` V | `32.000` | seen | PM p11 |
| Set OCP level | `CURR:LIM <a>` | real, A; manifest 0.1-10.2 | none | `CURR:LIM 5.0` | seen; read-back `5.000` | PM p10 |
| Query OCP level | `CURR:LIM?` | - | `%.3f` A | `10.200` | seen | PM p10-11 |
| Output on/off | `OUTP {ON\|OFF\|1\|0}` | bool | none | `OUTP ON` | seen; on with no load → INFO mode goes 0→1 within ≤2 s, VOUT ramps (`2.860` then `5.000`) | PM p9-10 |
| Query output | `OUTP?` | - | **`ON`/`OFF`** (manual says `1`/`0`) | `ON`, `OFF` | seen | PM p10 |
| Remote | `SYST:REM` | - | none | - | never sent; not needed for control (setters worked in local) | PM p12 |
| Local | `SYST:LOC` | - | none | - | never sent | PM p12 |
| Remote query | - | - | - | - | **no `SYST:REM?` documented** | PM p12 |
| Measured V | `MEAS:VOLT?` | - | `%.3f` V | `0.000` | seen | PM p7 |
| Measured I | `MEAS:CURR?` | - | `%.3f` A | `0.000` | seen | PM p7-8 |
| Measured P | `MEAS:POW?` | - | `%.3f` W | `0.000` | seen | PM p8 |
| V,I,P | `MEAS:ALL?` | - | manual: 3 space-separated; **device: 2 comma-separated** | `0.000,0.000` | seen; field set unclear (V,I?) - don't use | PM p8 |
| Status | `MEAS:ALL:INFO?` | - | 7 comma-separated fields (manual shows spaces + `1/0` booleans) | `5.000,0.000,0.000,OFF,OFF,OFF,1` | seen; **the PSU poll command**, see §4 | PM p9 |
| Panel key emulation | `SIM:KEY:{DISP\|V\|I\|OVP\|OCP\|ONOF\|OK\|MODE\|F1..F4}`, `SIM:KEY:KNOB <n>` | - | none | - | not seen; not needed | PM p19 |

No memory-bank SCPI commands are documented (manifest `memory_banks: 4` is panel-only).

### 3b. DMM

Function select (all no-reply setters; `[SENS:]` optional) - PM p13-15:

| MODE key | Set command | `CONFigure?` / `FUNC:x?` token | Seen |
|---|---|---|---|
| `VOLT_DC` | `FUNC:VOLT[:DC]` (Python: `FUNCtion:VOLTage:DC`) | `VOLT:DC` | yes |
| `VOLT_AC` | `FUNC:VOLT:AC` | `VOLT:AC` | yes |
| `CURR_DC` | `FUNC:CURR[:DC]` | `CURR:DC` | no |
| `CURR_AC` | `FUNC:CURR:AC` | `CURR:AC` | no |
| `RES` | `FUNC:RES` | `RES` | yes |
| `CAP` | `FUNC:CAP` | `CAP` | no |
| `DIOD` | `FUNC:DIOD` | `DIOD` | no |
| `CONT` | `FUNC:CONT` | `CONT` | yes |

The manual documents per-function queries `FUNC:VOLT?`, `FUNC:CURR?`, `FUNC:RES?` … (PM p13-15), **not** a generic `FUNC?` (Python's fallback; never sent on the wire). Mode is better read from `CONFigure?`.

Ranges (all setters no-reply; "can only be effectively set after function gear is correctly set", PM p16-18):

| Function | Set | Values (manual) | Auto | Query | Cite |
|---|---|---|---|---|---|
| VOLT:DC | `VOLT:DC:RANG <r>` | `200E-3, 2, 20, 200, 1000`, `MIN`, `MAX` | `VOLT:DC:RANG:AUTO {ON\|OFF\|1\|0}` | `VOLT:DC:RANG?`, `…:AUTO?` | PM p16 |
| VOLT:AC | `VOLT:AC:RANG <r>` | `200E-3, 2, 20, 200, 750` | `VOLT:AC:RANG:AUTO …` | `VOLT:AC:RANG?` | PM p16 |
| CURR:DC/AC | `CURR:{DC\|AC}:RANG <r>` | `200E-3, 10` | **no AUTO command documented** | `CURR:{DC\|AC}:RANG?` | PM p17 |
| RES | `RES:RANG <r>` | `200, 2E3, 20E3, 200E3, 2E6, 20E6, 100E6` | `RES:RANG:AUTO …` (manual text garbled) | `RES:RANG?` | PM p17-18 |
| CAP | **no setter** | query returns `2E-9 … 10E-3` | always auto ("No manual range mode is available for capacitance", UM p26) | `CAP:RANG?` | PM p18 |
| DIOD, CONT | - | - | - | - | - |

Relative (`…:RANG:NULL {ON|OFF}`, `CAP:NULL`) PM p16-18; hold `MULT:HOLD {ON|OFF}` / `MULT:HOLD?` PM p18-19. No rate/speed command exists.
Range query reply format is **undocumented and never captured** (Q4). Note UM p9 says manual range is "not available for capacitors, diodes and current" - conflicts with PM p17 for current (Q6).

Measurement:

| Command | Reply | Real examples | Cite |
|---|---|---|---|
| `CONFigure?` | `<FUNC> <value>` single space; value `%+.4E` in base SI unit (V, A, Ω, F) or `OL` | `VOLT:DC +1.0000E-04`, `VOLT:DC +1.0000E-03`, `VOLT:AC +1.8700E-02`, `VOLT:DC +0.0000E-308`, `RES OL`, `CONT OL` | PM p13 |
| `CONF:ALL?` | manual: `<FUNC>,<value+unit>,<AUTO\|Manual>,<range+unit>` | manual only: `VOLT:DC,+0.0011V,AUTO,2V`, `RES,+000.26Ohm,AUTO,200Ohm`, `CURR:DC,+000.00mA,Manual,200mA` - **never captured** | PM p12 |

## 4. Reply semantics

### `MEAS:ALL:INFO?` (PM p9)
Observed grammar: `<V>,<I>,<P>,<OVP>,<OCP>,<OTP>,<MODE>` - `%.3f` numbers, flags `ON|OFF`, mode integer.

| # | Field | Manual meaning | Observed |
|---|---|---|---|
| 1 | V | measured output voltage | `0.000`, `2.860`, `5.000` |
| 2 | I | measured output current | `0.000` |
| 3 | P | measured output power (device-computed → no V·I derivation needed) | `0.000` |
| 4 | OVP | over-voltage fault: `1\|ON` fault, `0\|OFF` normal | `OFF` |
| 5 | OCP | over-current fault | `OFF` |
| 6 | OTP | over-temperature fault | `OFF` |
| 7 | MODE | **0 = standby, 1 = CV, 2 = CC, 3 = failure** | `0` with output OFF, `1` with output ON (no load). 2 and 3 never observed |

Parse flags tolerantly: `ON|1` → true, `OFF|0` → false.

### `CONFigure?` (PM p13)
Grammar: `^(VOLT:DC|VOLT:AC|CURR:DC|CURR:AC|RES|CAP|DIOD|CONT) (OL|[+-]\d\.\d{4}E[+-]\d{2,3})$`
(DIOD/CAP/CURR replies are not captured - verify, Q5.)

- **`+0.0000E-308`**: not described in the manual. It is a plain 0 with a degenerate exponent (firmware formatting zero via log10 → −308, the f64 minimum normal exponent). It parses to `0.0` as an ordinary float. It appeared with open DCV inputs (191/196 polls in the read-only session), while other periods showed `+1.0000E-04` / `+1.0000E-03` (1 LSD of noise; the exponent tracks the active range resolution). **Rule: parse as number → 0.0; no special case beyond accepting a 3-digit exponent.** It is not an error indicator (Q3 to confirm).
- **Overload / open circuit:** the value token is literally `OL` (`RES OL`, `CONT OL` with open leads). UM p26: "If 'OL' appears … the input has exceeded the selected range"; UM p24: diode reverse polarity shows "OL". Python passes `OL` into `sci_to_value` → `{"si":{"number":""…},"sci":"OL"}` (no `val`) and the UI parseFloat gives NaN. **Rust: map `OL` → `MEAS = null` + `OVERLOAD = true`** (or a class-defined overload marker); never a number. Negative overload (`-OL`?) unverified (Q5).

### `CONF:ALL?` (PM p12)
4 comma fields: function, value **with unit and display prefix** (`+000.00mA`, `Ohm`), range mode `AUTO|Manual`, range with unit (`2V`, `200Ohm`, `200mA`). Unverified on hardware.

## 5. Recommended poll strategy

Per cycle (target interval ~200-500 ms; each exchange ~10-25 ms → ~30 ms/cycle vs ≈2 s in Python):

1. **`MEAS:ALL:INFO?`** → PSU
   - `VOUT`=f1, `IOUT`=f2, `POUT`=f3 (SI dicts from the numeric string)
   - `CV` = f7==1, `CC` = f7==2, `SBY` = f7==0, `FAIL` = f7==3 OR any of f4..f6 true
   - `OUTPUT` = `"ON"` if f7 ∈ {1,2} else `"OFF"` (Python maps 3 → ON; UM p17 says protection *cuts* the output, so 3 → OFF is likely correct - Q7). Optional raw: `OVP`,`OCP`,`OTP` (bool), `OM` (int).
   - `REMOTE`: **not queryable** (no `SYST:REM?`, PM p12). Use definition shadow state set by `set_remote` (`SYST:REM` / `SYST:LOC`), default `"OFF"`.
2. **`CONFigure?`** → DMM `MODE` (token 1, `:`→`_`), `MEAS` (token 2; `OL` → overload/null).
3. **Range**: queryable from the device, so **no client-side shadow state is needed** - either
   - (a) `CONF:ALL?` (fields 3-4 give `AUTO|Manual` + range, and it could replace `CONFigure?` entirely, but its value field carries display units/prefixes that need extra parsing), or
   - (b) `<FUNC>:RANG?` (+ `:RANG:AUTO?`) built from the current `MODE` token.
   Both are unverified (Q4). Since range only changes via our own setters or the front panel, poll it at a slow interval (e.g. every 5th cycle) and invalidate on any DMM setter. `RANGE` = `"AUTO"` when auto, else the range value; `null` for DIOD/CONT.

Avoid: `MEAS:VOLT?/CURR?/POW?` (3 commands for what INFO gives in one), `OUTP?` (derivable from f7), `*IDN?` per poll.

Setter effects: PSU setters → invalidate nothing in the poll (INFO is always read fresh); setpoint queries (`VOLT?`,`CURR?`,`*:LIM?`) are on-demand only. DMM `FUNC:*` / `*:RANG*` → invalidate cached range.

## 6. Discrepancies

### Manual vs device
| Item | Manual | Device |
|---|---|---|
| `OUTP?` reply | `1`/`0` (PM p10) | `ON`/`OFF` |
| `MEAS:ALL:INFO?` separators / flags | spaces, `1|ON`/`0|OFF` (PM p9) | commas, `ON`/`OFF` |
| `MEAS:ALL?` | 3 values, space-separated (PM p8) | **2** values, comma-separated (`0.000,0.000`) |
| IDN firmware | `FV:X.XX.XX` | `FV:V2.1.0` |
| Zero reading | `+4.0000E-04`-style examples (PM p13) | `+0.0000E-308` for exact zero |
| Link params / terminators | not documented | 115200 8N1, reply `\n` |

### Python driver vs device/manual (do not replicate)
- `manifest.json` `reol: "\r"` but device ends with `\n` → every read is a 1 s timeout (poll ≈2 s for 2 commands).
- Every setter (`set_output`, `set_voltage`, `set_current`, `set_*_limit`/`set_o?p_value`, `set_*_range`, `set_*_mode`) reads a reply that never comes → +1 s each.
- `query_voltage_limit` / `query_current_limit` **defined twice** (driver.py:161-169 and 185-189; second definition wins, delegating to `query_ovp_value`/`query_ocp_value` - same wire traffic).
- Debug `print()` in `query_measurement` (driver.py:42, 46).
- `poll_status_psu`: `parts[6]` IndexError on a short/garbled reply; `raw is None` check is dead (`or ""`); unknown mode value leaves CV/CC/SBY/FAIL/OUTPUT unset; `FAIL` ignores the OVP/OCP/OTP flags; mode 3 reported as `OUTPUT:"ON"`; `REMOTE` hardcoded `"OFF"`; OVP/OCP/OTP passed as raw strings.
- `RANGE` is always `null`: `set_*_range` stores under `cache["function"] + ":RANGE"` = `"VOLT:DC:RANGE"`, while `poll_status_dmm` reads `"VOLT_DC:RANGE"` (normalised). If the 5 s `function` cache has expired, `set_*_range` raises `TypeError` (`None + str`). Range is never queried from the device although the manual offers `RANG?` / `CONF:ALL?`.
- `VOLTage:DC:RANGe AUTO` is not valid per manual (auto is `…:RANGe:AUTO ON`, PM p16). Capture supports that it was ignored: after `VOLT:DC:RANG 20` the reading resolution changed `1E-04`→`1E-03` and stayed `1E-03` for >2 min after `…:RANGe AUTO` (meter probably left in manual 20 V range - Q8).
- `set_capacitance_range` sends `CAPacitance:RANGe <v>` - no such setter (PM p18, UM p26); manifest offers CAP ranges + `AUTO`.
- API float coercion rewrites range tokens: POST `resistance_range/2E3` → wire `RESistance:RANGe 2000.0`. Rust must send enum tokens verbatim.
- No `set_range` (class contract ⚠️) and no `set_remote` (→ 400) although `SYST:REM`/`SYST:LOC` exist.
- `set_mode` silently ignores unknown keys (204 with no wire traffic). `set_output` sends `str(value)` → API bool coercion yields `OUTP True` (invalid) for `true/false`.
- `_query_measurement`/`query_measurement`: `parts[1]` IndexError on a one-token reply; `OL` flows into `sci_to_value` → NaN in UI. `query_function` falls back to undocumented `FUNCtion?`.
- `query_output_voltage/current/power`, `query_status`, `query_output_all` return raw strings while `query_voltage` etc. return SI dicts (inconsistent API value types).

## 7. Open questions (verify on hardware)

| # | Question | Kind |
|---|---|---|
| Q1 | Is `\n` (and `\r\n`) accepted as TX terminator? Test `*IDN?\n`. | read-only |
| Q2 | Minimum gap after a no-reply setter before the next command (Tenma needed ≥50 ms); also delay after `FUNC:*` before `CONFigure?` shows the new mode and before a `RANG` setter is accepted. | state-changing |
| Q3 | Is `+0.0000E-308` always an exact zero (e.g. shorted DCV leads on fixed range), never "no reading yet"? | read-only (with shorted leads) |
| Q4 | Reply format of `CONF:ALL?`, `VOLT:DC:RANG?`, `…:RANG:AUTO?`, `RES:RANG?`, `CURR:DC:RANG?`, `CAP:RANG?`. Decide poll option 3a vs 3b. | read-only |
| Q5 | `CONFigure?` replies for `CURR:DC/AC`, `CAP`, `DIOD`, `CONT` (closed), and negative overload (`-OL`?). | state-changing (function switch) |
| Q6 | Do `CURR:{DC\|AC}:RANG` setters work (UM p9 says no manual current range)? Is there `CURR:…:RANG:AUTO`? | state-changing |
| Q7 | Behaviour on OVP/OCP trip: f7 = 3? flags ON? does `OUTP?` read `OFF`? how to clear (re-enable output)? Also force CC (f7 = 2) with a load. | state-changing (needs load / low limits) |
| Q8 | Does `VOLT:DC:RANG:AUTO ON` restore auto range (and is the bench currently stuck in manual 20 V range from the setter capture)? Check with `CONF:ALL?` first. | read-only check, then state-changing |
| Q9 | Does any remote command lock the front panel? Does `SYST:REM` change behaviour; is there an undocumented `SYST:REM?`? | state-changing (`SYST:REM`/`SYST:LOC`) |
| Q10 | Does `MEAS:ALL?` really return only V,I (which two)? | read-only (output on, with load) |
| Q11 | Setpoint formatting: are >3 decimals or scientific notation (`1.5e1`) accepted or silently ignored (as on Tenma)? Recommend `%.3f`. | state-changing |
| Q12 | Do PSU and DMM queries interfere (e.g. `CONFigure?` latency spikes to 40 ms; any lost replies at high poll rate)? Run a fast-poll soak. | read-only |
