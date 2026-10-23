# Open source and the official distribution

BenchMesh is open source (MIT): this repository, `BenchMesh/benchmesh-app`, is the whole
application, and anyone can build, run, change and redistribute it. The **official
distribution** - the installers on benchmesh.online and in this repository's Releases - is
built by the BenchMesh project from this repository plus a private repository,
`BenchMesh/services`, which adds what only an official build has, and holds the release process.

## What lives where

| | This repository (public) | `BenchMesh/services` (private) |
|---|---|---|
| Application | everything: server, drivers, device definitions, UI, desktop app, headless, automation, MCP, remote access | only extensions through the points below |
| App updates | - (a build from source is updated by building it again) | desktop updater (Tauri plugin) and headless updates (Instruments - System - Updates), the update addresses |
| Online services | the public, read-only community status from benchmesh.online (`community.rs`) | integrations that need the BenchMesh online service (accounts, later) |
| Build and release | local builds: `cargo build`, `cargo tauri build` (unsigned installers, no updater) | CI that builds, signs and publishes the official installers; the updater signing key; release notes |

Rules:

1. The open-source app is complete on its own. Nothing here is disabled or crippled to make the
   official build look better, and nothing here needs the private repository to build or to pass
   its tests.
2. No private code, keys, secrets, release workflows or release notes in this repository.
3. Extensions only use the extension points below. A new need means a new, small, documented
   extension point here first - it is public, so other distributions can use it too.
4. A build of this repository calls itself `community`; the official build `official`.
   `GET /version` `distribution`, and the About drawer shows "(community build)".

## Extension points

| Where | Contract | In this repository |
|---|---|---|
| Server | `bm_server::extension` (`Extension`: name, routes, their OpenAPI description; `Host`: data dir, headless; `Extensions` in `ServerOptions`); `System::hand_over` stops a headless process and runs the extension's step before it exits or restarts (`AfterStop`) | `benchmesh-server` passes none |
| Desktop app | crate `benchmesh-extensions`: `DISTRIBUTION`, `server(&Host)`, `desktop(builder, stop_server)` | the stub crate `crates/benchmesh-extensions`: adds nothing |
| UI | `frontend/src/extensions/contract.ts`: `root` (rendered once above the bench, e.g. a dialog), `systemGroups` (groups at the end of Instruments - System, with `viewOnly` and `onStopping`) | `frontend/src/extensions/index.ts`: empty lists |

Extension routes are served behind the same cross-site guard and remote gate as every other
route, and their remote scope follows their path (`remote/scope.rs`: an unclassified path is
local-only).

## How the official build is made

The services repository checks out this repository at a release tag into a separate directory,
copies its `overlay/` over it - replacing `crates/benchmesh-extensions/` and
`frontend/src/extensions/index.ts`, adding its UI files, a Tauri capability and a Tauri config
(`--config`) with the updater settings - then tests, builds and signs the installers for every
platform and publishes them as a draft release in this repository. Its CI also composes against
`develop` of this repository, so a change to a contract here shows up there at once.

Releasing: bump the version here (`Cargo.toml`, `crates/benchmesh-app/tauri.conf.json`) on
`develop`, tag `vX.Y.Z`, push the tag; the release itself is started in the services repository.

Official builds look for updates at `https://benchmesh.online/api/v1/update/latest.json` (the
website forwards to the latest published release here) and at this repository's
`releases/latest/download/latest.json`.

## Building your own

```bash
(cd frontend && npm ci && npm run build)
cd crates/benchmesh-app && cargo tauri build      # installers in target/release/bundle, unsigned
```

Your build is a community build: it never updates itself; build it again for a new version.
