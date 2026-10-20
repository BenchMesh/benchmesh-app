# Software layer: computed and emulated instrument features

Research for BenchMesh (develop @ 2fd8e06), read-only. Question: where should values and
procedures live that an instrument does not provide itself (Tenma has no power reading, a PSU
has no Wh counter, an AWG has no period entry, a load without a battery mode), so that every
instrument of a class gets them, with honest provenance and the same safety rules.

## 1. What the platform has today (facts)

### 1.1 DSL status expressions (L1 already exists, per definition)

- `classes.<CLS>.status.<KEY>` is a CEL expression, optionally `{expr, as: si}`
  (`ai_context/migration/DEVICE_DSL.md`, "classes.<CLS> (binding)" and "CEL environment").
- Evaluation: `bm_driver::status::CompiledStatus::evaluate` (`crates/bm-driver/src/status.rs`).
  Variables are only: poll command results by command name, `state`, `ch`
  (`bm_driver::expr::Vars`, `crates/bm-driver/src/expr.rs`). A status expression **cannot see
  other status keys** - `POUT` cannot say `VOUT * IOUT`, it must repeat the command names
  (`vout * iout`) and any parsing the other keys do.
- A failing expression (null operand, type error) yields `null` for that key, never a made-up
  value (`evaluate`, `unwrap_or(Value::Null)`); `represent()` turns numbers into SI dicts
  (`bm_driver::si::si_value`). SI dicts carry no provenance (`{si, sci, val}`).
- Custom CEL functions: only `num(x)` (`expr.rs::num`). No `abs/min/max/round/sqrt/log10`
  in definitions (automation adds `abs/min/max/round`, see 1.4).
- **State across polls: none usable for signal processing.** `Engine.latest`
  (`crates/bm-driver/src/engine.rs`, struct `Engine`) only keeps the last result per command
  so slower poll groups persist; `state` variables change only through command `effects`
  (setter side), never per poll. So integrators (Ah, Wh), min/max/avg, REL, max-hold,
  time-on-output cannot be expressed in a definition. They need a stateful component.
- Setter side: params support `from:` (CEL over cached query results, computed param) and
  `scale`; operations can be step lists with `only: [values]`. There is no class-level
  "virtual setter" (enter period, send frequency).

### 1.2 POUT / PIN today (grep over 31 PSU/ELL definitions, bases resolved)

POUT is `required: true` in `devices/classes/PSU.yaml` ("measured or V*I"), PIN required in
`ELL.yaml` ("Measured input power"). The validator enforces binding
(`crates/bm-devicedef/src/validate.rs::check_binding`, "required status key missing").

| Computed in CEL from V and I (12) | Read from the instrument (13 PSU + 6 ELL) |
|---|---|
| `tenma_72` (`vout * iout`, `devices/tenma_72.yaml:119`), `korad_ka`, `juntek_dpm86xx`, `gwinstek_gpd`, `gwinstek_gpd3303d` (base `gwinstek_gpd`), `hantek_pps` (`ch == 1 ? v1*i1 : v2*i2`), `manson_hcs`, `manson_hcs_10ma` (both decode `getd` twice), `siglent_spd3303x`, `siglent_spd4000x` (`meas_v * meas_i`), `tti_cpx`, `tti_mx` (base `tti_psu`) | PSU: `gwinstek_gpp`, `siglent_spd1000x` (`meas_p`), `owon_odp/p4000/spe/spm` (`num(info.p)`), `riden_rd6006/6006p/6012` (`st.pout`), `rigol_dp700/dp800`, `unit_udp3000s/udp6700` (`num(meas_all.p)`). ELL: all six (`bk_8600`, `itech_it8500plus`, `rigol_dl3000`, `siglent_sdl1000x` via `scpi_load`, `korad_kel10x` `meas_p`, `owon_oel` `info.p`) |

So the user's example (Tenma power) **already works**, but by copy: 12 files carry their own
V*I, two of them duplicating a fixed-point decode (`manson_hcs*`), and nothing tells the UI,
recordings or MCP that Tenma's POUT is computed while Rigol's is measured.
The scpi_psu base also computes (`devices/bases/scpi_psu.yaml:77`), and `unit_udp6700`
overrides it with a measured value.

