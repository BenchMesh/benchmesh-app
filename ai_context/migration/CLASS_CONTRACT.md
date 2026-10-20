# Instrument Class Contract (as-is, extracted 2026-10-16)
> **Note (2026-10-17):** the React components analysed below (`ClassPods`, `GenericPSU`, …) were
> replaced by the bench UI (`frontend/src/ui/bench/`, one panel per class). The registry statuses
> and operations documented here are unchanged and are what the bench panels consume.

Input for migration Phase 2 (class schemas). Derived from the consumers (frontend,
Node-RED nodes, recording service) and producers (Python drivers, manifests).
There is no class-level schema in the Python code; this document is the first one.

**Legend:** ✅ consumed and produced · ⚠️ mismatch/bug · 💤 produced, never consumed

## 0. Shared contract

### Registry (`/ws/registry`, pushed every `BM_WS_INTERVAL`=0.2 s, full dump)
```
{ dev_id: { "IDN": str, "<CLASS>": { "status_ch<N>": { ...driver poll dict... }, "updated_ch<N>": ms } } }
```
- `updated_ch<N>` (2026-10-20): when that channel's status was last updated (ms since the epoch). The
  snapshot is re-sent every interval; the UI keeps a channel's status object until this changes, so
  per-reading statistics and histories count readings, not messages.
- N = 1..`features.channels`. Multi-class devices (`polling[].multi_class`) poll once with ch=1 and write each `result[CLASS]` to `status_ch1` only (`poll_worker.py:84-106`).
- `IDN` always written (may be `""`). UI "online" ⇔ IDN is a non-empty string (`InstrumentPod.tsx:20-23`, `StatusBar.tsx:30`).
- On link drop `IDN` and all `status_ch*` keys are removed (`registry.py:47-53`).

### SI value dict (`utils/si.py:240-293`)
```
{"si": {"number": str, "symbol"?: str, "prefix"?: str}, "sci": "%.5E", "val": str(raw)}
```
- `symbol`/`prefix` omitted when no prefix. `sci_to_value` on unparseable input → `{"si":{"number":"","symbol":"","prefix":""},"sci":raw}` (no `val`).
- UI reads `si.number` (parseFloat) + `si.symbol`; PSU setpoint loads read `.val`.

### `GET /instruments`
`[{id, name?, IDN?, health:{status, consecutive_failures, is_alive}, classes:[{class, channels:["/instruments/CLS/dev/1",…], ui_component}]}]` with ETag/304. UI splits channel path: `[2]=class [3]=dev [4]=ch`.

### Calls
- `GET /instruments/{C}/{dev}/{ch}/{name}` → `query_{name}` → `{"path","value"}`.
- `POST …/{name}/{param}` → `set_{name}` → **204 empty body**.
- Prefix always added → callers must send partial names.
- Param coercion: `true/false`→bool, int (not with leading 0), float, else string (`api.py:236-250`).
- `GET /instruments/{C}/{dev}` → raw `instrument_class.C.features`.
- `ui_component` dispatch: `ClassPods.tsx:17-28`.

## PSU - `GenericPSU` (tenma_72), `GenericOWONPSU` (owon_spm)
- **Status read:** `VOUT`,`IOUT`,`POUT` (SI); `OUTPUT`,`REMOTE` ("ON"/"OFF").
- 💤 `CV`,`CC`,`SBY`,`FAIL` (bool); SPM `OVP`,`OCP`,`OTP`,`OM` (raw); Tenma `ch1Mode`,`ch2Mode`,`Tracking`,`BeepEnabled`,`lockEnabled`,`outEnabled`.
- **Ops:** GET `voltage`,`current` (reads `.value.val`); OWON also `voltage_limit`,`current_limit`. POST `voltage/{n}`,`current/{n}`,`output/ON|OFF`,`remote/ON|OFF`; OWON also `voltage_limit/{n}`,`current_limit/{n}`. Tooltips: GET `output_voltage`,`output_current`,`output_power`.
- **Features read:** `lock`,`compulsory_lock`,`channels`,`channel_colors`. 💤 `absolute_limits`, `modes.*`, `memory_banks`, `memory_storage`, `protection`, `auto_lock` (Tenma uses `.unit`, SPM `.display`).
- ⚠️ `set_remote` missing on Tenma and SPM → 400. SPM hardcodes `REMOTE:"OFF"`.
- ⚠️ Tenma `POUT` = `_parse_numeric` on SI-dict repr → wrong when a prefix is present.
- ⚠️ Tenma `query_output_*` inverted cache → returns None on miss (confirmed in simulated capture: `could not convert string to float: 'None'`) and doubles poll traffic (VOUT/IOUT queried twice per poll).
- ⚠️ Tenma `set_ocp`/`set_ovp` use `is` → never send.
- ⚠️ `MeasurementStatusBar.tsx:61-96` looks for `output_voltage_num`/`voltage_num`/… - no driver produces them.
- ⚠️ `TimeSeriesGraph` in GenericPSU plots the setpoint input, not readings (`GenericPSU.tsx:325`).

