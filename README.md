<div align="center">

![BenchMesh: power supply, multimeter, oscilloscope and electronic load on one bench](https://raw.githubusercontent.com/BenchMesh/benchmesh-app/develop/docs/static/bench.png)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/BenchMesh/benchmesh-app/develop/docs/static/logo/benchmesh-mark-on-dark.svg">
  <img alt="BenchMesh logo" src="https://raw.githubusercontent.com/BenchMesh/benchmesh-app/develop/docs/static/logo/benchmesh-mark-on-light.svg" width="72">
</picture>

# BenchMesh

**One cockpit for your whole test bench.**

Power supplies, multimeters, electronic loads, signal generators, oscilloscopes, LCR meters and
spectrum analyzers - connected over USB, serial or LAN, controlled from one window, recorded on
one shared timeline.

### [benchmesh.online](https://benchmesh.online)

[Download](https://benchmesh.online/download) · [Supported instruments](https://benchmesh.online/instruments) · [Docs](https://benchmesh.online/docs) · [Releases](https://github.com/BenchMesh/benchmesh-app/releases)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/BenchMesh/benchmesh-app/blob/develop/LICENSE)
[![Latest release](https://img.shields.io/github/v/release/BenchMesh/benchmesh-app)](https://github.com/BenchMesh/benchmesh-app/releases/latest)

</div>

## Why BenchMesh

- **One panel per instrument type** - every power supply, meter or scope looks and works the same,
  whatever the brand.
- **Recording on one time domain** - readings from all instruments line up second by second;
  export to CSV.
- **Automation** - build flows with conditions by hand, or describe what you want and let an AI
  assistant (your own Anthropic, OpenAI or OpenRouter key) assemble them.
- **AI assistants via MCP** - connect Claude, Cursor and other MCP clients to your bench.
- **Hundreds of models** described by device definitions, no drivers to write; the
  [instrument database](https://benchmesh.online/instruments) shows what is supported.
- **Free, open source, offline** - Windows, macOS and Linux.

## Get it

Download the installer for your system from **[benchmesh.online/download](https://benchmesh.online/download)**
(install notes for each platform, including opening the app on macOS).

## Build from source

The current code is on the [`develop`](https://github.com/BenchMesh/benchmesh-app/tree/develop) branch (Rust + Tauri, React UI).

```bash
cd frontend && npm ci && npm run build && cd -
cargo run -p bm-server        # API + UI on http://localhost:57666/ui
```

The desktop app: `cargo tauri dev` in `crates/benchmesh-app`; `cargo tauri build` there makes
installers for your system. Contributor notes are in
[CLAUDE.md](https://github.com/BenchMesh/benchmesh-app/blob/develop/CLAUDE.md) and [CONTRIBUTING.md](https://github.com/BenchMesh/benchmesh-app/blob/develop/CONTRIBUTING.md).

## Open source and the official build

Everything BenchMesh does is in this repository, under the MIT license. The installers on
benchmesh.online are the **official build**: this code plus a few things that only the BenchMesh
project ships - automatic updates (signed) and, later, the BenchMesh online services. A build you
make yourself is a **community build**: the same application, without them. Details:
[ai_context/OPEN_CORE.md](https://github.com/BenchMesh/benchmesh-app/blob/develop/ai_context/OPEN_CORE.md).

## License

[MIT](https://github.com/BenchMesh/benchmesh-app/blob/develop/LICENSE)
