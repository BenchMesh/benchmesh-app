# Class features: the common functional denominator and the software layer

Status: research result (2026-10-20), input for the next contract, definition and panel work.
Detailed reports, with feature matrices and manual page citations, are in `ai_context/features/`:
`AWG.md`, `PSU.md`, `DMM.md`, `ELL.md`, `OSC.md`, `LCR_SAL.md`, and `SOFTWARE_LAYER.md` (where
computed and emulated features live).

## Why

The bench panels expose little more than the simplest case of each class: the DGE2070 can sweep,
modulate (AM/FM/PM) and burst, but the AWG panel only offers a fixed sine/square/... with a
frequency. The research compared 8 to 15 families per class, from the cheapest instruments
(FY6900, Tenma/Korad, OWON XDM, Atorch, tinySA) to the pro ones (Keysight 33500B, E36300,
DMM6500, E4980A), and sorted every feature into tiers:

| Tier | Rule | In the product |
|---|---|---|
| **Core** | (practically) every family, including the cheapest | the panel always offers it; the contract makes it standard |
| **Common** | most families | the panel shows it when the definition binds it |
| **Advanced** | niche, options, heavy data | later, or through flows / the instrument console |

The second axis is **who provides the feature**. A feature is Core for the *user* when software
can provide it for every instrument, even when few instruments have it: power from V x I on every
PSU is the model case, and the same holds for period entry on every AWG, statistics on every DMM
or energy (Ah/Wh) on every PSU and load. The value BenchMesh adds is largest exactly where the
instruments are cheapest.

## The software layer (one rule: who must agree on the value)

| Layer | What | Lives in | Examples |
|---|---|---|---|
| L0 native | the instrument's own reading or setter | definition | DP800 measured power |
| L1 derived | stateless, from one status update | class contract `derive` (new), definition overrides | POUT = VOUT x IOUT, PIN, RIN = V/I, AWG PERIOD / HIGH / LOW / Vrms, LCR ESR/Q/Cs/Cp/abs(Z), SAL peak level/frequency |
| L1s translated setters | enter one quantity, send another | class contract (new), server | AWG period -> frequency, Vrms/dBm/high-low -> Vpp + offset, pulse width -> duty |
| L2 stateful signals | needs memory across readings | `bm-server` (new module) | Ah, Wh, on-time, min/max/avg/std dev, REL/null, dB/dBm reference, mx+b, limits pass/fail, LCR sorting bins |
| L2g guards | best-effort limits | `bm-server` (new module) | software OVP/OCP trip that only switches OFF; AWG output-limit guard at the DUT |
| L3 procedures | send commands over time | built-in automation flows | software list/ramp, stepped AWG sweep, battery discharge, PSU OCP trip test, I-V curve, LCR frequency sweep |
| UI | presentation only | `frontend/src/ui` | cursors, overlays, unit toggles |

Findings behind it:

- Twelve PSU definitions each compute `vout * iout` in their own file (Tenma: `devices/tenma_72.yaml:119`), one of
  them (SPD4000X) although the instrument measures power. A class-level `derive` gives every
  definition the value once, and the native value always wins.
- A status expression only sees command results, so integrators, min/max and REL cannot be
  written in a definition. They belong on the server, behind the event closure in `Devices::start`, so
  recordings, automation, MCP, remote browsers and every window see the same value.
- Today the DMM statistics, HOLD and REL, the LCR derived values and the SAL max-hold live in
  React state: not recorded, lost on reload, different per window. The DMM statistics also count
  every WebSocket push (200 ms) instead of each new reading (bug).
- Software sweeps, lists and tests already work as flows; built-in templates need XY result tables,
  `pow`/`sqrt`/`log10` in flow CEL and a fast-set mode (confirmation takes up to 6 s per step).

Every computed or emulated value carries its **provenance** (instrument / computed / software,
with the formula) in `/instruments/{CLS}/{id}` features, `/methods` (`emulated: true`), recordings
(series source), MCP and the panel (`calc` / `SW` tags). Safety rules (detail in
`SOFTWARE_LAYER.md` section 4):

1. Software protection never replaces hardware OVP/OCP: it reacts after a poll plus a command
   exchange (0.3 to 1.5 s) and is blind when the link drops. Where the instrument has hardware
   protection, the panel offers that first.
2. Emulated actions only move to the safe direction (output/input OFF, setpoint down); nothing
   emulated energises without `arm` or the two-press rule.
3. Stale inputs give no value, never an old one; an emulated setter that cannot be evaluated
   (Vrms of noise, dBm at High-Z) refuses with a reason instead of guessing.
4. Some behaviour must stay native: modulation, burst and fast sweeps (AWG), exact statistics at
   the meter's own rate (DMM), hardware counters and full-record FFT (OSC), RMS-detector
   measurements and quasi-peak (SAL), CV mode on a CC-only load.

## Per class

### AWG (13 families)

- **Core:** output, waveform, frequency **with period entry**, amplitude, offset, phase,
  **frequency sweep** (on/off, start, stop, time, LIN/LOG: 13 of 13), **AM / FM / PM** (12 of 13),
  **N-cycle burst** (12 of 13), plus software readouts Vrms/dBm, high/low, pulse width.
- **Common:** duty, symmetry, load 50 ohm / High-Z, pulse width and edges, FSK, sweep/burst trigger
  and a Trigger action, gated burst, native units, channel coupling, counter.
- **Advanced:** ASK/PSK/PWM/SUM, hold/return times, list sweep, harmonics, arbitrary upload.
- **Gap:** no definition of 24 binds duty, load, mode, modulation, sweep or burst; the panel ignores
  phase, symmetry and load although almost all definitions bind them.