## DMM - `GenericDMM` (owon_xdm, owon_spm[DMM])
- **Status read:** `MEAS` (SI). 💤 `MODE` (e.g. `VOLT_DC`), `RANGE` (string|null; SPM always null).
- **Ops:** POST `mode/{modeKey}` (manifest `modes` key), `range/{v}`, `set_temp_sensor/{v}`, `set_temp_scale/{sym}`. Node-RED: `query_measurement`.
- **Features read:** `modes{key:{symbol,unit,note,ui_name,ui_component}}`, `RANGe[modeKey]:[{value,display}]`, `RANGe.TEMP.Sensor`, `RANGe.TEMP.Scale`. 💤 `techname`,`hold`,`delta`,`rate`,`lock`,`DB:REFerence`,`DBM:REFerence`.
- ⚠️ No driver has `set_range` → 400 (drivers have `set_*_range`).
- ⚠️ GenericTemp posts `set_temp_sensor` → `set_set_temp_sensor` → 400.
- ⚠️ UI ignores registry `MODE` → dropdown doesn't track the instrument.
- ⚠️ XDM `query_measurement`/`query_function` inverted cache.

## ELL - `OwonOELELL` (owon_oel)
- **Status read:** `VIN`,`IIN`,`PIN` (SI); `REMOTE`,`INPUT` ("ON"/"OFF"); `MODE` ∈ `CURR|VOLT|RES|POW|DYN` (must match `features.modes` key). Legacy fallback numeric `VOUT`,`IOUT`,`POUT`. 💤 `OVP`,`OCP`,`OTP`.
- **Ops:** GET `curr|volt|res|pow` (parseFloat raw string). POST `mode/{KEY}`, `input/ON|OFF`, `remote/ON|OFF`, `{mode}/{n}`.
- **Features read:** `modes{KEY:{ui_name, setpoint:{min,max,symbol,unit}}}`, `lock`, `compulsory_lock`. 💤 `absolute_limits`, `setpoint.step`, `slew_presets`, `dynamic_list`, `remote_sense`, `parallel`.
- ⚠️ `REMOTE` hardcoded "ON". `INPUT` is the raw `INP?` reply (possibly "1"/"0" - verify by capture) while UI compares to "ON". `query_mode` returns None on unknown reply.
- ⚠️ Node-RED calls `set_current/voltage/power`; OEL has `set_curr/volt/pow`.

## AWG (contract `devices/classes/AWG.yaml`, panel `AwgChannel`)
- Required: `OUTPUT` and the `output` operation. Carrier keys SHAPE, FREQ (Hz), AMPL (Vpp), OFFS, PHASE, SYMM,
  LOAD (`"50"` | `HIGHZ`), pulse DUTY / WIDTH / RISE / FALL are optional. Period, Vrms, dBm and high/low levels
  are computed by the UI (`bench/awgMath.ts`), so a definition binds the native quantities only.
- **Modes (optional, AWG Core):** `MODE` CW | MOD | SWEEP | BURST. MOD: MOD_TYPE (AM, FM, PM, FSK, ASK, PSK, PWM),
  MOD_SOURCE, MOD_SHAPE, MOD_FREQ, AM_DEPTH (%), FM_DEV (Hz), PM_DEV (degrees). SWEEP: SWEEP_START / STOP / TIME,
  SWEEP_SPACING LIN | LOG, SWEEP_TRIG INT | EXT | MAN. BURST: BURST_MODE NCYC | INF | GATED, BURST_CYCLES (null when
  infinite), BURST_PERIOD, BURST_TRIG. Operations of the same names (`mode`, `mod_type`, `sweep_start`, ...,
  `trigger` ONCE). `mode` CW switches modulation, sweep and burst off.
