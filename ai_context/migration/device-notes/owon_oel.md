# OWON OEL15/30 electronic load (`owon_oel`, class ELL)

Bench unit: **OEL1515**, `OWON,OEL1515,25130087,FV:V1.1.0`, device id `owon-oel1515`, via FTDI FT232R USB-serial.

**Sources and citations.** `PM p.N` = *OEL15_30 Series SCPI Programming Manual* (Jul 2025 V1.0.0), **PDF** page N (printed page = N−4).
`UM p.N` = *OEL15&30 Series DC Electronic Load User Manual* (May 2025 V1.0.1), PDF page N (printed = N−5).
`CAP-L` = `captures/20261016-201609` (LOCAL mode). `CAP-R` = `captures/20261016-203514` (REMOTE mode + setters).
`PY` = `benchmesh-serial-service/src/benchmesh_service/drivers/owon_oel/`.
The OEL85 programming manual has the **same command set and the same `MEAS:ALL:INFO?` text**, so OEL85 can likely share this definition (not verified on hardware).

## 1. Identity

| Item | Value |
|---|---|
| `*IDN?` (PM p.11) | Documented: `<Factory>,<model>,<serial>,XX.X.X.X.X`. Observed: `OWON,OEL1515,25130087,FV:V1.1.0\n`, 32 B, first byte after 1.4–39 ms. Firmware field is `FV:V…`, not a bare version. |
| Works in LOCAL mode | Yes (CAP-L: 75/75). It is the only reliable presence check before `SYST:REM`. |
| Model match | `^OWON,(OEL(15(15\|20\|30)\|30(15\|20\|30)))(T)?,` (the `T` suffix adds battery/OCP/OPP test functions, UM p.31–37; the exact IDN form of T units is **unknown**). OEL85 family: `OEL851[123][BC]?`. |
| Ratings (UM p.8) | 1515: 150 V/15 A/150 W · 1520: 150/20/300 · 1530: 150/30/300 · 3015: 300/15/150 · **3020: 300/20/300** · 3030: 300/30/300. Voltage ranges: low 0–15 V, high 0–150 V (UM p.25, stated for 150 V models). |

## 2. Link

| | Manual | Observed |
|---|---|---|
| Ports | USB (CH340 CDC), RS232 on DB9 pins 2/3/5, RS485 (for parallel use). Only one of RS232/RS485 can be active (UM p.11, p.28) | FTDI FT232R adapter (probably on RS232) |
| Baud | Default **9600 8N1** (UM p.28). Menu choice of 8 rates from 1200 to 115200 (UM p.24) | **115200 8N1** (the panel setting was changed) |
| Protocol | Menu option `COMM PRO`: SCPI or Modbus RTU (UM p.24, p.28) | SCPI |
| TX terminator | Not documented | `\r` works (all CAP) |
| RX terminator | Not documented | **`\n` only** (no `\r`). Every query reply ends in `\n`, **including the empty LOCAL-mode reply `\n`** |
| Latency | — | First byte arrives after 3–42 ms (median ≈10–21 ms). Every Python exchange is ≈1020 ms because PY waits for `\r` (manifest `reol: "\r"`) and so always runs into the 1 s timeout |
| Setters | "Return format: None" | Never reply (`no_reply` for every setter in CAP-R) |
| DTR/RTS | — | Forced low by PY. Not shown to be required |

## 3. Command table

The SCPI tree has an optional `[SOURce:]` root, and `FUNC` and `MODE` are aliases (PM p.17).
Numeric setpoints default to the base unit and accept `MIN`/`MAX` (PM p.21, p.25, p.28).
**Precondition for every row except `*IDN?` and `SYST:*`: the load must be in REMOTE mode** (see §4).
In LOCAL mode every query returns `\n`, and setters are presumably ignored (not verified).