### 1.3 Server

- Device thread: `bm_driver::actor::run` (`crates/bm-driver/src/actor.rs`) polls
  `Engine::poll_due` and emits `Event::Status(StatusUpdate {class, ch, status})`
  (no timestamp in `StatusUpdate`).
- Registry: `bm_server::devices::Registry::apply` (`crates/bm-server/src/devices.rs`) stores
  `status[class][ch]` and `status_at` (`Instant::now()` at apply). The event sink is the closure
  in `Devices::start` (`Box::new(move |ev| registry.lock().unwrap().apply(&rid, ev))`): **the
  single choke point every status passes**, ideal hook for a server-side processor.
  `Registry::to_json` feeds `/ws/registry` (`lib.rs::registry_stream`) and REST.
- `fresh_value()` (max age) is the read path for recordings and automation.
- Recordings v2: one whole-second sampler (`recordings.rs::sampler` / `Recordings::tick`) reads
  `fresh_value(..., MAX_AGE = 5 s)` per series; `validate_signal` only accepts fields that are
  **numeric status keys of the class contract** (`catalog.classes[cls].status[field]`). Anything
  in the registry status dict and in the contract is recordable for free.
- Automation vocabulary (`automation/vocab.rs`, build loop ~line 96) lists `fields` from the
  **binding's** status keys (+ contract types) and operations from the binding. MCP
  `bench_overview` / `read_instrument` (`mcp.rs`) use the same registry and vocabulary.
- `bm_driver::metrics` is link metrics (exchanges, latency), not signal statistics.
- `/instruments/{CLS}/{id}` (`lib.rs::features`) returns binding features merged with model
  features; `/methods` (`lib.rs::methods`, `opspec.rs::value_spec`) returns setter specs and
  `energizes`.

### 1.4 Automation executor (L3 already exists as a runtime)

`ai_context/AUTOMATION.md`, `crates/bm-server/src/automation/exec.rs`, `flow.rs`:
- Steps `set` (confirmed by read-back, `CONFIRM_TIMEOUT` 6 s), `read` (status field newer than
  the last set, `FRESH_TIMEOUT` 5 s), `wait`, `let`, `if`, `repeat` (while/until, bound,
  `MIN_PASS` 0.1 s), `sweep` (from/to/step or values), `check` (stop/warn), `log`, `prompt`,
  `record` (start/stop recordings), `stop`; CEL with `abs/min/max/round`, `elapsed`.
- Safety: `arm` for energising flows (`flow::check` -> `energize`), `cleanup` always runs,
  `safe_state()` (exec.rs ~717) switches off every energising op the run switched on after a
  failure/abort, abort interrupts waits. One run at a time.
- So a **software sweep, list, battery discharge, OCP trip test, I-V curve** can already be
  written as a flow today. What is missing is packaging (built-in parameterised procedures),
  XY results (I-V is a curve, recordings are time series), and sub-second timing.

### 1.5 Browser (derived values computed in the UI today)

- `frontend/src/ui/bench/panels/PsuPanel.tsx:19`:
  `pout = siNumber(status.POUT) ?? (vout*iout)` - a fallback the recording does not have.
- `DmmPanel.tsx:15-61`: running min/max/avg (`useStats`), HOLD, REL - all React state.
- `SalPanel` / `kit/SpectrumScreen.tsx`: client-side max-hold ghost trace.
- `bench/profiles.ts` derives limits/decimals from features + methods (fine: presentation).

Downsides of computing in the browser: not in the registry, so **not recordable**
(`Recordable field="POUT"` records the server value, which may differ from what is shown),
not visible to automation `read`, MCP or `/ws/registry` clients (Node-RED, scripts); each
window (main, detached panel window `bench/PanelWindow.tsx`, remote browser) computes its own
min/max with different start times; lost on reload; REL applied on screen only, so an export
does not match what the user saw. The browser is the right place only for pure presentation
(units, digits, history sparkline, ghost traces that nobody records).

## 2. Proposed layered architecture

One rule decides the layer: **who must agree on the value**. If the recording, MCP and every
window must see the same number, it lives on the server; if it needs memory across polls it
cannot live in a definition; if it sends commands on its own, it is a procedure with safety.