- **OWON DGE2070:** SCPI has load, modulation (AM/FM/PM), sweep (incl. center/span), burst and
  pulse width/edges (`AWG.md` section 4.4 with syntax and pages). Not over SCPI: FSK, period,
  high/low, Vrms/dBm, DC, built-ins (software covers period, high/low, Vrms). No remote trigger
  and no input connector (offer internal sources only). Bench checks needed: `MODE:STATe` vs
  `AM:STATe`, the printed `SUQare` token.

### PSU (12 families in the matrix, 25 definitions)

- **Core:** V/I setpoints, measured V/I, output (some are global-only: the panel must say so),
  CV/CC, model limits, **power** (7 of 12 measure it, 12 of 12 with V x I).
- **Common:** OVP/OCP levels, enable and trip status, trip clear, keylock/remote, memories,
  series/parallel/tracking, OTP, beeper, list/timer, output delays, all-outputs command.
- **Advanced:** remote sense, recorder, condition trip, ranges, slew, statistics, Ah/Wh (Riden only).
- **Software:** power, R = V/I, Ah/Wh and on-time, statistics, CV/CC inferred where not reported
  (OWON ODP/P4000, TTi), presets stored in BenchMesh (recall never switches on), output-off
  timer, all-off action, ramps/lists as flows, efficiency with a DMM or load.

### DMM (15 families)

- **Core:** DCV/ACV/DCI/ACI, 2W/4W ohm, continuity, diode, capacitance, frequency, period,
  temperature, auto/manual range, speed, overload; null/REL, min/max/avg and dB/dBm exist in 14 of
  15 but are spelled seven different ways, so the software does them once for all.
- **Common:** limits, trigger/sample count, buffer, input impedance, auto zero, secondary display,
  std dev, AC filter, continuity threshold.
- **Software:** REL, dB/dBm with any reference, mx+b sensor scaling, percent, limits, statistics
  per new reading (exact statistics stay with the meter's own math or buffer), V x I across two
  meters with fresh readings only.
- **Bugs found:** statistics count pushes not readings; wrong full scale for `rigol_dm3000` range
  indices; fixed 5 digits; temperature unit label not following the scale; speed read only once.

### ELL (10 families)

- **Core:** CC setpoint, measured V/I, input on/off (power via software).
- **Common:** CV/CR/CP, short, transient, list, battery test, OCP/OPP test, Von/latch, slew, ranges,
  sense, protection status.
- **Software:** P, R, Ah/Wh/time, software battery discharge (stopping = input OFF, arm the load's
  own cut-off where it exists), CP/CR closed loop on CC-only loads (slow sources only), I-V curves,
  internal resistance, PSU OCP trip test.
- **Quick wins:** a Short control (bound in 4 definitions, missing in the panel), modes without a
  setpoint dropped from the selector, the Sense switch shows a guess, protection bits parsed but
  not reported.

### OSC (13 families)

- **Core (17):** all in the contract and bound by all 14 definitions.
- **Common (22):** only 4 bound (BW limit, invert, acquisition, sample rate); missing: force trigger,
  trigger coupling/holdoff/type, memory depth, average count, extended measurements and
  statistics, cursors, math, FFT, screenshot.
- **Software:** measurements computed from the downloaded trace for every scope with one definition
  across vendors (Vmax/Vmin/mean/RMS, frequency, duty, widths, rise/fall, delay/phase, clipping),
  labelled as computed; math between channels only on a stopped (coherent) acquisition.
- **Bug:** the BW-limit switch is hard-coded to 20M.

### LCR (8 families) and SAL (9 families)

- **LCR Core:** primary/secondary, L/C/R/Z with D/Q/theta/ESR, SER/PAR, frequency, level, auto
  range, open/short correction. **Software:** all 13 impedance parameters from any pair plus the
  frequency (held back when D is too small for a useful ESR), deviation % and tolerance bins,
  stable-reading capture, frequency sweep as a flow.
- **SAL Core:** frequency axis, RBW, ref level, attenuation, trace, marker with peak search,
  continuous/hold. **Software:** peak level/frequency and noise floor as recordable values, extra
  and delta markers, noise marker per Hz, units, max/min/average, reference trace, limit masks,
  channel power/OBW/ACPR, waterfall, export.

## Order of work (proposal)

1. **UI wins on what is already bound:** AWG period toggle, Vrms/dBm/high-low display and entry,
   phase/symmetry/load; ELL short and modeless modes; DMM statistics per reading and the other
   DMM bugs; OSC BW-limit values.
2. **Class `derive` (L1) with provenance:** delete the duplicated POUT expressions and the browser
   fallbacks; AWG period/high/low/Vrms, ELL RIN, LCR impedance set, SAL peak values.
3. **AWG Core in the contract and the bases:** `mode` CW/MOD/SWEEP/BURST, sweep, AM/FM/PM, burst,
   load, duty; a DGE bench session for the traps above and a capture for the replay tests.
4. **Server signals (L2):** energy, statistics, REL/dB/mx+b/limits, moving the DMM math out of the
   browser.
5. **PSU/ELL Common:** protection status and clear, all-off, memories (native or BenchMesh presets),
   tracking.
6. **Built-in procedures (L3):** battery discharge, software list/ramp, stepped AWG sweep, OCP trip
   test, I-V curve, LCR sweep, with the executor additions they need.
7. **Guards (L2g)** once the "software, best effort" wording is agreed; OSC computed measurements,
   cursors and math.

DSL additions this needs (collected from the reports): class `derive`, math functions in CEL
(`sqrt`, `pow`, `log10`, `atan2`) and list functions over traces, polls gated by the current mode,
limits that depend on other settings or the model, values that may be a number or `INF`, setters
with several parameters, GET with a parameter (one `measure` operation), chunked full-record reads.
