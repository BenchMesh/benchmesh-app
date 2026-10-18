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