- The panel shows a control only when its setter is in `/methods`; enum setters list only what the instrument
  takes (the DGE2000 has no trigger input, so its triggers are INT and MAN). A generator without `mode` is CW.
- Features: `channel_colors`. Only `owon_dge` binds the modes so far (bench-checked 2026-10-20,
  `scripts/awg_modes_check.py`); the other families follow (CLASS_FEATURES.md, step 3).

## OSC - `GenericOSC` (rigol_dho800)
- **Status read:** `SCALE`,`OFFSET`,`TIMEBASE` (SI), `COUPLING` (raw, unstripped). 💤 `CHANNEL` (int). 4 channels. No ops from UI.

## LCR / SAL (contracts 2026-10-18: `devices/classes/LCR.yaml`, `SAL.yaml`)
Written from the coverage reports (`coverage/LCR.md` §4, `coverage/SAL.md` §4); panels `LcrPanel`, `SalPanel`.

**LCR** - status PRIMARY, SECONDARY (si), FUNC (string), FREQ (si) required; LEVEL, CIRCUIT, SPEED, RANGE,
OVERLOAD, BIN, BIAS, BIAS_ON, REMOTE optional. **No setter is required** (read-only meters bind only the
status; the panel hides controls whose operation is missing from `/methods`). Conventions:
- `FUNC` / `function` values are canonical E4980 tokens (CPD, CSRS, LSQ, RX, ZTD, DCR, …). `features.functions`
  maps each to `{primary, secondary, circuit: SER|PAR (omitted when n/a), units: [primary, secondary], ui_name}`;
  the UI also knows the canonical tokens, so entries may omit fields. The `function` setter's first command
  lists every key (it is the `/methods` value spec). A/B meters (East Tester) bind `function` to several
  steps with `only: [...]`.
- `frequency` / `level`: an enum setter (keys = Hz / V as strings) → chips; a number setter → digit editor
  plus `features.frequency.presets` (or `.list`) chips.
- `speed` FAST|MED|SLOW (+ `features.speeds` displays), `range` AUTO|HOLD|<ohms> (+ `features.ranges`),
  `correction` OPEN|SHORT (AUTO where the meter decides), `trigger` ONCE, `bias_output` `safety: energize`.
- `features.counts`: display digits (or a count such as 20000).

**SAL** - required status CENTER, SPAN, START, STOP, RBW (si) and REF_LEVEL (number, dBm); required operations
start/stop/rbw/ref_level (get+set), center/span (set; the tinySA reports only start/stop, so its CENTER/SPAN status is computed in CEL) and sweep (set). The generator control appears only with `features.tg` (definitions bind `tg_output` family-wide). Frequencies/times are si; levels plain
numbers (dBm, dB). TRACE is a `list` of levels from START to STOP. Setter vocabularies: sweep
CONT|SINGLE|HOLD|RESTART, trace_mode WRITE|MAXHOLD|MINHOLD|AVERAGE|VIEW, detector POS|NEG|NORM|SAMP|RMS|AVER|QPE,
peak_search PEAK|NEXT|LEFT|RIGHT|MKR_CF, `full_span` without a parameter, `tg_output` `safety: energize`.
Features: freq_range, rbw/vbw lists (else 1-3-10 over the setter limits), ref_level, attenuation {min,max,step},
divisions, tg {min,max}, preamp.

Momentary actions (peak search, full span, restart, trigger, correction) use the kit `ActionButton`: it waits
until a reported value shows the effect (marker moved / at the peak, span changed, a new trace or reading).

## Cross-cutting mismatches
- Recording stores raw `query_*` return values; `LiveChart`/`RecordingDetails` parseFloat them → SI-dict values record as NaN/null.
- `RecordingControls.tsx:81,329` treats `/methods` as `string[]`; API returns `MethodInfo` objects.
- **All Node-RED nodes** send full names (`query_voltage`) → API resolves `query_query_voltage` → 400; also parseFloat on SI dicts → NaN; POST 204 empty body hits their error branch.

## Implications for the Rust class schemas
1. Keep the registry + SI dict wire shapes exactly (UI depends on them).
2. Define per class: required status keys (+types), optional keys, operations (partial name, value type/enum), features schema.
3. Fix (don't replicate) the ⚠️ items; each fix becomes a frontend or definition change tracked in the plan.