| Layer | What | Lives in | State | Sends commands |
|---|---|---|---|---|
| L0 | instrument native reading/setter | definition (`commands`, `status`, `operations`) | none | yes (bound) |
| L1 | stateless derivation from the status of one update | class contract `derive` + definition override | none | no |
| L1s | setter translation (period, Vrms, dBm, high/low) | class contract `virtual_ops`, server | none | yes, via native ops |
| L2 | stateful signal processing per channel (Ah, Wh, stats, REL) | `bm-server` new module `signals.rs` | per channel, in memory | no |
| L2g | software guards (limits that switch OFF) | `bm-server` `guards.rs` | per guard | only the safe direction |
| L3 | emulated procedures (list, sweep, battery, OCP, I-V) | built-in flows on `automation/` | run | yes, armed, confirmed |
| UI | presentation only | `frontend/src/ui` | per window | no |

### 2.1 L0 - instrument native

Unchanged. Native always wins: a definition that binds a key (or an op) suppresses the class
derivation for that key. This keeps measured values measured (Rigol DP800 POUT stays the
instrument's own reading).

### 2.2 L1 - class-level stateless derivation (the "every PSU gets POUT" layer)

**Contract change** (`devices/classes/<CLS>.yaml`, `bm_devicedef::model::ClassDef`): an
optional `derive` section, CEL over the **class status keys** of the same update (SI dicts
passed as their numeric `val`, enums/strings as is):

```yaml
# devices/classes/PSU.yaml
derive:
  POUT:  {expr: "VOUT * IOUT", as: si}
  RLOAD: {expr: "IOUT > 0.0005 ? VOUT / IOUT : null", as: si}   # optional new key
# ELL.yaml
derive:
  PIN:   {expr: "VIN * IIN", as: si}
  RIN:   {expr: "IIN > 0.0005 ? VIN / IIN : null", as: si}
# AWG.yaml
derive:
  PERIOD: {expr: "FREQ > 0.0 ? 1.0 / FREQ : null", as: si}
  HIGH:   {expr: "OFFS + AMPL / 2.0", as: si}
  LOW:    {expr: "OFFS - AMPL / 2.0", as: si}
  VRMS:   {expr: 'SHAPE == "SIN" ? AMPL / 2.8284271 : SHAPE == "SQU" ? AMPL / 2.0 : SHAPE == "RAMP" ? AMPL / 3.4641016 : null', as: si}
# SAL.yaml (example)
derive:
  SPAN_CHECK: ...  # only where useful; START/STOP from CENTER/SPAN when a model lacks one
```

Each derived key must also be declared in `status:` (type, description) so recordings
(`validate_signal`) and vocabulary accept it; `required: true` is satisfied by either a
binding or a class derivation (change in `validate.rs::check_binding`).

**Where it runs:** `bm_devicedef::load_catalog` (lib.rs) attaches the applicable class
derivations to each binding as a separate field (e.g. `ClassBinding.derived`, not merged into
`status`, because the variable scopes differ): only keys the binding does **not** bind, and only
if every key the expression references is bound. Decided at load time, so provenance is static
per definition, never flapping per poll. `CompiledStatus::new` compiles them;
`CompiledStatus::evaluate` runs a second pass over the first pass's output (numbers extracted
like `recordings.rs::numeric`). The device thread stays the only producer of status: registry,
`/ws/registry`, recordings, automation and MCP all see POUT with zero further changes.

**Consequences:**
- The 12 `vout * iout`-style POUT lines can be deleted (Tenma included) and the next PSU
  definition from a manual gets POUT for free. The Manson double decode goes away.
- `PsuPanel.tsx:19` fallback is removed: what is shown is what is recorded.
- Add the CEL helpers the derivations need (`abs`, `min`, `max`, `sqrt`, `log10`, `pow`) in
  `bm_driver::expr` once, shared with definitions (automation has its own; unify).
- Caveat: VOUT and IOUT are read in two exchanges (Tenma 30-45 ms each), so V*I has a small
  time skew; values from **different poll groups** may be up to one slow period apart. The
  validator should warn when a derivation mixes keys whose commands sit in different groups.