| Function | Command / query | Params | Reply (manual → observed) | Pre-cond. | Cite / capture |
|---|---|---|---|---|---|
| Identify | `*IDN?` | — | see §1 | none | PM p.11 · CAP-L/R |
| Reset | `*RST` | — | none | remote? | PM p.11 · not captured |
| Remote | `SYST:REM` | — | none → none | — | PM p.13 · CAP-R t=316 |
| Local | `SYST:LOC` | — | none → none. After it, `MEAS:ALL:INFO?` → `\n` | — | PM p.13 · CAP-R t=413–414 |
| Remote state | `SYST:REM?` | — | **not documented** → `1\n` in remote. Plan Log: `0` in local | none | CAP-R t=43, 318 |
| Remote sense | `SYST:SENS[:STAT] <bool>` / `?` | `0\|1\|OFF\|ON` | `0\|1` → query **not captured** | remote | PM p.13–14 · CAP-R t=383, 387 (setters only) |
| Mode select | `FUNC <f>` / `FUNC?` | `CURR\|VOLT\|POW\|RES\|DYN` | `CURRent…` → **lowercase long words**: `current`, `voltage`, `resistance`, `power` (`dynamic` not observed) | remote | PM p.17 · CAP-R t=320–353 |
| Input | `INP <bool>` / `INP?` | `0\|1\|OFF\|ON` | `0\|1` → **`ON`/`OFF`** | remote | PM p.17–18 · CAP-R t=359–371 |
| Short | `INP:SHOR <bool>` / `?` | `0\|1\|OFF\|ON` | `0\|1` → **`ON`/`OFF`** | remote | PM p.18 · CAP-R t=375–381 |
| CC setpoint | `CURR <A>` / `CURR?` | MIN~MAX, A | NR2 `5.0` → `%.3f` (`0.500`) | remote, **FUNC=CURR** (gating not tested for CURR) | PM p.25 · CAP-R t=355–357 |
| CV setpoint | `VOLT <V>` / `VOLT?` | MIN~MAX, V | → `%.3f` (`10.000`) | remote, **FUNC=VOLT** (ignored otherwise: t=395–398) | PM p.22 · CAP-R t=327–329 |
| CR setpoint | `RES <Ω>` / `RES?` | MIN~MAX, Ω | → `%.2f` (`100.00`) | remote, **FUNC=RES** (ignored otherwise: t=400–402) | PM p.26–27 · CAP-R t=336–339 |
| CP setpoint | `POW <W>` / `POW?` | MIN~MAX, W | → `%.3f` (`1.000`) | remote, **FUNC=POW** (ignored otherwise: t=404–406) | PM p.29 · CAP-R t=346–348 |
| Current limit | `CURR:LIM <A>` / `?` | MIN~MAX | NR2 | remote | PM p.26 · not captured |
| Von | `VOLT:ON <V>` / `?` (manual also writes `VOLT:[LEV:]ON`) | V | NR2 | remote | PM p.21–22, UM p.30–31 · not captured |
| V range | `VOLT:RANG <V>` / `?` | value → low/high range | NR2 | remote | PM p.18–19 · not captured |
| I range | `CURR:RANG <A>` / `?` | value → low/high range | NR2 | remote | PM p.22–23 · not captured |
| V slew | `VOLT:SLEW[:BOTH\|:RISE\|:FALL] <p>` / `?` | `FAST\|NORM\|SLOW` | same enum | remote | PM p.19–20 |
| I slew | `CURR:SLEW[:BOTH\|:RISE\|:FALL] <A/µs>` / `?` | number (FALL row is contradictory: lists the enum as param) | NR2 | remote | PM p.23–24 |
| R slew | `RES:SLEW:RISE\|FALL <p>` | `FAST\|NORM\|SLOW` | enum | remote | PM p.27–28 |
| P slew | `POW:SLEW:RISE\|FALL <p>` | `FAST\|NORM\|SLOW` | enum | remote | PM p.29–30 |
| OVP level | `VOLT:PROT[:LEV] <V>` / `?` | MIN~MAX\|MIN\|MAX | NR2 | remote | PM p.21 |
| OCP level | `CURR:PROT[:LEV] <A>` / `?` | same | NR2. The text mentions `CURR:PROT:DEL`, which is **not documented** | remote | PM p.24–25 |
| OPP level | `POW:PROT[:LEV] <W>` / `?` | same | NR2. Mentions `POW:PROT:DEL` (not documented) | remote | PM p.28–29 |
| OTP | — (fixed at ≈85 °C, no command) | — | — | — | UM p.27 |
| Protection clear | **no SCPI command documented** (cleared with the panel ENTER or power key) | — | — | — | UM p.26 |
| Dynamic | `DYN:HIGH[:LEV]`, `DYN:LOW[:LEV]` (A); `DYN:HIGH:DWEL`, `DYN:LOW:DWEL` (s); `DYN:SLEW[:RISE\|:FALL]` (A/µs); `DYN:MODE CONT\|PULS` | MIN~MAX | NR2 / `PULS` | remote, FUNC=DYN? | PM p.32–36, UM p.29–30. **No SCPI trigger** for pulse mode (panel `SHIFT+.`) |
| Measure | `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` | — | NR2 | remote | PM p.38–39 · not captured |
| Measure all | `MEAS:ALL:INFO?` | — | `V,I,P,f1,f2,f3` → `0.000,0.000,0.000,OFF,OFF,OFF\n` (330/335). LOCAL: `\n` | **remote** | PM p.39 · CAP-R, CAP-L |
| — | `MEAS:ALL?` | undocumented | `ERR` (Plan Log, local mode) | — | not in transcripts |
| Parallel | `PAR[:STAT]?`, `PAR:ID?` (`Master`/…), `PAR:NUM?` | — | NR2 / string | remote? | PM p.41–42 · not captured |
| Error queue | **none documented** (no `SYST:ERR?`) | — | — | — | — |
| Battery / OCP-test / OPP-test / LIST | **not in SCPI** (panel only, T-suffix models) | — | — | — | UM p.31–37 |

