# Instrument coverage research

Goal: definitions (`devices/*.yaml`, `ai_context/migration/DEVICE_DSL.md`) for the instruments
electronics hobbyists and small labs actually use, written from their manuals **before** we have
the hardware. Each definition's models carry a verification status (below); the community
confirms them on real instruments.

| Status | Meaning |
|---|---|
| **Bench-tested** | Verified live on the maintainers' bench (`bm-probe verify` + setter checks) |
| **Community-confirmed** | A user ran the first-contact check on the real instrument and it passed |
| **Awaiting confirmation** | Written from the manual; protocol well documented or shared with a tested family, not yet seen live |

Files: one report per class (`<CLASS>.md`). Manuals are stored outside the repo (copyright) in
`/home/marek/project/BenchMeshDOC/<vendor>_<family>/doc/` with a `SOURCES.md` (URL, date, title);
reports cite them by that path and page.

## Report format (per class)

1. Ranking: families by hobby/small-lab prevalence, with price band.
2. Per family:
   - Models (+ per-model limits/ranges/channels for `features`), rebrands/clones sharing the protocol
   - Interfaces: USB-serial (chip, vid:pid if known), USBTMC (vid:pid), LAN (port, raw SCPI/VXI-11), RS-232 settings
   - Protocol: SCPI / proprietary ASCII / Modbus RTU / binary; identify command + example reply
   - Command table for the class operations (read readings, setpoints, output, mode, range…), manual page cited
   - Quirks: terminators, setters without reply, delays, remote/local, known firmware bugs (forums)
   - Documentation quality, local file paths, confidence (high/medium/low) and why
   - DSL fit: works with today's DSL / needs feature X (TCP, Modbus, binary framing, …)
3. Shared command baselines across families (input for `extends`).
4. For classes without a contract yet: proposed status keys, operations, features and the panel
   (displays and controls as on the instruments' front panels).

## Manual examples (testing without hardware)

Every definition whose models are all *Awaiting confirmation* needs `devices/examples/<id>.yaml`:
replies quoted from its manual (page cited), which `crates/bm-server/tests/manual_examples.rs`
feeds through the real engine — identify must select the model, one poll of every class must
fill every required status key with the contract's type, every readable operation must answer
and every setter must accept a valid value. Optional `expect` pins decoded values.

```yaml
definition: rigol_dp800
transport: usbtmc            # optional: which link (and its framing) to use
model: DP832                 # the model the identify reply must select
replies:                     # regex over the whole command (no tx terminator) -> reply
  - {send: '\*IDN\?', reply: "RIGOL TECHNOLOGIES,DP832,DP8A000000,00.01.16", source: "PG p.2-81"}
  - {send: ':MEAS:VOLT\? CH1', reply: "5.0000", source: "PG p.2-60"}
expect:
  PSU: {1: {VOUT: 5.0, OUTPUT: "ON"}}
```

Modbus instruments give a register map instead of replies (`modbus: {holding: {reg: value}, input: {…}, coils: {…}}`); a simulated slave answers reads and applies writes.

Replies are text (YAML escapes such as `"\x51"` for binary bytes); in `terminator` framing the rx
terminator is appended when missing. Commands that are not matched get no reply (setters).

## Notes for the website

`ai_context/website/instrument-notes.yaml` collects, per definition, what users should know
before buying or connecting: interfaces only some models have, interfaces BenchMesh cannot use
yet, ports / USB ids not taken from a manual (to confirm on first contact), which interface to
prefer, and issues reported by users. The online service shows them next to each model's status;
keep the file in step when definitions change (a test checks the ids).
