<div align="center">

![BenchMesh](https://raw.githubusercontent.com/MarkoVcode/BenchMesh/rust-migration/docs/static/BenchMesh.png)

# BenchMesh

**One cockpit for your whole test bench.**

Power supplies, multimeters, electronic loads, signal generators, oscilloscopes, LCR meters and
spectrum analyzers — connected over USB, serial or LAN, controlled from one window, recorded on
one shared timeline.

### [benchmesh.online](https://benchmesh.online)

[Download](https://benchmesh.online/download) · [Supported instruments](https://benchmesh.online/instruments) · [Docs](https://benchmesh.online/docs) · [Releases](https://github.com/MarkoVcode/BenchMesh/releases)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/MarkoVcode/BenchMesh/blob/rust-migration/LICENSE)
[![Latest release](https://img.shields.io/github/v/release/MarkoVcode/BenchMesh)](https://github.com/MarkoVcode/BenchMesh/releases/latest)

</div>

## Why BenchMesh

- **One panel per instrument type** — every power supply, meter or scope looks and works the same,
  whatever the brand.
- **Recording on one time domain** — readings from all instruments line up second by second;
  export to CSV.
- **Automation** — build flows with conditions by hand, or describe what you want and let an AI
  assistant (your own Anthropic, OpenAI or OpenRouter key) assemble them.
- **AI assistants via MCP** — connect Claude, Cursor and other MCP clients to your bench.
- **Hundreds of models** described by device definitions, no drivers to write; the
  [instrument database](https://benchmesh.online/instruments) shows what is supported.
- **Free, open source, offline** — Windows, macOS and Linux.

## Get it

Download the installer for your system from **[benchmesh.online/download](https://benchmesh.online/download)**
(install notes for each platform, including opening the app on macOS).

## Build from source

The current code is on the [`rust-migration`](https://github.com/MarkoVcode/BenchMesh/tree/rust-migration) branch (Rust + Tauri, React UI).

```bash
cd benchmesh-serial-service/frontend && npm ci && npm run build && cd -
cargo run -p bm-server        # API + UI on http://localhost:57666/ui
```

The desktop app: `cargo tauri dev` in `crates/benchmesh-app`. Contributor notes are in
[CLAUDE.md](https://github.com/MarkoVcode/BenchMesh/blob/rust-migration/CLAUDE.md) and [CONTRIBUTING.md](https://github.com/MarkoVcode/BenchMesh/blob/rust-migration/CONTRIBUTING.md).

## License

[MIT](https://github.com/MarkoVcode/BenchMesh/blob/rust-migration/LICENSE)