## 4. Remote mode and mode gating

- **Remote is documented, but its effect on queries is not.** PM p.13 for `SYST:REM`: "panel key is locked … **All control commands need to be operated after this command**". UM p.14–15: the unit powers up in **local**. In remote mode every key is inactive except a **long press of SHIFT+On/Off, which returns the unit to local**. Neither manual says that *queries* return empty in local mode. CAP-L and CAP-R show that they do.
- **Mode gating is only implied.** PM describes VOLT/RES/POW/CURR as "the set X in CV/CR/CP/CC mode" (p.22, 25, 27, 29). The manuals never say that the setter is *silently dropped* in another mode. CAP-R shows exactly that.
- **Recommended sequence:**
  1. Open the port, send `*IDN?` (works in either mode), then `SYST:REM`. No reply comes back, so give it a short settle time (measure it, §8).
  2. Poll as in §6.
  3. If a query returns an empty line, the unit is in local mode (a user long-pressed SHIFT+On/Off). Report `REMOTE=OFF`, keep the link and `IDN`, and **do not treat it as a link failure** (PY does, which causes its reconnect loop). Whether to re-send `SYST:REM` automatically should be a definition/config flag. The default should be to re-assert only on an explicit `POST remote/ON`, so the user's panel takeover is respected.
  4. On disconnect or shutdown, send `SYST:LOC` so the panel unlocks.
- **Setpoints on a non-active mode:** reject with a clear error (`409: VOLT setpoint requires MODE=VOLT`). Do not switch `FUNC` implicitly: changing mode while `INPUT=ON` changes the load on the DUT.

## 5. Field and enum semantics

`MEAS:ALL:INFO?` returns 6 comma-separated fields, with no units and 3 decimals:

| # | Meaning | ELL key | Evidence |
|---|---|---|---|
| 1 | input voltage V | `VIN` (SI) | PM p.39 example `9.496,20.000,189.918`: 9.496×20.000 = 189.92 → V,I,P |
| 2 | input current A | `IIN` (SI) | same |
| 3 | input power W | `PIN` (SI) | same |
| 4 | OVP fault `ON`/`OFF` | `OVP` | PM p.39 "fault status of OVP, OCP, and OPP" |
| 5 | OCP fault `ON`/`OFF` | `OCP` | same |
| 6 | **OPP** per manual; PY calls it **OTP** | `OPP` (+`OTP`?) | ⚠ conflict. OTP may not be reported at all. Verify (§8) |

The manual does not say whether the readings are averaged or instantaneous (`MEAS:VOLT?` says "average", p.38).

`FUNC?` → manifest key mapping: `current`→`CURR`, `voltage`→`VOLT`, `resistance`→`RES`, `power`→`POW`. `dynamic`→`DYN` is **assumed** and not observed. Match case-insensitively on the prefix (`curr`, `volt`, `res`, `pow`, `dyn`) so that the manual's long and short forms also parse. Pass an unknown value through raw (e.g. a T-model battery mode); never return null.

## 6. Recommended poll (ELL keys)

Each query takes about 10–20 ms, so one full cycle takes under 100 ms. A 0.5–1 s interval is enough.

| Every cycle | Produces |
|---|---|
| `MEAS:ALL:INFO?` | `VIN`,`IIN`,`PIN` (SI from fields 1–3); `OVP`,`OCP`,field 6 (`ON`/`OFF`) |
| `INP?` | `INPUT` = reply as-is (`ON`/`OFF`). Poll it every cycle, because a protection trip turns the input OFF without any command |
| `FUNC?` (every cycle, or cached and invalidated by `set_mode`/remote re-entry) | `MODE` via §5 map |
| derived | `REMOTE` = `ON` if the replies above are non-empty, otherwise `OFF`. Alternatively poll `SYST:REM?` (`1`/`0`), which also answers in local per the Plan Log |

In local mode the empty reply makes the cycle emit only `REMOTE=OFF` and keep the previous IDN. Do not emit stale measurements.

