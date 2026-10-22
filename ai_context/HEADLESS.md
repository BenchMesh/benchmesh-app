# Headless BenchMesh

BenchMesh without a window: the server alone, used from browsers on other computers through
remote access (`ai_context/REMOTE_ACCESS.md`). Typical use: a Raspberry Pi (or any small
computer) next to the bench with the instruments plugged into it, looked after entirely from a
laptop. Available on Linux (x64, 64-bit ARM), Windows and macOS.

## Starting it

The installed desktop app and the standalone server do the same:

```text
benchmesh --headless --remote-password '<at least 10 characters>'     # the desktop app's executable
benchmesh-server --remote-password '<at least 10 characters>'         # the server binary
```

| Option | Effect |
|---|---|
| `--remote-password P` | switches remote access on with this password; its hash is kept in `<data dir>/remote.json`, so later starts need no password. `BENCHMESH_REMOTE_PASSWORD` does the same without showing the password in process lists |
| `--port N` | loopback port (default 57666) |
| `--data-dir D` | user data (default `~/.benchmesh`, or `BENCHMESH_DATA_DIR`) |
| `--config F` | instruments (default `<data dir>/config.yaml`) |

At start it prints the remote addresses (`https://<address>:57667/`) and the certificate
fingerprint to compare with the browser's warning. Without a password it warns that no other
computer can reach it. The headless app listens on 127.0.0.1:57666 only: it cannot run beside the
app window (both would open the same instruments). Ctrl-C / SIGTERM stops it cleanly (the
definitions' `on_disconnect` steps run). On Windows the app attaches to the terminal it was
started from for its output.

Code: `crates/bm-server/src/headless.rs` (shared runner), `crates/benchmesh-app/src/headless.rs`
(the app's resources), `crates/bm-server/src/main.rs`.

## Instruments - System

The UI's System tab (`frontend/src/ui/system/`) shows for every BenchMesh: host, OS,
architecture, cores, uptime, processor temperature (where the OS reports one: Linux incl.
Raspberry Pi; often none on Windows and macOS), load averages (not on Windows), processor and
memory use, the disk of the data directory, and BenchMesh's own memory and processor use; updated
every 2 s. Meters turn amber from 75 %, red from 90 %; the temperature from 70 / 80 °C (a Pi
throttles at 80).

Headless only (in the desktop app the window owns the process: these answer 409):

- **Restart** (two presses): devices stop cleanly, then the same command line starts again - on
  Unix in place (`exec`, same process id, so systemd / launchd keep tracking it), on Windows as a
  new process. Remote sessions live in memory: browsers sign in again (the page reloads by itself).
- **Shut down** (two presses): devices stop cleanly, the process exits with code 0.
- Both are refused (409) while a flow runs: its cleanup switches off what it switched on.
- **Start with this computer**: an entry for the user BenchMesh runs as (no administrator rights)
  that runs the same executable with `--headless` and the options given at start (never the
  password):

| OS | Entry | Starts |
|---|---|---|
| Linux | systemd user service `~/.config/systemd/user/benchmesh.service` (`Restart=on-failure`, enabled with `systemctl --user enable`) | at login; at power-on once the user lingers: `sudo loginctl enable-linger "$USER"` (shown with Copy) |
| macOS | LaunchAgent `~/Library/LaunchAgents/online.benchmesh.headless.plist` (`KeepAlive` on unsuccessful exit) | at login (power-on with automatic login) |
| Windows | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `BenchMeshHeadless` | at sign-in (power-on with automatic sign-in) |

A crash is restarted by systemd / launchd; Shut down (exit 0) is not. An AppImage registers the
AppImage file itself (`$APPIMAGE`), not its temporary mount. A simulation never starts with the
computer. Without a systemd user manager (Linux) the tab says so.

## Updates

The desktop app offers new versions itself at start (Tauri updater, modal). Headless there is no
window for that, so Instruments - System - **Updates** does it (`crates/bm-server/src/system/update.rs`,
UI `frontend/src/ui/system/UpdateGroup.tsx`): **Check for updates** reads the same signed
`latest.json` of the latest *published* GitHub release; **Install <version> and restart** (two
presses, refused while a flow runs) downloads this installation's file (`latest.json` key
`<linux|windows|darwin>-<arch>-<appimage|nsis|app|deb>`) into `<data dir>/updates/`, verifies its
minisign signature with the updater's public key (the same as `tauri.conf.json`; a test keeps them
equal; an unsigned or altered download is deleted and never installed), then:

