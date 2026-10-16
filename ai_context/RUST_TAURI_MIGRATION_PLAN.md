# Rust + Tauri Migration Plan

**Status:** in progress — see [Progress tracker](#progress-tracker) and [Log](#log)
**Started:** 2026-10-16
**Owner:** MarkoV (with Claude Code)

> **Resuming after an interruption?** Read "Decisions", then the first unchecked
> item in the Progress tracker, then the last entries of the Log. Everything
> needed to continue is in this file and the paths it links.

---

## Goal

Replace the entire Python backend and the Electron shell with **Rust + Tauri**.
End state: the repo contains Rust crates + the existing React/TypeScript
frontend. **No Python anywhere** (runtime, drivers, tests, tooling).

The Python service is used only as a **reference oracle** during Phase 1: we
record what it actually puts on the wire to the real instruments, then delete it.

## Decisions (agreed)

| # | Decision | Why |
|---|---|---|
| D1 | Full Rust backend, no Python kept | Single binary, no runtime deps, deterministic concurrency |
| D2 | Keep the HTTP + WebSocket API (axum) as the contract; Tauri webview loads the UI from the embedded server | Remote-browser use, external clients, zero frontend rewrite |
| D3 | Ignore Node-RED for now; future automation via sequences-as-data + embedded script engine (Rhai) + optional MCP server | Node-RED needs a Node runtime; out of scope for the port |
| D4 | Instruments are described by a **declarative device definition DSL** (YAML validated by JSON Schema + small expression language), interpreted by one generic Rust driver | No Rust/Python needed to add a device; LLM-generatable from PDF manuals |
| D5 | Wire-level captures of the Python service against real hardware are the ground truth for device quirks | Quirks (no terminators, implicit delays, no-reply setters) are undocumented and live only in behaviour |
| D6 | Drop legacy polling mode, probabilistic throttling and quality multipliers; port a simple per-device actor + interval + backoff + circuit breaker | Simplicity; actor-per-device removes the race/leak bug class |
| D7 | Keep SQLite recordings schema compatible (`recording_series`, `data_points`) | Existing `~/.benchmesh/recordings.db` keeps working |

## Architecture target

```
crates/
  bm-transport   serial (serialport), usbtmc (kernel fd → later nusb), tcp
  bm-framing     tx terminator/min-gap, rx: terminator | fixed len | idle-gap | ieee488 block | none
  bm-devicedef   DSL schema (serde + JSON Schema), loader, validator
  bm-driver      generic interpreter: commands, parse pipeline, state, cache, class bindings
  bm-core        per-device actor, scheduler, registry, metrics
  bm-store       SQLite recordings
  bm-server      axum REST + WS + static /ui + OpenAPI (utoipa)
  benchmesh-server   headless binary (systemd / Docker / lab server)
  benchmesh-app      Tauri shell (embeds bm-server, opens window)
  bm-probe       CLI: verify definition vs device / replay captures / link discovery
devices/         *.yaml device definitions (one per device family)
captures/        wire captures (golden transcripts) used by replay tests
```

Three contract layers:
1. **Class schema** (PSU, DMM, ELL, AWG, OSC, ...) — status keys + operations the UI/API rely on. Fixed contract.
2. **Device definition** — maps one device's protocol onto a class schema.
3. **Interpreter** — generic Rust runtime.

## Known device quirks the DSL must express (from Python drivers)

| Quirk | Device | DSL primitive |
|---|---|---|
| No tx/rx terminators; reads end only on 1 s timeout | Tenma 72 | `rx.mode: idle` / fixed length |
| 1 s read timeout acts as an implicit inter-command delay | Tenma 72 | `tx.min_gap_ms` (measure!) |
| Binary 1-byte status bitfield | Tenma `STATUS?` | `reply: {bytes: N}` + `parse: {bits: ...}` |
| Setters produce no reply; reading after them hangs / corrupts USBTMC | DGE, Rigol, likely Tenma | per-command `reply: none` |
| Leading CR/LF noise skipped, first line only | serial transport | `rx.skip_leading`, `first_line` |
| DTR/RTS forced low + 50 ms settle on open | serial transport | `link.serial.dtr/rts/settle_ms` |
| Quoted / space-split / comma-positional / lowercase-enum replies | XDM, SPM, OEL | parse pipeline (unquote, split, field, map, number) |
| Range not reported by device → client-side shadow state | XDM, SPM | `state:` + `effects:` |
| Per-value cache TTL + invalidation on set | XDM, SPM, OEL | `cache:` + `invalidates:` |
| Derived values (POUT = V·I; CV/CC/SBY from bits) | Tenma, SPM | expressions in class binding |
| Remote lock required before control | OEL | `preconditions:` |
| IEEE-488.2 binary block, 2 s pre-read delay, chunking, retries, partial accept | DGE screenshot | built-in `reply: ieee488_block` with options |
| `*IDN?` used each poll as presence check | XDM | declared `presence:` check |

## Phases

### Phase 1 — Capture ground truth (Python untouched)
Tooling: `scripts/wire_capture/` (see its README). Runs the existing service with
runtime monkeypatching of pyserial + USBTMC `os.read/os.write` — **no repo source
is modified** — logging every byte with timestamps and the driver method that
caused it. Then exercises every read-only API method.

### Phase 2 — Specification
- Class schemas (`devices/classes/*.yaml`) derived from frontend/Node-RED consumers.
- DSL spec v0 (JSON Schema) + hand-written definitions for all 6 current devices.

### Phase 3 — Rust core
Transport + framing + interpreter, verified by replaying captures, then live via `bm-probe verify`.

### Phase 4 — Server + Tauri
axum API parity (contract tests), WS, recordings, OpenAPI, Tauri shell, installers, updater, CI.

### Phase 5 — Delete Python + Electron
Remove `benchmesh-serial-service/src`, pytest suite, Python tooling, `mcp_services/testing`, `electron/`.

### Future
- `bm-probe discover` (link/framing auto-detection)
- PDF manual → LLM → device definition (schema-constrained, page citations, read-only live probe, human safety gate)
- Automation: sequences-as-data, Rhai scripts, MCP server

## Progress tracker

- [x] Assessment of current codebase (backend, frontend, Electron, Node-RED)
- [x] Plan saved (this file)
- [x] Phase 1: capture tooling built (`scripts/wire_capture/`, see its README) and validated: `scripts/wire_capture/selftest.sh` → SELFTEST PASS
- [x] Phase 1: bench capture, read-only (session `captures/20261016-201609`, config `captures/bench-config.yaml`): Tenma 72-2540, SPM3103, XDM1241, OEL1515, DGE2070. Rigol DHO800 is **not on the bench**.
- [x] Phase 1: Tenma query timing measured (fixed-length framing, no inter-query gap needed)
- [x] Phase 1: OEL remote-mode hypothesis confirmed (measurement queries answer only after `SYST:REM`); OEL polled + exercised in session `captures/20261016-203514`
- [x] Phase 1: setter capture, all 5 devices (`captures/setters-bench.yaml` → session `captures/20261016-203514`); bench restored to pre-capture state (verified by read-back)
- [x] Phase 1: Tenma setter timing/format probed
- [~] Phase 1: DGE screenshot — failure mode captured (`captures/20261016-204256-owon-dge2070-query_screenshot`); **DGE is wedged (USBTMC bTag desync) and needs a physical replug**, then verify the single-large-read fix
- [x] Phase 2: as-is class contract extracted → `ai_context/migration/CLASS_CONTRACT.md`
- [x] Phase 2: per-device notes from PDF manuals + captures → `ai_context/migration/device-notes/*.md`
- [x] Phase 2: class schemas `devices/classes/{PSU,DMM,ELL,AWG,OSC}.yaml` (contract fixes applied, see Log)
- [x] Phase 2: DSL v1 — Rust types + validator + CLI (`crates/bm-devicedef`), JSON Schemas `devices/schema/`, spec `ai_context/migration/DEVICE_DSL.md`
- [x] Phase 2: device definitions `devices/{tenma_72,owon_spm,owon_xdm,owon_oel,owon_dge}.yaml` — valid, and cover all captured wire traffic (`cargo test -p bm-devicedef`)
- [~] Phase 2: rigol_dho800 definition — **skipped by user decision** (no scope on bench); revisit when one is available
- [x] Phase 3: cargo workspace scaffold (root `Cargo.toml`, `crates/*`), CI job `rust` in `.github/workflows/ci.yaml
- [x] Phase 3: `bm-driver` pure core — incremental framer (terminator/fixed/idle/message/IEEE block), parse pipeline, CEL eval + `num()`, exact port of Python SI dicts, wire rendering with validation (20 unit tests)
- [x] Phase 3: replay tests green against captures (`crates/bm-driver/tests/replay.rs`): 29 995 replies parse; 142 Python GET results and 78 distinct registry values reproduced
- [x] Phase 3: engine (`bm-driver::engine`) + actor (`bm-driver::actor`, one thread per device, reconnect/backoff, events) — 12 engine tests incl. replay sims of all 5 bench devices
- [x] Phase 3: transports (`crates/bm-transport`): serial (`serialport`, no libudev), Linux USBTMC (one read per message, ioctl timeouts); `bm-driver::sim` simulated transport
- [x] Phase 3: `bm-probe` CLI (`verify` read-only, `get`, `set`, `screenshot`) — **4/5 bench devices verified live**; `scripts/bench_setters.sh` 26/26 live setter checks pass, bench restored
- [x] Phase 3: DGE live verification after replug — all text queries/both channels OK (poll ≈100 ms)
- [ ] DGE screenshot — **not possible via Linux kernel usbtmc** (device streams raw bulk packets without valid USBTMC headers; see device-notes/owon_dge.md). Needs a userspace USBTMC transport (`nusb`) + udev rule (user sudo). Operation disabled in the definition. **DGE is wedged again by a raw probe → needs another replug.**
- [ ] Phase 3: live verification on bench
- [x] Phase 4: `bm-server` (axum) core API parity — instruments (+ETag), features, methods, GET/POST ops, drivers, config + device CRUD (atomic YAML), serial/usbtmc discovery, `/ws/registry`, static UI + SPA routes; 6 API tests; **existing React UI runs unmodified against the Rust server on the real bench (all 5 devices)**
- [ ] Phase 4: remaining endpoints — recordings (SQLite, same schema), `/metrics` + `/ws/metrics`, `/ai/context`, `/openapi.json` (utoipa)
- [x] Phase 4: UI follow-ups — API on the page origin (`src/api/base.ts`, Vite dev proxy), DMM mode follows registry MODE, `temp_sensor`/`temp_scale`, AWG channel panel (status + output toggle), tooltip URLs, Node-RED nodes (shared helper, partial op names, SI → number, API error details); frontend tests repaired (vitest ran Playwright specs; missing RequestLogProvider; stale assertions) → 35/35; Node-RED tests 4/4 in CI; server sends `Cache-Control: no-cache` for the UI
- [ ] Phase 4: userspace USBTMC (`nusb`) for DGE screenshots + Windows/macOS (needs udev rule)
- [ ] Phase 4: Tauri shell + packaging
- [ ] Phase 5: Python + Electron removed

## Next action (when resuming)

Phase 4 core is running. Start it with:
`./target/debug/benchmesh-server --config captures/bench-config.yaml` (port 57666, UI at /ui).
Next candidates: (a) recordings + metrics endpoints, (b) UI follow-ups incl. relative API base
(prerequisite for Tauri), (c) Tauri shell around bm-server, (d) nusb USBTMC.
Open hardware checks per device are listed at the end of each `device-notes/*.md` (read-only vs state-changing).
**DGE needs a physical replug** before any further DGE work.
To re-run a capture: stop the normal service, then
`python3 scripts/wire_capture/run_traced_service.py --config captures/bench-config.yaml` (terminal A),
`python3 scripts/wire_capture/exercise.py --passive 30 --rounds 3` (terminal B), Ctrl-C A, `summarize.py`, gzip the `.jsonl`.

## Open questions

1. ~~Which instruments are on the bench?~~ → Tenma, SPM3103, XDM1241, OEL1515, DGE2070 (no Rigol). User's `~/.benchmesh/config.yaml` lists only Tenma + DGE and points DGE at `/dev/usbtmc0` (actual: `/dev/usbtmc2`).
2. ~~Safe setter values?~~ → user: nothing is connected to any instrument, all setters safe.
3. ~~OK to send `SYST:REM` to the OEL?~~ → yes, done.
4. Keep the Rigol DHO800 definition without hardware (port from code only), or drop it until a scope is available?

## Bugs found in Python (do not replicate in Rust; reference only)

- `tenma_72/driver.py:142,149` — `value is "ON"` identity compare → OCP/OVP never toggle
- `owon_xdm/driver.py:110,124`, `tenma_72/driver.py:19,26` — inverted cache checks
- `serial_manager.py:539` — `poll_empty` drops driver without `close()` → fd leak
- `unified_scheduler.py:446` — dead-device drain discards HIGH-priority API requests (10 s hang)
- `recording_service.py` — bypasses device queue/lock; blocks event loop
- `owon_dge` / `rigol_dho800` `query_screenshot` — API passes channel as first arg (DGE: `open(1,'wb')` closes stdout)
- `eol_detector.py` — constructs drivers with wrong signature; always fails
- `GenericTemp.tsx` posts `set_temp_sensor` → resolves to `set_set_temp_sensor`

## Log

- **2026-10-16** — Assessment done; DSL approach and full-Rust end state agreed. Plan saved. No instruments were connected (`lsusb` showed none), so started with capture tooling + simulated validation, and class-contract extraction.
- **2026-10-16** — Built `scripts/wire_capture/` (tracer, runner, exerciser, summarizer, Tenma pty simulator, self-test). Self-test passes. Saved the as-is class contract (`ai_context/migration/CLASS_CONTRACT.md`).
  Findings from the simulated Tenma run (simulator, but they come from the driver code itself, not from the device):
  - Tenma framing really is timeout-based: every exchange takes ≈1001 ms (pyserial timeout=1.0), ending as `timeout`.
  - `poll_status` sends **5 queries per cycle** (VOUT, IOUT, VOUT, IOUT, STATUS) because of the inverted cache in `query_output_*` → ≈5 s per poll cycle against a 6 s interval.
  - Inverted cache bug confirmed at runtime: API `GET …/output_voltage` crashes with `could not convert string to float: 'None'`.
  - The Python transport aborts open when DTR/RTS ioctls fail (pty). Real adapters are fine, but Rust should treat modem-line control as best-effort and configurable per definition.
  - Hardware not connected at this time (`lsusb` showed no instruments) → real captures pending.
- **2026-10-16** — Read-only bench capture of 5 instruments (session `captures/20261016-201609`, raw logs gzipped). Per-device ground truth:

  | Device | Reply framing (actual) | Python behaviour | Real exchange | Python exchange |
  |---|---|---|---|---|
  | Tenma 72-2540 (9600, no EOL) | fixed length: VOUT/IOUT/VSET 5 B, **ISET 6 B with trailing space** (`'0.988 '`), STATUS 1 B, IDN 18 B | reads until 1 s timeout | 30–45 ms | ≈1005 ms |
  | SPM3103 (115200) | ends `\n` | waits for `\r` → always 1 s timeout | 4–13 ms | ≈1015–1025 ms |
  | XDM1241 (115200) | ends `\r\n` | ends on `\r`; the `\n` remains and is skipped as leading noise on the next read | 4–11 ms | same (works) |
  | OEL1515 (115200, FV V1.1.0) | IDN ends `\n`; **all measurement queries return a bare `\n`**, `MEAS:ALL?` → `ERR`, `SYST:REM?` → `0` | poll returns empty → link dropped → reconnect loop every 3–6 s; **OEL unusable in Python app** | — | — |
  | DGE2070 (USBTMC) | one message per read, ends `\n` | works | 13–56 ms | same |

  Other facts: Tenma STATUS idle = `0x10` (beep on, output off); 1 STATUS no-reply in 97. Tenma inter-query gap 0 ms → 0/30 failures (queries wait for the full fixed-length reply first). Tenma poll still sends 5 queries/cycle on real hardware (inverted cache). SPM `CONFigure?` → `VOLT:DC +0.0000E-308` (sentinel value — needs a definition rule). SPM `MEAS:ALL:INFO?` → 7 fields `V,I,P,OVP,OCP,OTP,mode`. XDM polled every 100 ms incl. `*IDN?` each poll (4 596 polls). Projected Rust gains from correct framing: Tenma poll ≈5 s → ≈0.1 s; SPM poll ≈2 s → ≈20 ms.
- **2026-10-16** — OEL + setter captures (session `captures/20261016-203514`):
  - **OEL needs remote mode:** in local mode every measurement/setting query returns a bare `\n`; after `SYST:REM` all answer in ~10–50 ms. `MEAS:ALL:INFO?` = 6 fields `V,I,P,OVP,OCP,OTP` (SPM has 7). `INP?` → `ON`/`OFF`. `FUNC?` → lowercase `current|voltage|resistance|power`.
  - **OEL setpoints are mode-gated:** `VOLT`/`RES`/`POW` are silently ignored unless `FUNC` is in that mode → the definition must switch mode first (or reject).
  - **No device replies to any setter** (Tenma, SPM, XDM, OEL, DGE). Python reads after every setter → +1 s each; on the XDM that read once consumed the next `*IDN?` reply (reply stream shifted by one). Rust: `reply: none` for setters.
  - **Tenma setters:** need ≥50 ms before the next command (20 ms → setter lost 10/10). Accept `5`, `5.0`, `05.10`; `12.345` and `1.5e1` are ignored → format setpoints `%.2f` (voltage) / `%.3f` (current). `OCP`/`OVP` never sent (Python `is` bug, confirmed on wire).
  - **XDM:** `RATE F`/`RATE M` have no effect (read-back stays `S`) → check command syntax in the XDM programming manual. TEMP measurement works (≈20 °C).
  - **SPM:** RES/CONT on open circuit → `measurement` None (overload reply needs a rule); `+0.0000E-308` sentinel seen again after range changes.
  - **DGE screenshot:** device sends a valid IEEE block `#6391734` + BMP (391 734 B). Python reads a 12-byte header then 64 KiB chunks → Linux usbtmc ends the transfer after the short read; rest of the image is stranded on the device; subsequent reads desync (`wrong bTag`, then `EOVERFLOW`/-75 on every read, even `*IDN?`). Device clear ioctl does not recover; USB reset needs root → **physical replug required**. Rust: read a USBTMC message with one large read (or proper USBTMC framing via nusb); never split reads of a message.
  - Reference PDFs (programming + user manuals for every driver) live outside the repo in `/home/marek/project/BenchMeshDOC/<driver>/doc/` — the test corpus for the future PDF → definition pipeline.
- **2026-10-16** — Phase 2 done (except Rigol). 5 parallel agents turned each programming-manual PDF + Python driver + captures into `device-notes/<driver>.md` (page-cited command tables, quirks, open hardware questions) — a manual dry run of the future PDF→definition pipeline. DSL v1 implemented in Rust (`bm-devicedef`: serde types are the source of truth, schemars JSON Schema, semantic validator incl. CEL compile + variable checks, type-aware wire regexes). Features added after the notes: `requires` (refuse with message; OEL mode-gated setpoints), `on_empty: null` (OEL local mode ≠ link loss), `settle_ms` / `gap_after_ms` (XDM), computed params `from:` (SPM/XDM ranges depend on active function), poll groups (DGE fast/slow), `timeout_ms`/`max_bytes` (DGE screenshot), `rx.drain_idle_ms`, model `verified`/`channels`. Tests: 8 green incl. negative validator cases and **capture coverage: every TX the Python service sent (≈30k commands, 3 sessions) is producible by the definitions**, with 6 deliberate deviations documented in `NOT_REPRODUCED`.
  Class-contract fixes vs Python/UI (UI follow-ups needed in Phase 4): DMM `range` op now exists (Python had only `set_*_range` → UI got 400); temperature ops are `temp_sensor`/`temp_scale` (UI must stop posting `set_temp_sensor`); PSU `remote` op optional (TENMA has no remote command; REMOTE status constant ON); ELL field 6 is `OPP` (manual) not `OTP`; ELL `REMOTE` comes from `SYST:REM?` instead of being hard-coded; AWG status extended (SHAPE, FREQ, AMPL, OFFS, PHASE, SYMM); XDM `hold` feature dropped (no SCPI command); SPM `RANGe AUTO` replaced by `:RANGe:AUTO ON`.
- **2026-10-16** — Known issue (pre-existing, not caused by this branch): Python suite has **32 failing tests on `p0-stabilize` and identically on `rust-migration`** (17× `test_api_recording.py` `no such table: recording_series`, 9× `test_unified_scheduler.py`, 6 others). GitHub CI's Python job is therefore red on both branches. Rust job is green locally (clippy `-D warnings`, 8 tests, validate). Decide: fix, or mark xfail until the Python backend is deleted in Phase 5.
- **2026-10-16** — User decisions: 32 pre-existing Python failures marked xfail (conftest hook `KNOWN_FAILURES`, delete in Phase 5) → Python suite 300 passed / 32 xfailed; Rigol skipped; class-contract API changes approved. Phase 3 started: `crates/bm-driver` (I/O-free interpreter core). Python SI formatting ported exactly (`pyfmt.rs`, `si.rs`; expected values generated by running the Python functions). Replay harness findings: (a) XDM `\r\n` replies were split across exchanges by Python's `\r` framing — replay treats rx as a continuous stream; (b) 39 XDM replies in the setter session are shifted one command late (Python desync) and are **rejected** by our parsers (desired); (c) Python returned raw text / garbage SI dicts where the contract now has numbers / SI dicts / null+OVERLOAD — compared semantically; (d) XDM RANGE state now starts null (unknown) like Python, not AUTO. SPM `function` GET now returns mode keys (VOLT_AC) like XDM.
- **2026-10-16** — Engine, transports and bench tool done. Live results with the Rust definitions (Python for comparison): TENMA poll ≈100 ms (≈5 s), GET ≈30 ms (≈1 s); SPM poll 20–35 ms (≈2 s); XDM poll ≈13 ms without per-poll `*IDN?`; **OEL works for the first time** (poll ≈70 ms; remote on connect, local again on disconnect, verified `SYST:REM?` → 0). Live setter suite `scripts/bench_setters.sh`: 26/26 incl. TENMA output on/off verified via VOUT, OEL mode-gated refusal. New hardware finding: **XDM slow-rate token is `S`; the manual's `L` is ignored**; CONF does not reset the rate (definition fixed, notes updated). Dependencies: `serialport` (MPL-2.0, file-level copyleft — fine for distribution, noted in THIRD-PARTY-NOTICES), `libc`. DGE: still EPROTO (wedged since the Python screenshot) — replug pending.
- **2026-10-16** — DGE replugged: `bm-probe verify` passes (all AWG queries, 2 channels). Screenshot root cause found: the DGE's USBTMC implementation streams large messages as raw bulk packets after the first 52 bytes; the kernel driver misparses them (`too small first packet`, `wrong MsgID`, EOVERFLOW). The Rust engine's repeated reads kept the device healthy but got corrupt data; a raw Python probe (mine) wedged it again → replug needed. Decision pending: userspace USBTMC via `nusb` (also needed for Windows/macOS). Screenshot op disabled in the definition meanwhile.
- **2026-10-16** — Phase 4 core: `crates/bm-server` (+ `benchmesh-server` binary). Wire shapes taken from a full spec of the Python API (agent report). Deliberate fixes vs Python: usbtmc `device` field survives `POST /config`; config entries validated (unknown driver → 422); driver errors mapped to 400/404/409/502/503/504 with the definition's message (e.g. OEL refusal text). Live: all 5 instruments healthy; React UI unmodified at http://localhost:57666/ui (WS receiving, 5/5 instruments, setpoints and readings live); clean Ctrl-C shutdown runs on_disconnect (OEL back to local, verified). Bench note: after a USB re-enumeration the SPM's and XDM's identical CH340 adapters (no serial number) share one by-id link — the identify/model check refused the wrong device; bench config now addresses both by USB port (by-path).
- **2026-10-16** — UI fixes done and verified live on the bench against the Rust server (DMM dropdowns show the instruments' real mode, AWG panel shows live channel status). Found and fixed on the way: frontend CI was red (vitest collected Playwright specs; component tests lacked RequestLogProvider; assertions on channel paths the UI no longer renders); browsers kept a stale index.html after rebuilds (no Cache-Control) → server now sends `no-cache` for UI files; the repo-wide `lib/` gitignore rule would have dropped the new Node-RED helper → directory named `nodes/shared/`.
