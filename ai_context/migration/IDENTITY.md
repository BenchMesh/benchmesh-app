# Instrument identity (`match`) and Detect

Instruments are configured by **who they are**, not by the port they happen to be plugged into.
Works the same on Linux, Windows and macOS without root/admin rights and without udev rules
(`system/udev_rules.txt` is a development aid only).

## Two layers

1. **USB fingerprint** — which port to open. Read from the OS device tree (no rights needed):

   | field | meaning | Linux | Windows | macOS |
   |---|---|---|---|---|
   | `vid`, `pid` | USB vendor / product id (4 lowercase hex) | sysfs `idVendor`/`idProduct` | hardware id `USB\VID_xxxx&PID_xxxx` | IORegistry `idVendor`/`idProduct` |
   | `serial` | adapter/instrument USB serial number (absent on e.g. CH340) | sysfs `serial` | instance id `USB\VID_..&PID_..\<serial>` (ids containing `&` are synthesized by Windows = no serial) | `USB Serial Number` |
   | `revision` | `bcdDevice` (4 lowercase hex) — tells apart serial-less adapters of different chip revisions | sysfs `bcdDevice` | hardware id `REV_xxxx` | `bcdDevice` |
   | `location` | physical port position, OS-specific text, *soft*: only orders candidates | sysfs USB device name (`1-1.3.2.2.4`) | `CM_DRP_LOCATION_PATHS` first entry | `locationID` (`0x14320000`) |

   `vid`, `pid`, `serial` and `revision` must all match when present in the identity;
   `location` and the last known `port` only order the candidates.

2. **Instrument identity** — confirmed after opening: the definition's identify reply must match
   one of its models (as always) and, when the identity has `idn_serial`, contain that serial
   number as a whole token (`OWON,XDM1241,24512010,V4.3.0,3` → `24512010`). This is checked
   **before** the session starts (no `on_connect` traffic to the wrong instrument) and catches a
   USB-RS232 cable moved to another instrument.

## Config entry

```yaml
- id: owon-xdm1241
  name: OWON XDM1241
  driver: owon_xdm
  model: XDM1241
  transport: serial
  match:
    usb: {vid: "1a86", pid: "7523", revision: "0264", location: "1-1.3.2.2.4"}
    idn_serial: "24512010"
  port: /dev/ttyUSB2          # optional: last port seen (hint only)
```

An entry with `match` needs no `port`/`device`. An entry without `match` opens its `port`/`device`
path directly (manual setups, pseudo-terminals, tests).

Fingerprint built by Detect: `vid`, `pid`, `serial` if the device has one, otherwise `revision`;
`location` always; `idn_serial` when the identify reply carries one (SCPI `*IDN?` third field).

## Connecting (every connect and reconnect, ~2 s while offline)

1. List the USB ports of the definition's link kind (`serial` → USB serial ports, `usbtmc` →
   USBTMC instruments) — `bm_transport::discover::ports()`.
2. Keep those matching the fingerprint and not in use by another instrument of this process
   (in-process port lock: one open per path).
3. Order: same `location`, then same last `port`, then the rest.
4. Try each: open → identify → model match → `idn_serial` check → session (`on_connect`).
   First success wins. Nothing matching → offline with
   `no port matches usb 1a86:7523 rev 0264` (or the list of per-port rejections).

Unrelated ports (no fingerprint match) are never opened.

## Detect — `POST /devices/detect`

Lists USB serial ports and USBTMC instruments and identifies them:

- port held by a configured, connected instrument → reported as `configured_as` with the identity
  built from its last identify reply (nothing is sent);
- other ports → each candidate definition's identify command is sent with that definition's
  link settings (serial: all serial definitions in turn; USBTMC: definitions whose `link.usbtmc`
  `vid`/`pid` match, else all USBTMC definitions) until one model matches. Only the identify
  command; never session steps, never setters. Ports are probed in parallel.

Response:

