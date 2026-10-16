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
- [ ] Phase 1: OEL — confirm it needs remote mode (`SYST:REM`) before measurement queries answer → **needs user OK (state-changing)**, then re-capture OEL
- [ ] Phase 1: setter capture with user-approved safe values (`exercise.py --setters`) — also measure Tenma setter min-gap
- [ ] Phase 1: DGE screenshot binary-block capture (call `query_screenshot` directly, not via the GET API)
- [x] Phase 2: as-is class contract extracted → `ai_context/migration/CLASS_CONTRACT.md`
- [ ] Phase 2: class schemas written (`devices/classes/`) — decide fixes for ⚠️ items in CLASS_CONTRACT.md
- [ ] Phase 2: DSL JSON Schema v0
- [ ] Phase 2: device definitions for tenma_72, owon_xdm, owon_spm, owon_oel, owon_dge, rigol_dho800
- [ ] Phase 3: cargo workspace scaffold
- [ ] Phase 3: transports + framing engine
- [ ] Phase 3: interpreter + replay tests green against captures
- [ ] Phase 3: live verification on bench
- [ ] Phase 4: axum API parity + contract tests
- [ ] Phase 4: Tauri shell + packaging
- [ ] Phase 5: Python + Electron removed

## Next action (when resuming)

Read-only capture is done. Remaining Phase 1 items need user decisions (open questions 3–4).
Without them, Phase 2 can start: write the DSL JSON Schema v0 and the device definitions
against `captures/20261016-201609` (regenerate transcripts with
`python3 scripts/wire_capture/summarize.py captures/20261016-201609`).
To re-run a capture: stop the normal service, then
`python3 scripts/wire_capture/run_traced_service.py --config captures/bench-config.yaml` (terminal A),
`python3 scripts/wire_capture/exercise.py --passive 30 --rounds 3` (terminal B), Ctrl-C A, `summarize.py`, gzip the `.jsonl`.

## Open questions

1. ~~Which instruments are on the bench?~~ → Tenma, SPM3103, XDM1241, OEL1515, DGE2070 (no Rigol). User's `~/.benchmesh/config.yaml` lists only Tenma + DGE and points DGE at `/dev/usbtmc0` (actual: `/dev/usbtmc2`).
2. Which setter values are safe to exercise per device (e.g. PSU output into what load)?
3. OK to send `SYST:REM` / `SYST:LOC` to the OEL to confirm the remote-mode hypothesis?
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