## 7. Discrepancies

| Topic | Manual | Capture | Python |
|---|---|---|---|
| Baud | 9600 default | 115200 | config-driven (115200) ✓ |
| RX EOL | — | `\n` | manifest `reol: "\r"` → every read times out after 1 s |
| `FUNC?` reply | `CURRent`/`RESistance` (mixed case) | lowercase long words | maps lowercase; returns **None** for anything else |
| `INP?`, `INP:SHOR?` | `0\|1` | `ON`/`OFF` | raw passthrough (UI compares to `ON` → OK); CLASS_CONTRACT ⚠ resolved |
| Setpoint reply format | NR2 `5.0` | `10.000`, `100.00` | raw string |
| Field 6 of INFO | OPP | `OFF` (unknown) | named `OTP` |
| `REMOTE` | — | real state varies | **hardcoded `"ON"`**. `query_remote` exists but poll ignores it |
| Local mode | not stated | all queries `\n` | `{}` → link dropped → reconnect loop every 3–6 s (CAP-L, CAP-R t>414); unit unusable |
| No `SYST:REM` on connect | PM: "all control commands need to be operated after this" | required | never sent automatically |
| Setter replies | none | none | reads after every setter → +1 s each |
| Mode gating | implied | VOLT/RES/POW dropped silently | no check. Setters "succeed" (204) while doing nothing |
| `SYST:SENS?` | documented | — | no query method; setter named `remote_compensation` |
| OEL3020 power | 300 W (UM p.8) | — | README says 150 W; manifest inherits 150 W |
| POW setpoint max | per model (150/300 W) | — | DEFAULT `POW.max=200` (> 150 W for 1515/3015). RES `0.05–1000` and POW `min 0.5` are unsourced |
| CR mode keys | CR = SHIFT+**CV**, CP = SHIFT+**CC** in UM p.18, p.20; UM p.9–10 says the opposite | — | n/a (UM is self-contradictory) |
| Capture restore | — | `setters-bench.yaml` "restore" left **VOLT=10, RES=100, POW=1** (not 55/200/2) because of gating. **Restored afterwards outside the capture** (mode-by-mode, verified by read-back: CURR 1.000, VOLT 55.000, RES 200.00, POW 2.000, INP/SHOR/SENS OFF, local) | — |
| Node-RED | — | — | calls `set_current/voltage/power`, but the driver has `set_curr/volt/pow` |

## 8. Open questions to verify on hardware

R = read-only, S = state-changing (nothing is connected to the input, so S is safe per the plan).

1. R: In LOCAL mode, does `SYST:REM?` answer `0` (Plan Log) while measurement queries return `\n`? Use it as the authoritative `REMOTE` source.
2. R: What does field 6 of `MEAS:ALL:INFO?` mean (OPP or OTP)? Read `POW:PROT?` and `VOLT:PROT?`. A trip test (S) would need a source, so skip it unless one is available.
3. R: Do `MEAS:VOLT?`, `MEAS:CURR?`, `MEAS:POW?` answer, and do they match fields 1–3?
4. R: Find the setpoint limits: `CURR? MAX`, `VOLT? MAX`, `RES? MIN`, `RES? MAX`, `POW? MAX`, and the `*:PROT? MAX` variants (MIN/MAX is documented only for the setters).
5. R: Is there an error reporting mechanism? Try `SYST:ERR?` and a bogus `FOO?` (does it return `ERR` like `MEAS:ALL?`?).
6. R: Read `SYST:SENS?`, `CURR:LIM?`, `VOLT:ON?`, `VOLT:RANG?`, `CURR:RANG?`, the `*:SLEW?` queries, `DYN:*?` and `PAR?`. Record the reply formats.
7. R: Is `\n` accepted as the TX terminator, and is `\r\n` accepted?
8. S: Is `CURR` also gated (send `CURR x` while FUNC=VOLT)? Does a gated setter produce any error indication?
9. S: What do `FUNC DYN` → `FUNC?` return (expected `dynamic`)? Is `DYN:*` gated by FUNC=DYN?
10. S: What is the minimum gap after a no-reply setter (`SYST:REM`, `FUNC`, `INP`, setpoints) before the next command, similar to Tenma's ≥50 ms? Sweep 0/10/20/50 ms with read-back.
11. S: Does a setter sent in LOCAL mode get ignored, or does it apply or switch the unit to remote?
12. S: Does `*RST` leave the unit in remote? Which values does it reset?
13. ~~S: Restore the bench~~ — done 2026-10-16 after the setter capture (verified by read-back).