**API / provenance:** `/instruments/{CLS}/{id}` gains
`"provenance": {"POUT": "computed", "VOUT": "instrument", "WH": "server"}` (built in
`lib.rs::features` from the binding: bound = instrument, `derived` = computed, L2 = server);
the contract's `derive.expr` is reported too (`"POUT": {"source": "computed", "expr": "VOUT * IOUT"}`),
so users see how it is calculated. Status dicts stay unchanged (no per-sample overhead, no
breaking change for the UI's `siNumber`). Vocabulary `Field` and MCP `bench_overview` get a
`source` attribute; the assistant prompt can then say "POUT is computed".

**UI:** `profiles.ts` reads `provenance`; the kit `Readout` gets a small tag (e.g. `calc` in
the annunciator style, tooltip with the expression) for computed values, `SW` for L2.

**Tests:** `bm-devicedef` validator tests (derive references only class keys, types match,
required satisfied by derive, native suppresses derive, cross-group warning); `status.rs` unit
tests (second pass, null propagation, SI unwrapping); the existing manual-example harness
(`devices/examples/tenma_72.yaml` gains `POUT: 0.6` in `expect`), which runs through the real
engine; one server test that `/instruments/PSU/{id}` reports `provenance`; vitest that the
panel shows the `calc` tag and no longer multiplies. Simulated bench check:
`--simulate captures/20261016-203514` and read `/ws/registry`.

### 2.3 L1s - setter translation, defined once per class

**Contract change:** `virtual_ops` in the class contract, each mapping a user value to one or
more native ops of the same class, plus the read-back key (an L1 derived key):

| Class | Virtual op | Sends | Read-back | Precondition |
|---|---|---|---|---|
| AWG | `period` (s) | `frequency = 1/x` | `PERIOD` | x > 0 |
| AWG | `high_level`, `low_level` / `levels {high, low}` | `amplitude = H-L`, `offset = (H+L)/2` | `HIGH`, `LOW` | order the two sets so the intermediate state stays inside the limits (shrink first) |
| AWG | `amplitude_vrms` | `amplitude = Vrms * k(SHAPE)` | `VRMS` | SHAPE in SIN/SQU/RAMP, else 409 with a message |
| AWG | `amplitude_dbm` | `amplitude` from dBm and `LOAD` | derived `DBM` | numeric `LOAD` (50 ohm); refuse with HIGHZ |
| PSU | `power` (constant-power software setpoint) | not L1s: needs a loop, see L3 | | |

Native wins again: a definition that binds `period` (some generators accept it) is used as is.
**Where:** a small `bm-server` module (`virtual_ops.rs`) consulted by `lib.rs::op_set` /
`methods` and by the automation `set` path (`exec.rs`), dispatching to `DeviceClient::set` for
the native ops. Its value spec in `/methods` is computed from the native spec (`period` min =
1/frequency.max, max = 1/frequency.min; `format` from significant digits), and it carries
`"emulated": true`. `energizes` is inherited from the native op. Confirmation works unchanged
because the read-back key exists (L1). Tests: pure conversion and spec-transformation unit
tests; an engine-level test on a simulated AWG that `POST .../period/0.001` sends `FREQ 1000`.

### 2.4 L2 - stateful server-side signal processing

**New module** `crates/bm-server/src/signals.rs` (single responsibility: turn a stream of
status updates into stateful per-channel values). Hook: the `Devices::start` event closure
calls `signals.process(&id, &mut update)` before `Registry::apply`, so L2 keys land in the
same status dict and flow everywhere (WebSocket, recordings, automation `read`, MCP).

Needs `StatusUpdate.at: Instant` stamped by `Engine::poll_due` (time of the reads, not of the
apply) so integration uses real poll intervals.

Processors, declared per class in the contract (`accumulate:` section) so the panel and
vocabulary learn them from the API:

| Class | Keys | Rule |
|---|---|---|
| PSU | `AH`, `WH`, `T_ON` | trapezoid integration of IOUT, POUT over dt while OUTPUT = ON |
| ELL | `AH`, `WH`, `T_ON` | same with IIN, PIN while INPUT = ON (battery capacity) |
| DMM | `MIN`, `MAX`, `AVG`, `N`, `REL` / `MEAS_REL` | since reset, restarted on MODE/RANGE change (replaces `DmmPanel.useStats`) |
| LCR, OSC | `MIN/MAX/AVG` of PRIMARY or VPP/FREQ | optional, same generic statistic |
| SAL | max/min-hold trace | keep in browser unless recorded (lists are not recorded) |

Rules: never integrate across a gap (dt > MAX_AGE, null operand, disconnect) - the gap is
skipped and counted (`AH_GAPS` or a flag); values survive reconnects of the same device id but
not a server restart (in memory, like the registry); reset via server-side ops in the contract
(`energy_reset`, `stats_reset`, `rel {ON|OFF}`) that `op_set` handles without touching the
instrument, confirmed by the status key (AH back to 0). Accuracy is bounded by the poll rate
(Tenma 250 ms; a 1 s poll over a pulsed load is wrong) - the feature dict reports
`poll_ms` so the UI can say "integrated from 4 readings/s".

Provenance `server`; the recording of `WH` is the server's value, identical in every window.
**Tests:** pure unit tests of the integrator and statistics with synthetic timestamped
updates (constant current 1 A for 3600 s = 1 Ah, gap skipped, output off pauses, reset,
MODE change restarts stats); a `devices.rs`-style test with `simulated_connectors`; vitest
that DmmPanel shows server MIN/MAX. All suitable for CI (no hardware).

### 2.5 L2g - software guards (best-effort limits)

Separate from L2 because it **acts**. `crates/bm-server/src/guards.rs`: per channel, user-set
conditions over status keys (CEL, the same environment as L1) with a single permitted action:
set the energising op to its safe value (`vocab::Op::safe_value`, the same one
`exec.rs::safe_state` uses). Examples: software OPP for a PSU without OPP (`POUT > 20`),
battery cutoff for a load without a battery mode (`VIN < 3.0`), software OVP on a load,
"charge stop at 2 Ah". Configured through the API (stored in `<data dir>/guards.json`), shown
on the panel as an `SW LIMIT` annunciator, a trip is logged in the wire log
(`bm_driver::wirelog`) and as a registry flag (`SW_TRIP`). Guards run while no flow runs and
during flows (independent of the one-run limit). Never auto-re-energise.

### 2.6 L3 - emulated procedures on the automation executor

Ship **built-in parameterised flows** (read-only templates in the Automation drawer, copied on
edit) instead of new engine code; the executor already provides arm, confirmation, fresh
reads, abort, cleanup and `safe_state`:

| Procedure | Flow shape | Uses |
|---|---|---|
| Software list (PSU/ELL/AWG) | `sweep values: [...]` + `set` + `wait dwell` | L0 setters |
| Software frequency/amplitude sweep (AWG), lin or log | `sweep` over `let f = start * pow(r, i)` | needs `pow` in automation CEL |
| Battery discharge (ELL) | setup CC + `record` VIN/IIN/WH/AH; `repeat until VIN < cutoff`; cleanup input OFF | L2 `AH`, `WH` |
| Battery charge CC/CV (PSU) | set V/I, `repeat until IOUT < Iend or AH > max` | L2, L2g as backup |
| OCP trip test (PSU + ELL) | ELL current ramp by `sweep`, `check` PSU `OCP`/`CC` after each step, report trip current | L0 flags |
| I-V curve / LED / diode | `sweep` PSU voltage, `read` IOUT into a result table | needs XY output |
| Constant power from a PSU | `repeat`: `set voltage = Ptarget / IOUT` clamped | L1 POUT, slow loop |

Gaps to close (executor, small): a `table` result (rows of variables per sweep point, CSV
export, XY chart), `pow/sqrt/log10` in flow CEL, a "fast set" option for lists (skip waiting
for read-back up to a dwell, then verify once) because 6 s confirmation per step makes
sub-second lists impossible. Document the timing floor: one `set` + confirm is at least one
poll period (Tenma 250 ms) plus link time; the `MIN_PASS` 0.1 s per repeat pass. Hardware
lists (where an instrument has them) stay preferable for fast dwell times and should be bound
in L0 when they exist.

Provenance: a procedure run is a flow run (log, recording, `run_status` in MCP); built-ins are
labelled "emulated by BenchMesh" in the drawer, and the assistant can propose them
(`assist.rs` prompt gets the template list).
**Tests:** every template validated against a vocabulary fixture in CI (`flow::check`), and
executor tests on simulated instruments (abort mid-sweep switches the output off, cutoff
ends the discharge, cleanup runs).

### 2.7 UI layer (presentation only)

Keep in the browser: digits, units, sparkline history, the SAL ghost trace, layout. Remove
from the browser: `PsuPanel` V*I fallback, `DmmPanel` stats/REL (read L2 keys), any future
computation that a user could record. HOLD can stay local (it freezes the display, not data),
but should say so.

## 3. How each layer appears to consumers

| | Registry status | `/instruments/{CLS}/{id}` | `/methods` | Recordings | Automation / MCP |
|---|---|---|---|---|---|
| L0 | key | `provenance: instrument` | native op | yes | field / op |
| L1 | key (same dict) | `computed` + expr | - | yes (contract key) | field, `source: computed` |
| L1s | read-back key | - | op with `emulated: true`, transformed spec | - | settable op |
| L2 | key (same dict) | `server` + `poll_ms` | reset ops (server-side) | yes | field, `source: server` |
| L2g | `SW_TRIP` flag | guard list | guard API | flag not numeric | MCP read; control access to edit |
| L3 | - | - | - | via `record` step | flows, `run_flow` |

## 4. Safety rules for emulated features

1. **Software protection never replaces hardware OVP/OCP/OPP.** Guards and flows react after a
   poll (Tenma 250 ms, slow groups seconds) plus a command exchange plus, on USB serial,
   driver latency: a short circuit or a battery reversal is over long before. The UI states this
   next to every SW limit; where the instrument has hardware protection, the panel offers to
   set it (L0 `voltage_limit`/`current_limit`, `ovp`, `ocp`) before the software one.
2. Software actions only move to the **safe direction** (output/input OFF, setpoint down);
   nothing emulated ever energises without the existing `arm` (flows) or two-press UI rule.
3. A guard or integrator is **blind when the device is offline or stale**; staleness is shown
   (`fresh_value` semantics: no value rather than an old one), a guard whose input is stale
   for longer than its tolerance switches OFF (fail-safe) if the user chose so, else warns.
4. Emulated setters confirm through the derived read-back key; if the derivation cannot be
   evaluated (unknown SHAPE for Vrms, HIGHZ for dBm) the op refuses with a 409 and a reason
   instead of guessing.
5. Every computed or emulated value is labelled (`calc`, `SW`, "emulated by BenchMesh") in the
   panel, the API (`provenance`, `emulated`), MCP and exported recordings (series metadata
   `source`), so evidence copied from BenchMesh never passes off a calculation as a
   measurement.
6. Computed values inherit the uncertainty of their inputs (V*I with two readings taken
   ~40 ms apart, integration bounded by the poll rate); the feature dict reports the poll rate
   and the expression so users can judge.
7. The server owns everything stateful: closing the browser does not stop a guard, a server
   crash stops it (stated in the guard UI).

## 5. Suggested order of work

1. L1 class `derive` (contract + loader + `CompiledStatus` second pass + validator + provenance
   in features), delete the 12 duplicated POUT expressions and the PsuPanel fallback.
2. L2 `signals.rs` with PSU/ELL energy and DMM statistics (moves `DmmPanel.useStats` to the
   server), `StatusUpdate.at`.
3. L1s AWG virtual ops (period, high/low, Vrms).
4. L3 built-in templates (battery discharge, software list, OCP trip test) plus `table`
   results and `pow` in flow CEL.
5. L2g guards, after the UI wording for "software, best effort" is agreed.

Docs to update when implemented: `DEVICE_DSL.md` (CEL environment, class `derive`),
`CLASS_CONTRACT.md`, the class YAMLs, `AUTOMATION.md` (templates, table), `RECORDINGS.md`
(series `source`), CLAUDE.md "Instrument API" (provenance, emulated).
