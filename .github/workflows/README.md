# GitHub Actions workflows

| Workflow | Runs on | What it does |
|---|---|---|
| `ci.yaml` | every push and pull request | Frontend: lint, unit tests (including the no-dash check of the whole repository), build. Rust: clippy, tests (including the device definitions against the wire captures), definition validation. Port discovery built and tested on Windows and macOS; there also the server is linted and its headless system code tested (start with the computer, restart, shut down). Desktop app crate: clippy and unit tests (with the WebKitGTK packages), on x64 and 64-bit ARM. |
| `e2e-tests.yml` | pushes and pull requests to `develop`, manual | Playwright end-to-end tests of the UI against a preview build. |
| `release-tauri.yml` | tags `v*.*.*`, manual (draft with a given version) | Builds the desktop app for Linux x64 and ARM64 (AppImage, deb; ARM64 on GitHub's native ARM runner), Windows (installer) and macOS (Apple silicon and Intel), signs the updater files and creates a **draft** release with the notes. |

## Releasing

1. Bump the version in `Cargo.toml` (workspace) and `crates/benchmesh-app/tauri.conf.json`, commit on `develop`.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`
3. When the build is done, check the draft and publish it: `gh release edit vX.Y.Z --draft=false`.
   Installed apps offer the update once it is published (signed `latest.json`).

The updater signing key is in the CI secrets `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`; without it
installed apps can no longer update.