```json
{"ports": [{
  "port": "/dev/ttyUSB2", "kind": "serial",
  "usb": {"vid": "1a86", "pid": "7523", "serial": null, "revision": "0264",
          "manufacturer": null, "product": "USB Serial", "location": "1-1.3.2.2.4"},
  "configured_as": "owon-xdm1241",
  "locked": false,
  "instrument": {"driver": "owon_xdm", "model": "XDM1241", "vendor": "OWON", "family": "XDM",
                 "idn": "OWON,XDM1241,24512010,V4.3.0,3", "classes": ["DMM"]},
  "match": {"usb": {"vid": "1a86", "pid": "7523", "revision": "0264", "location": "1-1.3.2.2.4"},
            "idn_serial": "24512010"},
  "certainty": "instrument",
  "error": null
}]}
```

- `locked`: the configured entry already identifies by `match` (else the UI offers to lock it).
- `instrument`/`match` are `null` when nothing answered (`error` says why).
- `certainty`: `instrument` (identify reply has a serial number) · `adapter` (USB serial number) ·
  `chip` (no serial, but the vid/pid/revision is unique among the detected ports) · `position`
  (only the USB port position separates it from an identical device — the UI warns).

## Code

| Piece | Where |
|---|---|
| port enumeration (`Port`, `UsbInfo`, `ports()`) | `crates/bm-transport/src/discover/` (`linux.rs`, `windows.rs`, `macos.rs`) |
| identity, matching, ordering, certainty, `idn_serial` | `crates/bm-transport/src/identity.rs` |
| in-process port lock | `crates/bm-transport/src/lock.rs` |
| connect attempts (`Attempt`, identify check before the session) | `crates/bm-driver/src/actor.rs`, `Engine::open` / `start_session` |
| Detect endpoint | `crates/bm-server/src/detect.rs` |
| UI | Instruments drawer → Detect (`frontend/src/ui/drawers/DetectView.tsx`, `detect.ts`) |
| what the OS reports | `cargo run -p bm-transport --example ports` |

## Status

Linux: verified on the bench (2026-10-17). Windows and macOS enumeration: compile- and
clippy-checked for both targets, built and run natively in CI, not yet run with instruments.

## USBTMC instruments per OS

| OS | How BenchMesh talks to them | Port path | One-time setup |
|---|---|---|---|
| Linux | kernel `usbtmc` driver | `/dev/usbtmcN` | access to `/dev/usbtmc*` (generic rule, installer) |
| Windows | own USBTMC protocol over `nusb` | `usb:1ab1:044d:DHO8A272907001` | the instrument needs the **WinUSB** driver (Zadig: select it → WinUSB → Replace Driver), unless … |
| Windows with a VISA (NI-VISA, Keysight IO Libraries, Rigol UltraSigma) | that VISA, loaded at run time | `USB0::0x1AB1::0x044D::DHO8A272907001::INSTR` | none — the VISA's driver stays |
| macOS | own USBTMC protocol over `nusb` | `usb:…` | none |

Discovery picks the right path per instrument (on Windows: WinUSB-bound → `usb:`, else the VISA
resource with the same vid/pid/serial). Identity matching is the same for all paths. On Windows
USBTMC instruments are listed from the device tree (compatible id `USB\Class_FE&SubClass_03`), so
an instrument **without any driver** is listed too (`nusb` cannot see those): `/usbtmc-devices`
gives it a `note` with the instructions above, opening it fails with them, and Detect shows them.
(First Windows test, 2026-10-18: neither the Rigol DHO800 nor the OWON DGE2070 was listed —
most likely because neither had a driver; not yet confirmed.)

The USBTMC protocol (`usbtmc_proto.rs`) discards replies whose bTag belongs to an earlier
request (a late answer after a timeout) and sends INITIATE_ABORT_BULK_IN after a timeout, so a
slow reply can no longer leave an instrument "one reply behind".
