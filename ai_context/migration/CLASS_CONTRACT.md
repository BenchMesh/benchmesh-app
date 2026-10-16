# Instrument Class Contract (as-is, extracted 2026-10-16)

Input for migration Phase 2 (class schemas). Derived from the consumers (frontend,
Node-RED nodes, recording service) and producers (Python drivers, manifests).
There is no class-level schema in the Python code; this document is the first one.

**Legend:** ✅ consumed and produced · ⚠️ mismatch/bug · 💤 produced, never consumed

## 0. Shared contract

### Registry (`/ws/registry`, pushed every `BM_WS_INTERVAL`=0.2 s, full dump)
```
{ dev_id: { "IDN": str, "<CLASS>": { "status_ch<N>": { ...driver poll dict... } } } }
```
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

## PSU — `GenericPSU` (tenma_72), `GenericOWONPSU` (owon_spm)
- **Status read:** `VOUT`,`IOUT`,`POUT` (SI); `OUTPUT`,`REMOTE` ("ON"/"OFF").
- 💤 `CV`,`CC`,`SBY`,`FAIL` (bool); SPM `OVP`,`OCP`,`OTP`,`OM` (raw); Tenma `ch1Mode`,`ch2Mode`,`Tracking`,`BeepEnabled`,`lockEnabled`,`outEnabled`.
- **Ops:** GET `voltage`,`current` (reads `.value.val`); OWON also `voltage_limit`,`current_limit`. POST `voltage/{n}`,`current/{n}`,`output/ON|OFF`,`remote/ON|OFF`; OWON also `voltage_limit/{n}`,`current_limit/{n}`. Tooltips: GET `output_voltage`,`output_current`,`output_power`.
- **Features read:** `lock`,`compulsory_lock`,`channels`,`channel_colors`. 💤 `absolute_limits`, `modes.*`, `memory_banks`, `memory_storage`, `protection`, `auto_lock` (Tenma uses `.unit`, SPM `.display`).
- ⚠️ `set_remote` missing on Tenma and SPM → 400. SPM hardcodes `REMOTE:"OFF"`.
- ⚠️ Tenma `POUT` = `_parse_numeric` on SI-dict repr → wrong when a prefix is present.
- ⚠️ Tenma `query_output_*` inverted cache → returns None on miss (confirmed in simulated capture: `could not convert string to float: 'None'`) and doubles poll traffic (VOUT/IOUT queried twice per poll).
- ⚠️ Tenma `set_ocp`/`set_ovp` use `is` → never send.
- ⚠️ `MeasurementStatusBar.tsx:61-96` looks for `output_voltage_num`/`voltage_num`/… — no driver produces them.
- ⚠️ `TimeSeriesGraph` in GenericPSU plots the setpoint input, not readings (`GenericPSU.tsx:325`).

## DMM — `GenericDMM` (owon_xdm, owon_spm[DMM])
- **Status read:** `MEAS` (SI). 💤 `MODE` (e.g. `VOLT_DC`), `RANGE` (string|null; SPM always null).
- **Ops:** POST `mode/{modeKey}` (manifest `modes` key), `range/{v}`, `set_temp_sensor/{v}`, `set_temp_scale/{sym}`. Node-RED: `query_measurement`.
- **Features read:** `modes{key:{symbol,unit,note,ui_name,ui_component}}`, `RANGe[modeKey]:[{value,display}]`, `RANGe.TEMP.Sensor`, `RANGe.TEMP.Scale`. 💤 `techname`,`hold`,`delta`,`rate`,`lock`,`DB:REFerence`,`DBM:REFerence`.
- ⚠️ No driver has `set_range` → 400 (drivers have `set_*_range`).
- ⚠️ GenericTemp posts `set_temp_sensor` → `set_set_temp_sensor` → 400.
- ⚠️ UI ignores registry `MODE` → dropdown doesn't track the instrument.
- ⚠️ XDM `query_measurement`/`query_function` inverted cache.

## ELL — `OwonOELELL` (owon_oel)
- **Status read:** `VIN`,`IIN`,`PIN` (SI); `REMOTE`,`INPUT` ("ON"/"OFF"); `MODE` ∈ `CURR|VOLT|RES|POW|DYN` (must match `features.modes` key). Legacy fallback numeric `VOUT`,`IOUT`,`POUT`. 💤 `OVP`,`OCP`,`OTP`.
- **Ops:** GET `curr|volt|res|pow` (parseFloat raw string). POST `mode/{KEY}`, `input/ON|OFF`, `remote/ON|OFF`, `{mode}/{n}`.
- **Features read:** `modes{KEY:{ui_name, setpoint:{min,max,symbol,unit}}}`, `lock`, `compulsory_lock`. 💤 `absolute_limits`, `setpoint.step`, `slew_presets`, `dynamic_list`, `remote_sense`, `parallel`.
- ⚠️ `REMOTE` hardcoded "ON". `INPUT` is the raw `INP?` reply (possibly "1"/"0" — verify by capture) while UI compares to "ON". `query_mode` returns None on unknown reply.
- ⚠️ Node-RED calls `set_current/voltage/power`; OEL has `set_curr/volt/pow`.

## AWG — `GenericAWG` (owon_dge)
- Driver emits `{"OUTPUT":"ON"|"OFF"}` per channel (2). UI renders placeholders only; no ops used. Features: `channel_colors`.

## OSC — `GenericOSC` (rigol_dho800)
- **Status read:** `SCALE`,`OFFSET`,`TIMEBASE` (SI), `COUPLING` (raw, unstripped). 💤 `CHANNEL` (int). 4 channels. No ops from UI.

## LCR / SAL
- No drivers/manifests. UI components are static placeholders.

## Cross-cutting mismatches
- Recording stores raw `query_*` return values; `LiveChart`/`RecordingDetails` parseFloat them → SI-dict values record as NaN/null.
- `RecordingControls.tsx:81,329` treats `/methods` as `string[]`; API returns `MethodInfo` objects.
- **All Node-RED nodes** send full names (`query_voltage`) → API resolves `query_query_voltage` → 400; also parseFloat on SI dicts → NaN; POST 204 empty body hits their error branch.

## Implications for the Rust class schemas
1. Keep the registry + SI dict wire shapes exactly (UI depends on them).
2. Define per class: required status keys (+types), optional keys, operations (partial name, value type/enum), features schema.
3. Fix (don't replicate) the ⚠️ items; each fix becomes a frontend or definition change tracked in the plan.