| Installed as | Update |
|---|---|
| Linux AppImage (`$APPIMAGE`) | BenchMesh stops, the AppImage file is replaced in place (old one kept as `<file>.previous`), the new one starts with the same arguments (Unix `exec`: same process id, systemd keeps tracking it) |
| Windows (installer) | BenchMesh stops; a detached helper waits, runs the signed installer silently (`/S`) and starts `benchmesh.exe` with the same arguments |
| macOS app | BenchMesh stops, the `.app` bundle is replaced (old one kept as `<bundle>.previous`) and started |
| Linux `.deb` (executable under `/usr`) | downloaded and verified only: the package manager owns the files; the tab shows `sudo apt install <data dir>/updates/BenchMesh_<v>_<arch>.deb` with Copy, then Restart |
| built from source (`benchmesh-server`) / simulation / desktop app | check only, with the reason |

When installing fails the old version starts again. The UI shows the download's progress and
reloads once the new BenchMesh answers (remote browsers sign in again). For a computer looked after
only remotely (a Raspberry Pi) the AppImage is the convenient choice: it updates itself.
`BENCHMESH_UPDATE_URL` replaces the manifest's address (staging tests); the key cannot be changed.

## API (scope `system`, `crates/bm-server/src/system/`)

| Route | |
|---|---|
| `GET /system` | `{host, os, arch, cpus, uptime_s, load: [1, 5, 15] \| null, cpu_percent, memory: {total, used}, process: {pid, memory, cpu_percent, started_ms}, disk: {path, total, available} \| null, temperature_c \| null, headless, version}` (bytes) |
| `POST /system/restart`, `POST /system/shutdown` | 202 `{action}`; 409 in the desktop app or while a flow runs |
| `GET /system/autostart` | `{supported, enabled, method, location, starts_at: boot\|login, boot_command, note, reason, command}` |
| `PUT /system/autostart` `{enabled}` | as GET; 400 without `enabled`, 409 in the desktop app or a simulation |
| `GET /system/update` | `{current, installed_as, can_install, replaces_itself, reason, checked_ms, latest: {version, notes, pub_date} \| null, available, phase: idle\|checking\|downloading\|installing\|manual\|failed, downloaded, total, error, command}` |
| `POST /system/update/check` | as GET, after reading the latest release |
| `POST /system/update/install` | 202 as GET (follow `GET`); 409 when this installation cannot install, without a newer version, while one is under way or while a flow runs |

Remote control sessions hold the `system` scope (headless, the browser is the only way to look
after the computer); view-only sessions read `GET /system` only. Restart, shut down and install
are not sent from the API reference drawer (`x-no-try`).

## Raspberry Pi

64-bit Raspberry Pi OS on a Pi 4 or 5: use `BenchMesh_<version>_aarch64.AppImage` (needs
`libfuse2`; it updates itself from the System tab) or install `BenchMesh_<version>_arm64.deb`
(`sudo apt install ./BenchMesh_*_arm64.deb`; updates need that command again), give the instruments
to the user (the udev rule / dialout group, https://benchmesh.online/docs/linux), run
`benchmesh --headless --remote-password '...'` once, then switch **Start with this computer** on
and run the linger command so it starts at power-on. The release builds ARM64 natively on
GitHub's ARM runners (`release-tauri.yml`); CI builds and tests the desktop app on ARM on every
push.
